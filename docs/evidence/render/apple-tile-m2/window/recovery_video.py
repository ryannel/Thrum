from pathlib import Path
import os,json,subprocess,time,re,sys
recipe=sys.argv[1];out=Path('tmp/g3t-m2/attended/visible')/(recipe+'-recovery-settled-video');out.mkdir(exist_ok=False)
env={k:v for k,v in os.environ.items() if not k.startswith('THRUM_')};env.update(THRUM_RECIPE=recipe,THRUM_MAC_QUALIFY=str(out.resolve()),THRUM_MAC_TAP='on',THRUM_MAC_REPLACE='1',THRUM_V1_FRAMES='10000',THRUM_SCENE='valley',THRUM_ASSET_STORE=str(Path('tmp/g3t-m0/valley').resolve()),THRUM_ROUTE_OVERLAY='proof',THRUM_STARS='on',THRUM_CULLING='direct',THRUM_PARAMS=str(Path('examples/v1-game/params/showcase-noon.params.json').resolve()))
log=(out/'game.log').open('wb');p=subprocess.Popen([str(Path('tmp/g3t-m2/bin/v1-game').resolve())],env=env,stdout=log,stderr=subprocess.STDOUT)
report={'pid':p.pid,'environment':{k:v for k,v in env.items() if k.startswith('THRUM_')},'clock':'time.monotonic_ns / time.time_ns','captures':[]};start=time.monotonic();rec=None;captured=set();window=None
while p.poll() is None and time.monotonic()-start<60:
 metadata=out/'window.txt'
 if metadata.exists() and window is None:
  window=int(re.search(r'^window_number=(\d+)',metadata.read_text(),re.M)[1]);ready=time.monotonic()
 if window and rec is None and time.monotonic()-ready>=1:
  report['record_start_unix_ns']=time.time_ns()
  rec=subprocess.Popen(['/usr/sbin/screencapture','-x','-v','-V','4','-l',str(window),str(out/'window-response.mov')],stdout=(out/'recorder.log').open('w'),stderr=subprocess.STDOUT)
 if window:
  for t in []:
   if time.monotonic()-ready>=t and t not in captured:
    captured.add(t);subprocess.run(['/usr/sbin/screencapture','-x','-o','-l',str(window),str(out/f'window-{t}s.png')],check=True)
    info=json.loads(subprocess.check_output(['tmp/g3t-m2/attended/windows',str(p.pid)]))
    report['captures'].append({'name':f'window-{t}s.png','unix_ns':time.time_ns(),'windows':info})
 time.sleep(.05)
if p.poll() is None:p.terminate()
report['game_exit']=p.wait(timeout=10);report['recorder_exit']=rec.wait(timeout=15) if rec else None
(out/'observer.json').write_text(json.dumps(report,indent=2)+'\n')
print(recipe,report['game_exit'],report['recorder_exit'])
