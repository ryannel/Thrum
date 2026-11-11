import json,os,signal,sys,time
from pathlib import Path
sys.path.insert(0,'scripts')
from qualify_mac_m2 import panel_witness
root=Path('tmp/mac-m2/presented-deferred-20261111')
observe=panel_witness(root)
while not (root/'off-traced').exists():
 try:
  os.kill(59690,0)
  observe()
 except ProcessLookupError:
  break
 except Exception as e:
  (root/'queue-blocked.json').write_text(json.dumps({'error':str(e),'unix_ns':time.time_ns()}))
  if not (root/'off-traced').exists():os.kill(59690,signal.SIGINT)
  raise
 time.sleep(.5)
print('queue guard finished; per-arm collector guard takes over if recording began',flush=True)
