import json,subprocess,sys,hashlib,time
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0,'scripts')
from capture_spool import decoded_frames
root=Path('tmp/t2-capture-m0-r3/qualified')
# Recordings must finish first; no decoder/package/compiler runs between captures.
assert (root/'binary.sha256').exists()
rows=json.loads((root/'order.json').read_text())
def job(row):
 name=row['name'];out=root/name
 if name.startswith('writer-') and name.split('-')[-1].isdigit():
  r=json.loads((out/'result.txt').read_text());begin=time.monotonic_ns()
  count=sum(1 for _ in decoded_frames(out/'frames.zst','ZstdDelta',r['frame_sha256']))
  r.update(complete=count==480 and not r['misses'],decoded_frames_verified=count,deferred_verification_ns=time.monotonic_ns()-begin,full_flush_ns=row['full_flush_ns'],archive_sha256=hashlib.sha256((out/'frames.zst').read_bytes()).hexdigest())
  (out/'result.json').write_text(json.dumps(r,indent=2)+'\n');assert r['complete']
 elif row['exit']==0:
  cmd=['python3','scripts/capture_v1_runtime.py','--output',str(out),'--finalize']
  if name not in ['series-00','series-01','series-02','render-stall','still']:cmd+=['--verify-only']
  with (out/'finalize.log').open('w') as log:subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,check=True)
  assert json.loads((out/'manifest.json').read_text())['complete']
 print('VERIFIED',name,flush=True)
with ThreadPoolExecutor(max_workers=2) as pool:list(pool.map(job,rows))
(root/'VERIFIED').write_text('All successful recordings independently decoded against original hashes. Expected writer-slow remained incomplete.\n')
