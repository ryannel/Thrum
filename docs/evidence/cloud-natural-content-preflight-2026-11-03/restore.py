#!/usr/bin/env python3
"""Restore exact rejected preflight + original shared inputs. NEVER executes recipe/tests."""
from pathlib import Path
import argparse,gzip,hashlib,json,shutil,stat,zipfile
P=Path(__file__).resolve().parent
def sha(f):
 h=hashlib.sha256()
 with f.open('rb')as s:
  while b:=s.read(1048576):h.update(b)
 return h.hexdigest()
def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--output',required=True,type=Path);a=ap.parse_args();out=a.output.resolve()
 if out.exists()or out.is_relative_to(P.parent):raise SystemExit('Choose an absent output outside retained evidence')
 for n,v in json.loads((P/'manifest.json').read_text())['files'].items():
  f=P/n
  if f.stat().st_size!=v['bytes']or sha(f)!=v['sha256']:raise SystemExit('Package mismatch '+n)
 s=json.loads((P/'restore-spec.json').read_text());archive=P/s['archive']
 if sha(archive)!=s['archiveSHA256']:raise SystemExit('Archive mismatch')
 for v in s['external_by_decoded_sha256'].values():
  f=(P/v['container']).resolve()
  if f.stat().st_size!=v['container_bytes']or sha(f)!=v['container_sha256']:raise SystemExit('Shared input container mismatch')
 with zipfile.ZipFile(archive)as z:
  if len(z.namelist())!=len(set(z.namelist()))or set(z.namelist())!=set(s['members']):raise SystemExit('Member set mismatch')
  for n,v in s['members'].items():
   if not(out/n).resolve().is_relative_to(out)or stat.S_ISLNK(z.getinfo(n).external_attr>>16):raise SystemExit('Unsafe member')
   b=z.read(n)
   if len(b)!=v['bytes']or hashlib.sha256(b).hexdigest()!=v['sha256']:raise SystemExit('Member mismatch '+n)
  out.mkdir(parents=True);z.extractall(out)
 for h,v in s['external_by_decoded_sha256'].items():
  if v['kind']!='gzip':raise SystemExit('Unselected native container type')
  for n in v['destinations']:
   f=(out/n).resolve()
   if not f.is_relative_to(out):raise SystemExit('Unsafe native destination')
   f.parent.mkdir(parents=True,exist_ok=True)
   with gzip.open((P/v['container']).resolve(),'rb')as src,f.open('wb')as dst:shutil.copyfileobj(src,dst,1048576)
   if f.stat().st_size!=v['decoded_bytes']or sha(f)!=h:raise SystemExit('Decoded input mismatch')
   f.chmod(0o444)
 print('Exact rejected preflight and original inputs restored:',out)
 print('No recipe/projection/base/EDT/nativecook/tests/GPU/timing executed. Explicit guarded reproduction commands are in README.')
if __name__=='__main__':main()
