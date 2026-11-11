"""Start one required milestone check and bind its test GPU lease to its PID."""
import os,subprocess,sys
from pathlib import Path
root=Path(__file__).resolve().parent
lease=root/'profile-gpu'
lease.mkdir(exist_ok=True)
assert not (lease/'started').exists(),'Archive the previous, finished lease directory first'
env=dict(os.environ,CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER=str(root/'gpu-test-runner.py'))
with (root/'milestone-check.json').open('w') as log:
 p=subprocess.Popen(['python3','/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.7.1/skills/orchestration/build/scripts/agent_loop.py','check','--module','APPLE-TILE','--milestone','M3','--worktree','.','--timeout','7200','--json'],env=env,stdout=log)
 (lease/'check.pid').write_text(str(p.pid)+'\n')
 try:code=p.wait()
 finally:(lease/'release').touch()
raise SystemExit(code)
