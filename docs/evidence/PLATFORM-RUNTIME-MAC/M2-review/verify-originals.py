import sys,json,gzip,hashlib
from pathlib import Path
sys.path.insert(0,str(Path('scripts').resolve()))
from capture_spool import decoded_frames
base=Path('/Users/ryannel/Workspace/thrum/.claude/worktrees/mac-m2/tmp/mac-m2/current-20261113')
retained=Path('docs/evidence/render/mac-m2-20261113')
report={}
for rel in ['motion','window/grid-replacement','window/valley-replacement','presented/on-untraced','presented/on-traced']:
 if rel=='motion':
  m=json.load(gzip.open(retained/rel/'manifest.json.gz','rt')); hashes=[r['rgba_sha256'] for r in m['records']]; source=base/rel/'session/frames.zst'
 else:
  hashes=gzip.open(retained/rel/'hashes.txt.gz','rt').read().splitlines();source=base/rel/'frames.zst'
 count=0
 for raw in decoded_frames(source,'ZstdDelta',hashes):
  assert len(raw)==1920*1080*4
  assert hashlib.sha256(raw).hexdigest()==hashes[count]
  count+=1
 assert count==len(hashes)
 report[rel]={'originals_verified':count,'source':str(source)}
 print(rel,count,flush=True)
Path('docs/evidence/PLATFORM-RUNTIME-MAC/M2-review/originals-verified.json').write_text(json.dumps(report,indent=2)+'\n')
