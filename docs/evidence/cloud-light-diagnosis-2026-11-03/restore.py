#!/usr/bin/env python3
"""Restore an exact rejected Cb anatomy/light comparison, with durable scalar."""
import argparse,gzip,hashlib,json,pathlib,subprocess,zipfile
P=pathlib.Path(__file__).resolve().parent

def sha(b):return hashlib.sha256(b).hexdigest()

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--output',type=pathlib.Path,required=True);ap.add_argument('--variant',choices=['daab','0c3','f06'],default='daab');args=ap.parse_args()
 manifest=json.loads((P/'manifest.json').read_text())
 for name,m in manifest['files'].items():
  b=(P/name).read_bytes()
  if len(b)!=m['bytes'] or sha(b)!=m['sha256']:raise SystemExit('Package mismatch: '+name)
 out=args.output.resolve()
 if out.exists():raise SystemExit('Choose a fresh absent output directory')
 if out.is_relative_to(P):raise SystemExit('Output must be outside the retained package')
 out.mkdir(parents=True);bundle=out/'source-package';bundle.mkdir()
 with zipfile.ZipFile(P/'source-and-research.zip') as z:
  for name in z.namelist():
   if not (bundle/name).resolve().is_relative_to(bundle):raise SystemExit('Unsafe archive member')
  z.extractall(bundle)
 scalar=out/'cb-covered-256.density.r16f';scalar.write_bytes(gzip.decompress((P/'cb-covered-256.density.r16f.gz').read_bytes()))
 if sha(scalar.read_bytes())!=manifest['scalar_sha256']:raise SystemExit('Decoded scalar mismatch')
 subprocess.run(['python3','-B',str(bundle/'restore.py'),'--scalar',str(scalar),'--variant',args.variant,'--target',str(out/'viewer')],check=True)
 print('Run: python3 '+str(out/'viewer/serve.py')+' --port 8154')
 print('Rejected source/light comparison only; this is not an accepted Cb or production budget.')

if __name__=='__main__':main()
