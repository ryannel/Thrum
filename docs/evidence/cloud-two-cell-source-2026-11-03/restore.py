#!/usr/bin/env python3
"""Restore the exact CPU-only two-cell source contract; no browser/GPU or benchmark."""
from pathlib import Path
import argparse,gzip,hashlib,json,shutil,stat,subprocess,sys,zipfile
P=Path(__file__).resolve().parent

def sha(p):
 h=hashlib.sha256()
 with p.open('rb')as f:
  while b:=f.read(1024*1024):h.update(b)
 return h.hexdigest()

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--output',type=Path,required=True);ap.add_argument('--test',action='store_true');a=ap.parse_args();manifest=json.loads((P/'manifest.json').read_text())
 for n,v in manifest['files'].items():
  p=P/n
  if p.stat().st_size!=v['bytes']or sha(p)!=v['sha256']:raise SystemExit('Package mismatch: '+n)
 s=json.loads((P/'restore-spec.json').read_text());archive=P/s['archive']
 if sha(archive)!=s['archive_sha256']:raise SystemExit('Archive mismatch')
 out=a.output.resolve()
 if out.exists()or out.is_relative_to(P.parent):raise SystemExit('Choose a fresh absent output outside retained evidence')
 for n,v in s['external'].items():
  p=(P/v['container']).resolve()
  if p.stat().st_size!=v['container_bytes']or sha(p)!=v['container_sha256']:raise SystemExit('Shared container mismatch: '+n)
 out.mkdir(parents=True)
 with zipfile.ZipFile(archive)as z:
  if len(z.namelist())!=len(set(z.namelist()))or set(z.namelist())!=set(s['members']):raise SystemExit('Archive member set mismatch')
  for n in z.namelist():
   info=z.getinfo(n)
   if not(out/n).resolve().is_relative_to(out)or stat.S_ISLNK(info.external_attr>>16):raise SystemExit('Unsafe archive member')
   data=z.read(n);v=s['members'][n]
   if len(data)!=v['bytes']or hashlib.sha256(data).hexdigest()!=v['sha256']:raise SystemExit('Member mismatch: '+n)
  z.extractall(out)
 for n,v in s['external'].items():
  dest=(out/n).resolve()
  if not dest.is_relative_to(out):raise SystemExit('Unsafe dependency target')
  dest.parent.mkdir(parents=True,exist_ok=True);container=(P/v['container']).resolve()
  if v['kind']=='gzip':
   with gzip.open(container,'rb')as src,dest.open('wb')as dst:shutil.copyfileobj(src,dst,1024*1024)
  elif v['kind']=='zip':
   with zipfile.ZipFile(container)as z,z.open(v['member'])as src,dest.open('wb')as dst:shutil.copyfileobj(src,dst,1024*1024)
  else:raise SystemExit('Unknown dependency kind')
  if dest.stat().st_size!=v['decoded_bytes']or sha(dest)!=v['decoded_sha256']:raise SystemExit('Decoded shared input mismatch: '+n)
 pins=json.loads((out/'SOURCE-PINS.json').read_text())
 for n,h in pins['ownedCodeSHA256'].items():
  if sha(out/n)!=h:raise SystemExit('Frozen code mismatch: '+n)
 if a.test:subprocess.run([sys.executable,str(out/'run-guarded.py'),'--phase','all'],cwd=out,check=True)
 print('Verified CPU source revision',s['cpuContractRevision'],'at',out,'; tests run:',a.test)
 print('No two-cell renderer/browser/GPU/art/performance qualification.')
if __name__=='__main__':main()
