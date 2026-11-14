"""Run the official command with GPU-serialized native tests."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parent
lease = Path(tempfile.mkdtemp(prefix='lease-', dir=root))
env = dict(os.environ, CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER=str(root / 'gpu-runner.py'), APPLE_TILE_M3_LEASE=str(lease))
loop = '/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/2.2.2/skills/orchestration/build/scripts/agent_loop.py'
p = subprocess.Popen(['python3', loop, *sys.argv[1:]], env=env)
(lease / 'owner.pid').write_text(str(p.pid))
try:
    code = p.wait()
finally:
    (lease / 'release').touch()
raise SystemExit(code)
