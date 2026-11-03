#!/usr/bin/env python3
"""Restore exact, build-scoped instruments without changing retained evidence or live inputs."""
from pathlib import Path
import argparse,gzip,hashlib,json,shutil
ROOT=Path(__file__).resolve().parent
sha=lambda data:hashlib.sha256(data).hexdigest()
def checked(path,expected):
 data=path.read_bytes()
 if sha(data)!=expected:raise RuntimeError('Dependency SHA mismatch: '+str(path))
 return data

def restore(build,out,master):
 if out.resolve().is_relative_to(ROOT):raise RuntimeError('Output must be outside retained evidence')
 index=json.loads((ROOT/'builds.json').read_text());record=index['builds'][build];src=ROOT/record['source'];expected=json.loads((ROOT/record['identity']).read_text())
 if out.exists():raise RuntimeError('Refusing existing output directory: '+str(out))
 shutil.copytree(src,out)
 shutil.copytree(ROOT/'common/assets',out/'assets',dirs_exist_ok=True)
 for item in index['binary_inputs']:
  if item['kind']=='referenced-density':compressed=master/item['master_package_path']
  else:compressed=ROOT/item['compressed_path']
  data=gzip.decompress(checked(compressed,item['compressed_sha256']))
  if len(data)!=item['bytes'] or sha(data)!=item['sha256']:raise RuntimeError('Restored binary mismatch: '+item['file'])
  (out/item['file']).write_bytes(data)
 # Check the exact served file set, every file SHA, and canonical JSON build hash.
 files={}
 for p in sorted(out.rglob('*')):
  if not p.is_file():continue
  rel=p.relative_to(out);name=str(rel)
  if name in ('index.html','style.css') or (rel.parts[0]=='src' and p.suffix=='.js') or (rel.parts[0]=='shaders' and p.suffix=='.wgsl') or (rel.parts[0]=='assets' and p.suffix in ('.r16f','.json')):files[name]=sha(p.read_bytes())
 actual={'build':sha(json.dumps(files,sort_keys=True).encode())[:16],'files':files}
 if actual!=expected:raise RuntimeError('Unexpected restored payload or build: '+build)
 print('Exact restored identity:',build,'files:',len(files),'directory:',out)
 return out
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--build',choices=['all','81c3f43f76b490fa','173d261dc10b86c2','fd0181d8140a4ff9'],default='81c3f43f76b490fa');p.add_argument('--output',type=Path,required=True);p.add_argument('--master-package',type=Path,default=ROOT.parent/'cloud-authored-masters-2026-11-03');a=p.parse_args()
 if a.output.resolve().is_relative_to(ROOT):raise RuntimeError('Output must be outside retained evidence')
 if a.build=='all':
  if a.output.exists():raise RuntimeError('Refusing existing output root')
  a.output.mkdir(parents=True)
  for b in json.loads((ROOT/'builds.json').read_text())['builds']:restore(b,a.output/b,a.master_package)
 else:restore(a.build,a.output,a.master_package)
