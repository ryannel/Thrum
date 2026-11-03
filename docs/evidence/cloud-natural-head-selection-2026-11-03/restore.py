from pathlib import Path
import argparse,json,zipfile,gzip,hashlib,subprocess,sys
P=Path(__file__).resolve().parent;sha=lambda b:hashlib.sha256(b).hexdigest();ap=argparse.ArgumentParser();ap.add_argument('--output',type=Path,required=True);ap.add_argument('--run',action='store_true');a=ap.parse_args();out=a.output.resolve()
for n,v in json.loads((P/'manifest.json').read_text())['files'].items():
 b=(P/n).read_bytes();assert len(b)==v['bytes'] and sha(b)==v['sha256'],n
s=json.loads((P/'restore-spec.json').read_text());assert sha((P/s['archive']).read_bytes())==s['archive_sha256']
if out.exists() or out.is_relative_to(P):raise SystemExit('Choose a fresh absent output outside evidence')
out.mkdir(parents=True)
with zipfile.ZipFile(P/s['archive']) as z:
 for n in z.namelist():
  assert (out/n).resolve().is_relative_to(out),n;b=z.read(n);v=s['members'][n];assert len(b)==v['bytes'] and sha(b)==v['sha256'],n
 z.extractall(out)
v=s['master'];b=(P/v['container']).read_bytes();assert sha(b)==v['container_sha256'];b=gzip.decompress(b);assert len(b)==v['decoded_bytes'] and sha(b)==v['decoded_sha256'];f=out/v['target'];f.parent.mkdir(exist_ok=True);f.write_bytes(b)
print('Verified measured natural-head diagnostic inputs and code; no mask/asset/runtime is restored')
if a.run:
 for n in ('cloud-natural-lobe-selection/raw_diagnostic.py','cloud-natural-lobe-selection/component_slices.py','cloud-natural-core-probe/core_probe.py'):subprocess.run([sys.executable,str(out/n)],check=True)
