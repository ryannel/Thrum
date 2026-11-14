"""Discard only this review's verified bulk spools; preserve selected lossless frames and original-time previews."""
from pathlib import Path
import json,hashlib
root=Path(__file__).resolve().parent
planned=[]
previous=json.loads((root/'bulk-selection.json').read_text())['removed'] if (root/'bulk-selection.json').exists() else []
for name in ['native-m1','native-m2']:
 folder=root/name
 if not folder.exists():continue
 report=json.loads((folder/'manifest.json').read_text())
 if not report['passed']:continue
 for p in folder.rglob('frames.zst'):
  m=json.loads(p.with_name('manifest.json').read_text())
  archive=m.get('lossless_source_archive',{})
  assert m['complete'] and archive['decoded_frames_verified']>0
  h=hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
  assert h==archive['sha256']
  assert p.with_name('preview-1x.mp4').exists() or list(p.parent.glob('checkpoint-0.*'))
  planned.append({'path':str(p.relative_to(root)),'sha256':h,'bytes':p.stat().st_size})
(root/'bulk-selection.json').write_text(json.dumps({'reason':'verified source archives reduced to selected lossless checkpoints, timing/identity manifests and original-time previews; recapture with retained runner to reproduce full source','removed':previous+planned},indent=2)+'\n')
for row in planned:(root/row['path']).unlink()
print('Removed verified owned spool bytes',sum(p['bytes'] for p in planned))
