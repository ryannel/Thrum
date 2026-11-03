#!/usr/bin/env python3
"""Restore a frozen static cloud component proof; not a live lifecycle."""
import argparse,gzip,hashlib,json,pathlib,subprocess,zipfile
P=pathlib.Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--output',type=pathlib.Path,required=True);ap.add_argument('--variant',choices=['v1','v2'],default='v2');ap.add_argument('--test',action='store_true');a=ap.parse_args()
 manifest=json.loads((P/'manifest.json').read_text())
 for name,m in manifest['files'].items():
  b=(P/name).read_bytes()
  if len(b)!=m['bytes'] or sha(b)!=m['sha256']:raise SystemExit('Package mismatch: '+name)
 spec=json.loads((P/('build-v2.json' if a.variant=='v2' else 'build.json')).read_text());out=a.output.resolve()
 if out.exists():raise SystemExit('Choose a fresh absent output directory')
 if out.is_relative_to(P):raise SystemExit('Output must be outside retained evidence')
 out.mkdir(parents=True)
 with zipfile.ZipFile(P/spec['archive']) as z:
  for n in z.namelist():
   if not (out/n).resolve().is_relative_to(out):raise SystemExit('Unsafe archive member')
  z.extractall(out)
 for name,compressed in spec['external_gzip'].items():
  dest=out/name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(gzip.decompress((P/compressed).read_bytes()))
 for name,expected in spec['identity']['files'].items():
  if sha((out/name).read_bytes())!=expected:raise SystemExit('Served file mismatch: '+name)
 if a.test:
  tests=sorted(str(x) for x in (out/'tests').rglob('*.test.mjs'))
  if not tests:raise SystemExit('No tests found')
  subprocess.run(['node','--test',*tests],cwd=out,check=True)
 print('Verified',spec['identity']['build'],'Run: python3',out/'serve.py','--port 8155')
if __name__=='__main__':main()
