#!/usr/bin/env python3
"""Restore an exact observed integrated Cu prototype, with shared frozen inputs."""
import argparse,gzip,hashlib,json,pathlib,subprocess,zipfile
P=pathlib.Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--output',type=pathlib.Path,required=True);ap.add_argument('--test',action='store_true');a=ap.parse_args()
 m=json.loads((P/'manifest.json').read_text())
 for n,v in m['files'].items():
  b=(P/n).read_bytes()
  if len(b)!=v['bytes'] or sha(b)!=v['sha256']:raise SystemExit('Package mismatch: '+n)
 s=json.loads((P/'build.json').read_text());out=a.output.resolve()
 if out.exists():raise SystemExit('Choose a fresh absent output directory')
 if out.is_relative_to(P):raise SystemExit('Output must be outside evidence')
 out.mkdir(parents=True)
 with zipfile.ZipFile(P/s['archive']) as z:
  for n in z.namelist():
   if not (out/n).resolve().is_relative_to(out):raise SystemExit('Unsafe archive member')
  z.extractall(out)
 for n,v in s['external'].items():
  b=gzip.decompress((P/v['gzip']).read_bytes())
  if sha(b)!=v['sha256']:raise SystemExit('Shared input mismatch: '+n)
  dest=out/n;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(b)
 for n,h in s['identity']['files'].items():
  if sha((out/n).read_bytes())!=h:raise SystemExit('Served file mismatch: '+n)
 if a.test:
  tests=sorted(str(p) for p in (out/'tests/render').glob('*.test.mjs'))
  subprocess.run(['node','--test',*tests],cwd=out,check=True)
  subprocess.run(['node',str(out/'tests/source/run.mjs')],cwd=out,check=True)
 print('Verified',s['identity']['build'],'Run: python3',out/'serve.py','--port 8157')
 print('Bounded source/consumer prototype; not final art, full sky or device-budget admission.')
if __name__=='__main__':main()
