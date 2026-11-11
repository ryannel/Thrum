#!/usr/bin/env python3
"""Hold one agent-loop GPU lease across this check's remaining native tests."""
import os,sys,time,json,subprocess
from pathlib import Path
root=Path(__file__).resolve().parent
lease=root/'profile-gpu'
if Path(sys.argv[1]).parent.name == 'deps':
 try:
  fd=os.open(lease/'started',os.O_CREAT|os.O_EXCL|os.O_WRONLY);os.close(fd)
  log=(lease/'holder.log').open('ab')
  subprocess.Popen(['python3','/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.7.1/skills/orchestration/build/scripts/agent_loop.py','lock','gpu','--','python3',str(root/'profile-gpu-hold.py')],stdin=subprocess.DEVNULL,stdout=log,stderr=subprocess.STDOUT)
 except FileExistsError:pass
 while not (lease/'ready.json').exists():
  if (lease/'finished').exists():raise SystemExit('GPU lease ended before admission')
  time.sleep(.1)
 holder=json.loads((lease/'ready.json').read_text())
 os.kill(holder['holder_pid'],0)
 if (lease/'finished').exists():raise SystemExit('GPU lease ended before test execution')
os.execv(sys.argv[1],sys.argv[1:])
