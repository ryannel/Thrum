#!/usr/bin/env python3
"""Restore/reproduce one rejected pre-EDT CPU screen. Never starts browser/GPU/EDT."""
from pathlib import Path
import argparse,gzip,hashlib,json,shutil,subprocess,sys,zipfile
R=Path(__file__).resolve().parent

def digest(p):
 h=hashlib.sha256()
 with p.open('rb')as f:
  while b:=f.read(1024*1024):h.update(b)
 return h.hexdigest()

def verify_package():
 manifest=json.loads((R/'manifest.json').read_text())
 for n,v in manifest['files'].items():
  p=R/n
  if p.stat().st_size!=v['bytes'] or digest(p)!=v['sha256']:raise SystemExit('Package mismatch: '+n)
 deps=json.loads((R/'dependencies.json').read_text())['inputs']
 for d in deps:
  p=(R/d['container']).resolve()
  if d['kind']=='copy':expected=d['decoded_sha256'];size=d['decoded_bytes']
  else:expected=d['container_sha256'];size=d['container_bytes']
  if p.stat().st_size!=size or digest(p)!=expected:raise SystemExit('Dependency mismatch: '+d['container'])
 return deps

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--verify-only',action='store_true');ap.add_argument('--output',type=Path);ap.add_argument('--test',action='store_true');ap.add_argument('--run',action='store_true');a=ap.parse_args();deps=verify_package()
 if a.verify_only:
  if a.output or a.test or a.run:ap.error('--verify-only is exclusive')
  print('Verified package and all compressed dependency identities; no native restore/render');return
 if a.output is None:ap.error('--output required unless --verify-only')
 out=a.output.resolve()
 if out.exists():raise SystemExit('Choose a fresh absent output directory')
 if out.is_relative_to(R.parent):raise SystemExit('Output must be outside retained evidence')
 out.mkdir(parents=True);lab=out/'cloud-crown-preflight-v3';lab.mkdir()
 for p in (R/'frozen').iterdir():
  if p.is_file():shutil.copyfile(p,lab/p.name)
 for d in deps:
  target=(out/d['target']).resolve()
  if not target.is_relative_to(out):raise SystemExit('Unsafe target')
  target.parent.mkdir(parents=True,exist_ok=True);source=(R/d['container']).resolve()
  if d['kind']=='copy':shutil.copyfile(source,target)
  elif d['kind']=='gzip':
   with gzip.open(source,'rb')as src,target.open('wb')as dst:shutil.copyfileobj(src,dst,1024*1024)
  elif d['kind']=='zip':
   with zipfile.ZipFile(source)as z,z.open(d['member'])as src,target.open('wb')as dst:shutil.copyfileobj(src,dst,1024*1024)
  else:raise SystemExit('Unknown dependency kind')
  if target.stat().st_size!=d['decoded_bytes'] or digest(target)!=d['decoded_sha256']:raise SystemExit('Decoded mismatch: '+d['target'])
 if a.test:subprocess.run([sys.executable,str(lab/'test_preflight.py')],cwd=lab,check=True)
 if a.run:subprocess.run([sys.executable,str(lab/'preflight.py'),'--run'],cwd=lab,check=True)
 print('Restored exact preflight inputs to',out,'; render executed:',a.run)
if __name__=='__main__':main()
