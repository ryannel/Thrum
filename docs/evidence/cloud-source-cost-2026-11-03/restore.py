#!/usr/bin/env python3
"""Restore exact frozen CPU evidence and shared payload links into a fresh directory."""
import argparse,gzip,hashlib,json,os,pathlib,subprocess,sys,zipfile
P=pathlib.Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--output',type=pathlib.Path,required=True);a=ap.parse_args();out=a.output.resolve()
 if out.exists():raise SystemExit('Choose a fresh absent output directory')
 if out.is_relative_to(P):raise SystemExit('Output must be outside retained evidence')
 subprocess.run([sys.executable,str(P/'verify.py')],check=True);spec=json.loads((P/'restore-spec.json').read_text());out.mkdir(parents=True)
 with zipfile.ZipFile(P/spec['archive'])as z:
  for n in z.namelist():
   dest=(out/n).resolve()
   if not dest.is_relative_to(out)or pathlib.PurePosixPath(n).is_absolute():raise SystemExit('Unsafe archive member')
  z.extractall(out)
 for decoded,m in spec['external_by_decoded_sha256'].items():
  data=gzip.decompress((P/m['gzip_file']).read_bytes())
  if len(data)!=m['bytes']or sha(data)!=decoded:raise SystemExit('Decoded external payload mismatch')
  first=None
  for n in m['destinations']:
   dest=out/n;dest.parent.mkdir(parents=True,exist_ok=True)
   if first is None:dest.write_bytes(data);first=dest
   else:os.link(first,dest)
 subprocess.run([sys.executable,str(P/'verify.py'),'--restored',str(out)],check=True)
 print('Restored exact CPU proof layouts at',out,'No replay/benchmark/browser was run.')
if __name__=='__main__':main()
