import os,time,json
from pathlib import Path
root=Path(__file__).resolve().parent/'profile-gpu'
check_pid=int((root/'check.pid').read_text())
ready=root/'ready.json.tmp'
ready.write_text(json.dumps({'holder_pid':os.getpid(),'check_pid':check_pid,'time_ns':time.time_ns()}))
ready.replace(root/'ready.json')
start=time.monotonic()
try:
 while not (root/'release').exists() and time.monotonic()-start<7500:
  try:os.kill(check_pid,0)
  except ProcessLookupError:break
  time.sleep(.2)
finally:
 (root/'finished').touch()
 print('profile GPU lease released',flush=True)
