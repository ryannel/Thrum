#!/usr/bin/env python3
"""Restore exact6bd CPU-screened assembled candidate; --test is functional only. No server/GPU/timing."""
from pathlib import Path
import argparse,gzip,hashlib,importlib.util,json,os,shutil,stat,subprocess,sys,zipfile
P=Path(__file__).resolve().parent

def sha(p):
 h=hashlib.sha256()
 with p.open('rb')as f:
  while b:=f.read(1024*1024):h.update(b)
 return h.hexdigest()

def unpack(entry,out):
 archive=P/entry['file']
 if archive.stat().st_size!=entry['bytes']or sha(archive)!=entry['sha256']:raise SystemExit('Archive mismatch')
 with zipfile.ZipFile(archive)as z:
  if len(z.namelist())!=len(set(z.namelist()))or set(z.namelist())!=set(entry['members']):raise SystemExit('ZIP member set mismatch')
  for n,v in entry['members'].items():
   if not(out/n).resolve().is_relative_to(out)or stat.S_ISLNK(z.getinfo(n).external_attr>>16):raise SystemExit('Unsafe ZIP member')
   b=z.read(n)
   if len(b)!=v['bytes']or hashlib.sha256(b).hexdigest()!=v['sha256']:raise SystemExit('ZIP member mismatch '+n)
  z.extractall(out)

def disk_bytes(*roots):
 seen=set();total=0
 for root in roots:
  for f in root.rglob('*'):
   if f.is_file():
    st=f.stat();key=(st.st_dev,st.st_ino)
    if key not in seen:seen.add(key);total+=st.st_size
 return total

