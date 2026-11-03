"""Recover one exact rejected anatomy instrument; no numerical packages required."""
from pathlib import Path, PurePosixPath
import argparse, hashlib, importlib.util, json, shutil, subprocess, zipfile
ROOT=Path(__file__).resolve().parent

def digest(path):
 h=hashlib.sha256()
 with path.open('rb') as stream:
  for block in iter(lambda:stream.read(1024**2),b''):h.update(block)
 return h.hexdigest()

def restore(build,output,run_tests=False):
 record=json.loads((ROOT/'builds.json').read_text())[build]
 archive=ROOT/record['archive']
 if digest(archive)!=record['sha256']:raise ValueError('Archive SHA mismatch')
 output=output.resolve()
 if output.exists():raise ValueError('Choose a fresh recovery directory')
 if ROOT in output.parents:raise ValueError('Do not restore inside retained evidence')
 with zipfile.ZipFile(archive) as source:
  if set(source.namelist())!=set(record['files']):raise ValueError('Archive entry mismatch')
  for name in source.namelist():
   path=PurePosixPath(name)
   if path.is_absolute() or '..' in path.parts or '\\' in name:raise ValueError('Unsafe archive path')
  output.mkdir(parents=True)
  for name,expected in record['files'].items():
   target=output/name;target.parent.mkdir(parents=True,exist_ok=True)
   with source.open(name) as src,target.open('wb') as dst:shutil.copyfileobj(src,dst,1024**2)
   if target.stat().st_size!=expected['bytes'] or digest(target)!=expected['sha256']:raise ValueError('Recovered bytes differ: '+name)
 spec=importlib.util.spec_from_file_location('recovered_server',output/'serve.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
 payload,identity=module.payload()
 if identity['build']!=build:raise ValueError('Recovered browser identity differs')
 result={'build':build,'served_files':len(identity['files']),'payload_bytes':sum(map(len,payload.values())),'archive_bytes':archive.stat().st_size,'source_file_count':len(record['files'])}
 if run_tests:subprocess.run(['node','tests/anatomy.test.mjs'],cwd=output,check=True)
 return result

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--build',choices=['f30d926dcc6695fc','0ab02b3a38d03427'],required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--test',action='store_true');a=p.parse_args()
 print(json.dumps(restore(a.build,a.output,a.test),indent=2))
