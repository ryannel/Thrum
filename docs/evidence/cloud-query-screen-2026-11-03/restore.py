from pathlib import Path
import argparse,json,hashlib,zipfile,gzip
P=Path(__file__).resolve().parent;sha=lambda b:hashlib.sha256(b).hexdigest();ap=argparse.ArgumentParser();ap.add_argument('--build',choices=['e6456b622215cdd7','c2f6348636a059e0'],required=True);ap.add_argument('--output',type=Path,required=True);a=ap.parse_args();out=a.output.resolve()
for n,v in json.loads((P/'manifest.json').read_text())['files'].items():
 b=(P/n).read_bytes();assert len(b)==v['bytes'] and sha(b)==v['sha256'],n
s=json.loads((P/f'build-{a.build}.json').read_text());assert sha((P/s['archive']).read_bytes())==s['archive_sha256']
if out.exists() or out.is_relative_to(P):raise SystemExit('Choose a fresh absent directory outside evidence')
out.mkdir(parents=True)
with zipfile.ZipFile(P/s['archive']) as z:
 for n in z.namelist():assert (out/n).resolve().is_relative_to(out),n
 z.extractall(out)
for n,v in s['external'].items():
 b=gzip.decompress((P/v['gzip']).read_bytes());assert sha(b)==v['sha256'],n;f=out/n;f.parent.mkdir(parents=True,exist_ok=True);f.write_bytes(b)
for n,h in s['identity']['files'].items():assert sha((out/n).read_bytes())==h,n
print('Restored rejected query candidate',a.build,'Run python3',out/'serve.py','--port 8159')
