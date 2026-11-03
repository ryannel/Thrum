import argparse,pathlib,json,hashlib,zipfile,gzip,subprocess,sys,os
P=pathlib.Path(__file__).resolve().parent
def main():
 a=argparse.ArgumentParser();a.add_argument('--output',type=pathlib.Path,required=True);args=a.parse_args();out=args.output.resolve()
 if out.exists()or out.is_relative_to(P):raise SystemExit('Choose a fresh output outside retained package')
 subprocess.run([sys.executable,str(P/'verify.py')],check=True);s=json.loads((P/'restore-spec.json').read_text());out.mkdir(parents=True)
 with zipfile.ZipFile(P/s['archive'])as z:
  for n in z.namelist():
   if not(out/n).resolve().is_relative_to(out):raise SystemExit('Unsafe archive member')
  z.extractall(out)
 for decoded,v in s['external_by_decoded_sha256'].items():
  raw=gzip.decompress((P/v['gzip_file']).read_bytes());assert hashlib.sha256(raw).hexdigest()==decoded;first=None
  for n in v['destinations']:
   d=out/n;d.parent.mkdir(parents=True,exist_ok=True)
   if first is None:d.write_bytes(raw);first=d
   else:os.link(first,d)
 subprocess.run([sys.executable,str(P/'verify.py'),'--restored',str(out)],check=True)
 print('Restored',out,'No replay or GPU run.')
if __name__=='__main__':main()
