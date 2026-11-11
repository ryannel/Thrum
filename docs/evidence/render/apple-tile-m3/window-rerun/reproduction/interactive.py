exec(open('tmp/g3t-m3-mf1/run.py').read().split("for n in ['window-references.py'")[0])
import sys
recipe=sys.argv[1]
run(['python3',str(r/'interactive-config.py'),recipe])
subprocess.run(['open','-n','-a',str(r/'ThrumM2.app')],check=True)
helper=Path('tmp/g3t-m2/attended/windows').resolve()
for i in range(60):
 preflight();ws=json.loads(subprocess.check_output([str(helper)]))
 ws=[w for w in ws if w.get('kCGWindowOwnerName')=='ThrumM2' and 'Thrum V1' in w.get('kCGWindowName','')]
 if ws:break
 time.sleep(.5)
else:raise RuntimeError('No game window')
w=ws[0];pid=w['kCGWindowOwnerPID']
(r/f'{recipe}-interactive-ready.json').write_text(json.dumps(w,indent=2)+'\n')
waiting=time.monotonic()
while not (r/f'{recipe}-record-start').exists():
 preflight()
 if time.monotonic()-waiting>240:raise RuntimeError('Timed out waiting for CUA recorder start')
 time.sleep(.25)
rec=subprocess.Popen(['/usr/sbin/screencapture','-x','-v','-V','75','-l',str(w['kCGWindowNumber']),str(r/f'{recipe}-interactive.mov')],stdout=(r/f'{recipe}-interactive-recorder.log').open('w'),stderr=subprocess.STDOUT)
try:
 start=time.monotonic()
 while not (r/f'{recipe}-interactive-done').exists():
  preflight()
  if time.monotonic()-start>240:raise RuntimeError('Timed out waiting for CUA resize/input')
  time.sleep(.5)
finally:
 import signal
 rec.wait(timeout=90)
 os.kill(pid,signal.SIGTERM)