def verify_source(out,s):
 for n,h in s['sourceFiles'].items():
  if sha(out/n)!=h:raise SystemExit('Served source mismatch '+n)
 if sha(out/'assets/policy/source-pins.json')!=s['sourcePinsSHA256']:raise SystemExit('CPU marker mismatch')
 sp=importlib.util.spec_from_file_location('restored_payload',out/'serve.py');m=importlib.util.module_from_spec(sp);sp.loader.exec_module(m);data,i=m.payload()
 if i['build']!=s['build']or i['files']!=s['sourceFiles']:raise SystemExit('Restored served identity mismatch')
 a=json.loads((out/'assets/assembly.json').read_text())
 for n,v in a['sourceApproval']['files'].items():
  if n not in data or len(data[n])!=v['bytes']or hashlib.sha256(data[n]).hexdigest()!=v['sha256']:raise SystemExit('Approved path missing/unhashed '+n)
 return {'build':i['build'],'servedFiles':len(data),'approvedSourceFiles':len(a['sourceApproval']['files']),'provenanceServed':True}

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--output',type=Path,required=True);ap.add_argument('--test',action='store_true');ap.add_argument('--with-observations',action='store_true');a=ap.parse_args();out=a.output.resolve()
 for n,v in json.loads((P/'manifest.json').read_text())['files'].items():
  p=P/n
  if p.stat().st_size!=v['bytes']or sha(p)!=v['sha256']:raise SystemExit('Package mismatch '+n)
 if out.exists()or out.is_relative_to(P.parent):raise SystemExit('Fresh absent output outside evidence required')
 s=json.loads((P/'restore-spec.json').read_text());out.mkdir(parents=True);unpack(s['sourceArchive'],out)
 for decoded,v in s['external_by_decoded_sha256'].items():
  container=(P/v['container']).resolve()
  if container.stat().st_size!=v['container_bytes']or sha(container)!=v['container_sha256']:raise SystemExit('Shared native container mismatch')
  targets=[(out/n).resolve()for n in v['destinations']]
  if any(not p.is_relative_to(out)for p in targets):raise SystemExit('Unsafe native target')
  first=targets[0];first.parent.mkdir(parents=True,exist_ok=True)
  if v['kind']=='gzip':
   with gzip.open(container,'rb')as src,first.open('wb')as dst:shutil.copyfileobj(src,dst,1024*1024)
  elif v['kind']=='zip':
   with zipfile.ZipFile(container)as z,z.open(v['member'])as src,first.open('wb')as dst:shutil.copyfileobj(src,dst,1024*1024)
  else:raise SystemExit('Unsupported native container')
  if first.stat().st_size!=v['decoded_bytes']or sha(first)!=decoded:raise SystemExit('Decoded native mismatch')
  for dest in targets[1:]:dest.parent.mkdir(parents=True,exist_ok=True);os.link(first,dest)
 if a.with_observations:
  unpack(s['observationsArchive'],out)
  for v in s['clips']:
   source=P/v['file'];dest=(out/v['destination']).resolve()
   if not dest.is_relative_to(out)or source.stat().st_size!=v['bytes']or sha(source)!=v['sha256']:raise SystemExit('Clip mismatch/unsafe target')
   dest.parent.mkdir(parents=True,exist_ok=True)
   try:os.link(source,dest)
   except OSError:shutil.copy2(source,dest)
 if a.with_observations and 'comparisonReference'in s:
  v=s['comparisonReference'];container=(P/v['container']).resolve();dest=(out/v['restoreDestination']).resolve()
  if not dest.is_relative_to(out)or container.stat().st_size!=v['containerBytes']or sha(container)!=v['containerSHA256']:raise SystemExit('Matched100 reference container mismatch')
  with zipfile.ZipFile(container)as z:b=z.read(v['member'])
  if len(b)!=v['bytes']or hashlib.sha256(b).hexdigest()!=v['sha256']:raise SystemExit('Matched100 reference PNG mismatch')
  dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(b)
 identity=verify_source(out,s);costBefore={n:sha(out/n)for n in s['costReceiptFiles']}
 if any(json.loads((out/n).read_text())['pass']is not True for n in costBefore):raise SystemExit('Both retained91CPUprotocols must remain passed')
 summary={'scope':'Functional restoration only; no cost/GPU/browser/server;First bounded91CPU screen PASS unchanged; GPU/art unqualified','identity':identity,'observationsRestored':a.with_observations,'costReceiptsSHA256':costBefore,'phases':[]}
 if a.test:
  env=dict(os.environ);env.pop('OBSERVED_NATURAL',None)
  subprocess.run([sys.executable,str(out/'run-guarded.py'),'--phase','all'],cwd=out,env=env,check=True)
  summary['phases']=json.loads((out/'guarded-all-results.json').read_text())['phases']
  subprocess.run([sys.executable,str(out/'run-bridge-guard.py'),'restored-bridge','node','--test','--test-concurrency=1','tests/bridge/loader.test.mjs','tests/bridge/lifecycle.test.mjs','tests/bridge/actual-reference.test.mjs','tests/bridge/control-sync.test.mjs','tests/bridge/matched100.test.mjs'],cwd=out,env=env,check=True)
  summary['bridge']=json.loads((out/'results/restored-bridge.guard.json').read_text())
  if {n:sha(out/n)for n in costBefore}!=costBefore:raise SystemExit('Cost receipts changed during functional test')
  verify_source(out,s)
 summary['newUniquePayloadDiskBytesPackagePlusRestore']=disk_bytes(P,out)
 if summary['newUniquePayloadDiskBytesPackagePlusRestore']>=80*1024**2:raise SystemExit('New80MiBdiskguard exceeded')
 summary['pass']=all(p['exitCode']==0 and p['adverse']is None for p in summary['phases'])and summary.get('bridge',{'pass':True})['pass']
 (out/'fresh-restore-results.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps({'pass':summary['pass'],'build':s['build'],'withObservations':a.with_observations,'functionalTests':a.test,'newUniquePayloadDiskBytes':summary['newUniquePayloadDiskBytesPackagePlusRestore']}))
if __name__=='__main__':main()
