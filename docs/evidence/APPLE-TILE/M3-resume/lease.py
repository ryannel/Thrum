"""Own a GPU lease only while its Cargo and top-level check/merge are alive."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time

loop = '/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/2.2.2/skills/orchestration/build/scripts/agent_loop.py'
if len(sys.argv) == 4:
    raise SystemExit(subprocess.call(['python3', loop, 'lock', 'gpu', '--', 'python3', __file__, *sys.argv[1:], 'held']))
lease, group, cargo = Path(sys.argv[1]), Path(sys.argv[2]), int(sys.argv[3])
owner = int((lease / 'owner.pid').read_text())
pending = group / 'ready.tmp'
pending.write_text(json.dumps({'holder_pid': os.getpid(), 'cargo_pid': cargo, 'owner_pid': owner}))
pending.replace(group / 'ready.json')
while not (lease / 'release').exists():
    try:
        os.kill(owner, 0)
        os.kill(cargo, 0)
    except ProcessLookupError:
        break
    time.sleep(.2)
