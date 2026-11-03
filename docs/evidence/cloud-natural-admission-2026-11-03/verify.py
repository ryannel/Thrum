import argparse,pathlib,json,hashlib,zipfile,gzip
P=pathlib.Path(__file__).resolve().parent;sha=lambda b:hashlib.sha256(b).hexdigest()
def main():
 a=argparse.ArgumentParser();a.add_argument('--restored',type=pathlib.Path);args=a.parse_args();m=json.loads((P/'manifest.json').read_text())
 for n,v in m['files'].items():b=(P/n).read_bytes();assert len(b)==v['bytes']and sha(b)==v['sha256'],n
 s=json.loads((P/'restore-spec.json').read_text());assert sha((P/s['archive']).read_bytes())==s['archive_sha256']
 with zipfile.ZipFile(P/s['archive'])as z:
  assert len(z.namelist())==len(set(z.namelist()))and set(z.namelist())==set(s['files'])
  for n,v in s['files'].items():b=z.read(n);assert len(b)==v['bytes']and sha(b)==v['sha256'],n
 for decoded,v in s['external_by_decoded_sha256'].items():
  b=(P/v['gzip_file']).read_bytes();assert len(b)==v['gzip_bytes']and sha(b)==v['gzip_sha256'];raw=gzip.decompress(b);assert len(raw)==v['bytes']and sha(raw)==decoded
 if args.restored:
  for n,v in s['files'].items():b=(args.restored/n).read_bytes();assert len(b)==v['bytes']and sha(b)==v['sha256'],n
  for decoded,v in s['external_by_decoded_sha256'].items():
   for n in v['destinations']:assert sha((args.restored/n).read_bytes())==decoded
 print(json.dumps({'pass':True,'archiveMembers':len(s['files']),'externalPayloads':5,'restored':bool(args.restored),'scope':'exactbyte verification only; no replay/GPU/timing'}))
if __name__=='__main__':main()
