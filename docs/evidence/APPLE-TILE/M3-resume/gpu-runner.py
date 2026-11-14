#!/usr/bin/env python3
"""Give each Cargo test invocation one shared GPU lease, after merge's lock."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time

root = Path(__file__).resolve().parent
if Path(sys.argv[1]).parent.name == 'deps':
    lease = Path(os.environ['APPLE_TILE_M3_LEASE'])
    group = lease / ('cargo-' + str(os.getppid()))
    group.mkdir(exist_ok=True)
    try:
        fd = os.open(group / 'started', os.O_CREAT | os.O_EXCL | os.O_WRONLY)
        os.close(fd)
        with (group / 'holder.log').open('ab') as log:
            subprocess.Popen(['python3', str(root / 'lease.py'), str(lease), str(group), str(os.getppid())],
                             stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT)
    except FileExistsError:
        pass
    while not (group / 'ready.json').exists():
        if (lease / 'release').exists():
            raise SystemExit('GPU lease owner ended')
        time.sleep(.2)
    os.kill(json.loads((group / 'ready.json').read_text())['holder_pid'], 0)
os.execv(sys.argv[1], sys.argv[1:])
