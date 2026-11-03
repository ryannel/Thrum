#!/usr/bin/env python3
"""Restore an exact isolated cohort proof; bulk inputs reuse retained evidence."""
import argparse,gzip,hashlib,json,pathlib,subprocess,zipfile,os
P=pathlib.Path(__file__).resolve().parent

def digest(b): return hashlib.sha256(b).hexdigest()

def main():
 ap=argparse.ArgumentParser(description=__doc__)
 ap.add_argument('--build',default='7c719e6a164480db')
 ap.add_argument('--output',type=pathlib.Path,required=True)
 ap.add_argument('--test',action='store_true')
 args=ap.parse_args(); spec=json.loads((P/'builds.json').read_text())[args.build]
 out=args.output.resolve()
 if out.exists(): raise SystemExit('Output must be a fresh absent directory')
 if out.is_relative_to(P): raise SystemExit('Output must be outside the retained package')
 archive=P/spec['archive']; raw=archive.read_bytes()
 if len(raw)!=spec['archive_bytes'] or digest(raw)!=spec['archive_sha256']: raise SystemExit('Archive mismatch')
 out.mkdir(parents=True)
 with zipfile.ZipFile(archive) as z:
  for name in z.namelist():
   dest=(out/name).resolve()
   if not dest.is_relative_to(out): raise SystemExit('Unsafe archive member')
  z.extractall(out)
 root=P.parents[2]
 for name,rel in spec['external_gzip'].items():
  blob=gzip.decompress((root/rel).read_bytes()); target=out/name; target.parent.mkdir(parents=True,exist_ok=True); target.write_bytes(blob)
 for name,expected in spec['identity']['files'].items():
  blob=(out/name).read_bytes()
  expected_sha=expected['sha256'] if isinstance(expected,dict) else expected
  if digest(blob)!=expected_sha or (isinstance(expected,dict) and len(blob)!=expected['bytes']): raise SystemExit('Served mismatch: '+name)
 identity=hashlib.sha256(json.dumps(spec['identity']['files'],sort_keys=True).encode()).hexdigest()[:16]
 if identity!=args.build: raise SystemExit('Aggregate build mismatch')
 if args.build=='7c719e6a164480db':
  old=json.loads((P/'builds.json').read_text())['eac4ae3bc6a5a681']; a=P/old['archive']
  if digest(a.read_bytes())!=old['archive_sha256']: raise SystemExit('Historical archive mismatch')
  h=out/'history/eac4ae3bc6a5a681'; h.mkdir(parents=True)
  with zipfile.ZipFile(a) as z:
   for name in z.namelist():
    if not (h/name).resolve().is_relative_to(h): raise SystemExit('Unsafe historical member')
   z.extractall(h)
  for name in old['external_gzip']: os.link(out/name,h/name)
  for name,expected in old['identity']['files'].items():
   if digest((h/name).read_bytes())!=expected: raise SystemExit('Historical served mismatch: '+name)
 if args.test:
  for p in sorted((out/'tests').glob('*.test.mjs')): subprocess.run(['node',str(p)],cwd=out,check=True)
 print(f'Verified {identity}, {len(spec["identity"]["files"])} exact served files in {out}')
 print(f'Run: python3 {out / "serve.py"} --port 8153')
 print('Offline source tests do not establish actual browser, visual or performance admission.')

if __name__=='__main__':main()
