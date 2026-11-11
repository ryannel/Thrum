import subprocess,os
from pathlib import Path
out=Path('docs/evidence/APPLE-TILE/M3-review').resolve()
store=out/'valley'
steps=[['target/release/thrum-cook','cook','examples/v1-game/assets/showcase',str(store)],['python3','scripts/apple_tile_m1.py','--store',str(store),'--output',str(out/'native-m1')],['python3','scripts/apple_tile_m2.py','--store',str(store),'--output',str(out/'native-m2')]]
for step in steps:
 print('RUN',step,flush=True)
 r=subprocess.run(step)
 if r.returncode: raise SystemExit(r.returncode)
