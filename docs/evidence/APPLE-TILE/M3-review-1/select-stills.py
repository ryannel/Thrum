import sys,json,subprocess
from pathlib import Path
sys.path.insert(0,'scripts')
from t2_writer_probe import package
out=Path('docs/evidence/APPLE-TILE/M3-review');selected=out/'selected';selected.mkdir(exist_ok=True)
for recipe in ['portable','apple-tile']:
 for name in ['approach','mirror-light','pre-cut','post-cut','near-plane']:
  raw=out/'native-m1'/f'{recipe}-sdr'/'checkpoints'/f'{name}.rgba8'
  if raw.exists():package(raw.read_bytes(),selected/f'{recipe}-{name}.png',1280,720)
