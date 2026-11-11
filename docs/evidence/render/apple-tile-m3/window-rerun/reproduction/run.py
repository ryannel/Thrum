import subprocess,time,json,ctypes,os
from pathlib import Path
r=Path(__file__).resolve().parent
cg=ctypes.CDLL('/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics');cg.CGPreflightScreenCaptureAccess.restype=ctypes.c_bool
def preflight():
 d={'unix_ns':time.time_ns(),'monotonic_ns':time.monotonic_ns(),'console_locked':'"IOConsoleLocked" = Yes' in subprocess.check_output(['/usr/sbin/ioreg','-n','Root','-d1'],text=True),'screen_recording_granted':cg.CGPreflightScreenCaptureAccess(),'grant_requested':False}
 with (r/'session.jsonl').open('a') as f:f.write(json.dumps(d)+'\n')
 if d['console_locked'] or not d['screen_recording_granted']:raise RuntimeError('BLOCKED: session locked or observer grant unavailable')
def run(args):
 preflight();p=subprocess.Popen(args,start_new_session=True)
 try:
  while p.poll() is None:preflight();time.sleep(1)
  if p.returncode:raise RuntimeError(f'Failed {args}: {p.returncode}')
 except BaseException:
  import signal
  
  try:os.killpg(p.pid,signal.SIGTERM)
  except ProcessLookupError:pass
  raise
for n in ['window-references.py','window.py']:
 run(['python3',str(r/n)])
for recipe in ['portable','apple-tile']:run(['python3',str(r/'recovery-video.py'),recipe])
(r/'automatic-complete').touch()
for recipe in ['portable','apple-tile']:run(['python3',str(r/'interactive.py'),recipe])
(r/'all-complete').touch()
