#!/usr/bin/env python3
"""Recover either failed CPU work-removal checkpoint; --test never runs timings."""
from pathlib import Path
import argparse,gzip,hashlib,json,os,shutil,stat,subprocess,sys,zipfile
P=Path(__file__).resolve().parent

def sha(p):
 h=hashlib.sha256()
 with p.open('rb')as f:
  while b:=f.read(1024*1024):h.update(b)
 return h.hexdigest()

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--variant',choices=['work','front'],required=True);ap.add_argument('--output',type=Path,required=True);ap.add_argument('--test',action='store_true');a=ap.parse_args();out=a.output.resolve()
 for n,v in json.loads((P/'manifest.json').read_text())['files'].items():
  p=P/n
  if p.stat().st_size!=v['bytes']or sha(p)!=v['sha256']:raise SystemExit('Package mismatch '+n)
 if out.exists()or out.is_relative_to(P.parent):raise SystemExit('Choose a fresh absent output outside retained evidence')
 spec=json.loads((P/'restore-spec.json').read_text());s=spec['variants'][a.variant];s['external_by_decoded_sha256']=spec['external_by_decoded_sha256'];archive=P/s['archive']
 if sha(archive)!=s['archiveSHA256']:raise SystemExit('Archive mismatch')
 for v in s['external_by_decoded_sha256'].values():
  p=(P/v['container']).resolve()
  if p.stat().st_size!=v['container_bytes']or sha(p)!=v['container_sha256']:raise SystemExit('Shared container mismatch')
 out.mkdir(parents=True)
 with zipfile.ZipFile(archive)as z:
  if len(z.namelist())!=len(set(z.namelist()))or set(z.namelist())!=set(s['members']):raise SystemExit('Member set mismatch')
  for n,v in s['members'].items():
   if not(out/n).resolve().is_relative_to(out)or stat.S_ISLNK(z.getinfo(n).external_attr>>16):raise SystemExit('Unsafe ZIP member')
   b=z.read(n)
   if len(b)!=v['bytes']or hashlib.sha256(b).hexdigest()!=v['sha256']:raise SystemExit('Member mismatch '+n)
  z.extractall(out)
 for decoded,v in s['external_by_decoded_sha256'].items():
  targets=[(out/n).resolve()for n in v['destinations']]
  if any(not p.is_relative_to(out)for p in targets):raise SystemExit('Unsafe native target')
  first=targets[0];first.parent.mkdir(parents=True,exist_ok=True);container=(P/v['container']).resolve()
  if v['kind']=='gzip':
   with gzip.open(container,'rb')as src,first.open('wb')as dst:shutil.copyfileobj(src,dst,1024*1024)
  elif v['kind']=='zip':
   with zipfile.ZipFile(container)as z,z.open(v['member'])as src,first.open('wb')as dst:shutil.copyfileobj(src,dst,1024*1024)
  else:raise SystemExit('Unsupported dependency')
  if first.stat().st_size!=v['decoded_bytes']or sha(first)!=decoded:raise SystemExit('Decoded mismatch')
  for dest in targets[1:]:dest.parent.mkdir(parents=True,exist_ok=True);os.link(first,dest)
 if sha(out/'SOURCE-PINS.json')!=s['sourcePinsSHA256']:raise SystemExit('Source marker mismatch')
 for n,v in json.loads((out/'SOURCE-PINS.json').read_text())['source'].items():
  if sha(out/n)!=v['sha256']:raise SystemExit('Source mismatch '+n)
 costBefore={n:sha(out/'results'/n)for n in s['costReceiptsSHA256']};runs=[]
 if costBefore!=s['costReceiptsSHA256']:raise SystemExit('Archived cost mismatch')
 if a.test:
  env=dict(os.environ);env.pop('OBSERVED_NATURAL',None)
  for command in s['functionalCommands']:
   if any('cost' in Path(word).name and word.endswith('.mjs')for word in command):raise SystemExit('Timing execution prohibited by restore')
   subprocess.run([sys.executable,str(out/'run-guard.py'),*command],cwd=out,env=env,check=True)
   r=json.loads((out/'results'/(command[0]+'.guard.json')).read_text());runs.append({'phase':command[0],**r})
  if {n:sha(out/'results'/n)for n in costBefore}!=costBefore:raise SystemExit('Cost receipt changed during functional-only run')
  for n,v in json.loads((out/'SOURCE-PINS.json').read_text())['source'].items():
   if sha(out/n)!=v['sha256']:raise SystemExit('Source changed during functional run '+n)
  (out/'fresh-functional-results.json').write_text(json.dumps({'pass':all(r['pass']for r in runs),'runs':runs,'scope':'Fresh functional relocation only; cost evaluator NEVER executed; recorded cost failure unchanged','variant':a.variant,'sourcePinsSHA256':s['sourcePinsSHA256'],'costReceiptsSHA256':costBefore},indent=2)+'\n')
 print('Verified frozen source',s['sourcePinsSHA256'],'at',out,'; functional tests:',a.test)
 print('Recorded4ms cost gate FAIL remains; no cost/GPU/browser/adoption qualification.')
if __name__=='__main__':main()
