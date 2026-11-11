"""Unattended OS window state/control and retained native replacement observer."""
import os,json,time,subprocess,hashlib,sys,ctypes
from pathlib import Path
root=Path('tmp/g3t-m3/window');root.mkdir(parents=True,exist_ok=True)
raw=subprocess.check_output(['/usr/sbin/ioreg','-n','Root','-d1'],text=True)
cg=ctypes.CDLL('/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics');cg.CGPreflightScreenCaptureAccess.restype=ctypes.c_bool
preflight={'unix_ns':time.time_ns(),'console_locked':'"IOConsoleLocked" = Yes' in raw,'screen_recording_granted':cg.CGPreflightScreenCaptureAccess(),'grant_requested':False}
(root/'preflight.json').write_text(json.dumps(preflight,indent=2)+'\n')
if preflight['console_locked'] or not preflight['screen_recording_granted']:
 raise SystemExit('BLOCKED: OS window proof needs an unlocked session and the existing observer grant')
binary=Path('tmp/g3t-m3/bin/v1-game').resolve();sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
helper=Path('tmp/g3t-m2/attended/windows').resolve()
def windows(pid):return json.loads(subprocess.check_output([str(helper),str(pid)]))
def save(p,x):p.write_text(json.dumps(x,indent=2)+'\n')
def environment(recipe):
 env={k:v for k,v in os.environ.items() if not k.startswith('THRUM_')}
 env.update(THRUM_RECIPE=recipe,THRUM_ASSET_STORE=str(Path('tmp/g3t-m0/valley').resolve()),THRUM_SCENE='valley',THRUM_ROUTE_OVERLAY='proof',THRUM_STARS='on',THRUM_CULLING='direct',THRUM_V1_FRAMES='10000',THRUM_REPORT='1')
 return env
for recipe in ['portable','apple-tile']:
 out=root/recipe;out.mkdir(exist_ok=False)
 controls=out/'controls.json';controls.write_bytes(Path('examples/v1-game/params/showcase-sunset-clear-sun.params.json').read_bytes())
 env=environment(recipe);env['THRUM_PARAMS']=str(controls.resolve())
 with (out/'game.log').open('wb') as log:
  p=subprocess.Popen([str(binary)],env=env,stdout=log,stderr=subprocess.STDOUT)
  report={'pid':p.pid,'recipe':recipe,'binary_sha256':sha(binary),'environment':{k:v for k,v in env.items() if k.startswith('THRUM_')},'observer':'/usr/sbin/screencapture','observations':[]}
  try:
   deadline=time.monotonic()+20;w=None
   while time.monotonic()<deadline:
    candidates=[w for w in windows(p.pid) if 'Paused' in w.get('kCGWindowName','')]
    if candidates:w=candidates[0];break
    time.sleep(.1)
   if w is None:raise RuntimeError('No settled paused game window')
   def snapshot(name):
    w=next(w for w in windows(p.pid) if 'Thrum V1' in w.get('kCGWindowName',''))
    start=time.monotonic_ns();subprocess.run(['/usr/sbin/screencapture','-x','-o','-l',str(w['kCGWindowNumber']),str(out/(name+'.png'))],check=True)
    report['observations'].append({'state':name,'unix_ns':time.time_ns(),'monotonic_before_ns':start,'monotonic_after_ns':time.monotonic_ns(),'window':w,'controls':json.loads(controls.read_text())});save(out/'observer.json',report)
   time.sleep(1);snapshot('sun')
   with (out/'recorder.log').open('wb') as rec_log:
    rec=subprocess.Popen(['/usr/sbin/screencapture','-x','-v','-V','7','-l',str(w['kCGWindowNumber']),str(out/'control-response.mov')],stdout=rec_log,stderr=subprocess.STDOUT)
    for name,param in [('night','showcase-night-upslope'),('noon','showcase-noon')]:
     controls.write_bytes(Path('examples/v1-game/params',param+'.params.json').read_bytes());time.sleep(2);snapshot(name)
    report['video_exit']=rec.wait(timeout=15)
    if report['video_exit']:raise RuntimeError('OS control video failed')
   time.sleep(1)
  finally:
   if p.poll() is None:p.terminate()
   report['game_exit']=p.wait(timeout=10);save(out/'observer.json',report)
 print(recipe,'OS settled states and control-response complete')
