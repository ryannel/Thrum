import fcntl, subprocess, os
from pathlib import Path
lock=Path(subprocess.check_output(['git','rev-parse','--git-common-dir'],text=True).strip())/'orchestrate-gpu.lock'
with lock.open('w') as f:
 fcntl.flock(f,fcntl.LOCK_EX)
 env=dict(os.environ,THRUM_EFFECT_GRAPH='1',THRUM_RECIPE='portable',THRUM_STARS='on',THRUM_CULLING='direct',THRUM_CAPTURE_ENCODING='zstd-delta')
 subprocess.run(['python3','scripts/capture_v1_runtime.py','--binary','target/release/v1-game','--store','tmp/exact-m0/valley','--output','docs/evidence/EFFECT-GRAPH/M2-review/capture','--case','route','--route','examples/v1-game/routes/effect-state.json','--mode','realtime','--defer'],env=env,check=True)
