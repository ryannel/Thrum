"""Stage compact motion review artifacts; keep raw source archives in owned scratch."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import shutil

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('target',choices=['mac','chrome'])
parser.add_argument('--source',type=Path)
parser.add_argument('--output',type=Path)
args = parser.parse_args()
target = args.target
root = Path('tmp/apple-presentation-m3')
source = args.source or root / (target + '-motion-final')
out = args.output or root / 'selected-motion' / target
out.mkdir(parents=True, exist_ok=False)
campaign = json.loads((source/'campaign.json').read_text())
assert campaign['complete']
shutil.copy2(source/'campaign.json', out/'campaign.json')
retained = {}
for arm in campaign['arms']:
    name = arm['case'] + '-' + arm['reconstruction']
    session = source/name/('session' if target == 'mac' else '')
    dest = out/name
    dest.mkdir()
    for filename in ['runtime.json', 'manifest.json']:
        (dest/(filename+'.gz')).write_bytes(gzip.compress((session/filename).read_bytes(),mtime=0))
    shutil.copy2(session/'preview-1x.mp4',dest/'preview-1x.mp4')
    shutil.copy2(session/'selected.json',dest/'selected.json')
    manifest = json.loads((session/'manifest.json').read_text())
    source_record = ({'lossless_source': str(session/manifest['spool_file']),
                      'archive':manifest['lossless_source_archive']} if target == 'mac' else
                     {'lossless_source_directory':str(session),
                      'source_records':len(manifest['records']),
                      'source_layout':'One gzip RGBA source per manifest record.file; decoded SHA-256 is record.rgba_sha256.'})
    retained[name] = {**source_record, 'local_only':True,
                     'retention_reason':'Original writer-hash-verifiable frames for unresolved motion inspection; remove after motion items are qualified and selected retained artifacts verified.'}
for path in out.rglob('*'):
    if path.is_file():
        retained[str(path.relative_to(out))] = {'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'bytes':path.stat().st_size}
(out/'retention.json').write_text(json.dumps(retained,indent=2)+'\n')
print(out)
