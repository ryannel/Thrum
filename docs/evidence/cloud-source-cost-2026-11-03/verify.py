#!/usr/bin/env python3
"""Verify retained package or restored bytes; never execute a model/benchmark."""
import argparse,hashlib,json,pathlib,zipfile
P=pathlib.Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--restored',type=pathlib.Path);a=ap.parse_args()
 manifest=json.loads((P/'manifest.json').read_text())
 for n,m in manifest['files'].items():
  b=(P/n).read_bytes();assert len(b)==m['bytes']and sha(b)==m['sha256'],n
 assert sum(p.stat().st_size for p in P.rglob('*')if p.is_file())<=8*1024**2,'Durable8MiBcap'
 spec=json.loads((P/'restore-spec.json').read_text());archive=P/spec['archive'];assert sha(archive.read_bytes())==spec['archive_sha256']
 with zipfile.ZipFile(archive)as z:
  assert len(z.namelist())==len(set(z.namelist()))and set(z.namelist())==set(spec['files'])
  for n,m in spec['files'].items():
   b=z.read(n);assert len(b)==m['bytes']and sha(b)==m['sha256'],n
 for decoded,m in spec['external_by_decoded_sha256'].items():
  b=(P/m['gzip_file']).read_bytes();assert len(b)==m['gzip_bytes']and sha(b)==m['gzip_sha256'],m['gzip_file']
 if a.restored:
  out=a.restored.resolve()
  for n,m in spec['files'].items():
   b=(out/n).read_bytes();assert len(b)==m['bytes']and sha(b)==m['sha256'],n
  for decoded,m in spec['external_by_decoded_sha256'].items():
   for n in m['destinations']:
    b=(out/n).read_bytes();assert len(b)==m['bytes']and sha(b)==decoded,n
  identity=json.loads((P/'baseline-c3e0-identity.json').read_text());base=out/'cloud-system-proof'
  assert all(sha((base/n).read_bytes())==v for n,v in identity['files'].items())
  assert sha(json.dumps(identity['files'],sort_keys=True).encode())[:16]==identity['build']=='c3e0ebee9680337c'
 print(json.dumps({'pass':True,'archive':spec['archive'],'archiveMembers':len(spec['files']),'externalUniquePayloads':len(spec['external_by_decoded_sha256']),'restoredBytesVerified':bool(a.restored),'baselineBuild':spec['baselineBuild'],'scope':'Package/exact restored-byte verification only; no replay or timing'}))
if __name__=='__main__':main()
