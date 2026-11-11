from pathlib import Path
import json,gzip,hashlib,sys
root=Path(__file__).resolve().parent
for name in ['native-m1','native-m2']:
 assert json.loads((root/name/'manifest.json').read_text())['passed']
paths=[p for name in ['native-m1','native-m2'] for p in (root/name).rglob('*') if p.suffix in ['.reference','.rgba8','.rgba16f','.surface-bgra8','.ppm']]
planned=[{'path':str(p.relative_to(root)),'bytes':p.stat().st_size,'sha256':hashlib.file_digest(p.open('rb'),'sha256').hexdigest()} for p in paths]
print('Owned verified readbacks:',len(planned),'bytes:',sum(x['bytes'] for x in planned))
if '--apply' not in sys.argv:raise SystemExit()
for recipe in ['portable','apple-tile']:
 p=root/'native-m1'/f'{recipe}-scene-linear'/'checkpoints/mirror-light.rgba16f'
 raw=p.read_bytes();gz=gzip.compress(raw,mtime=0);assert gzip.decompress(gz)==raw
 (root/'selected'/f'{recipe}-mirror-light.rgba16f.gz').write_bytes(gz)
(root/'readback-selection.json').write_text(json.dumps({'retained':'selected lossless PNGs, two mirror-light linear sources, original-time movies, exact per-checkpoint hashes/identities and comparison results; complete matched M0 references remain in the repository','removed':planned},indent=2)+'\n')
for p in paths:p.unlink()
