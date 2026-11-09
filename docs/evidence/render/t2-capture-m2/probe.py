"""Minimal target prerequisite probe; run in this worktree under the gpu lock.

Temporarily attaches the existing cooked scene to the ordinary simulator host.
Restores source even on failure and deletes its owned simulator unless --device is given.
No capture implementation or renderer fallback is added.
"""
from pathlib import Path
import argparse,hashlib,json,os,shutil,subprocess,time
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument("--output",type=Path,required=True)
parser.add_argument("--store",type=Path,required=True)
parser.add_argument("--device",help="reuse this task's owned booted simulator")
args=parser.parse_args()
OUT=args.output
OUT.mkdir(exist_ok=False)
SOURCE=Path('examples/v1-game-ios/src/ios/render.rs')
original=SOURCE.read_text()
patched=original.replace('let host = pollster::block_on(FrameHost::new_with_capabilities(', 'let mut host = pollster::block_on(FrameHost::new_with_capabilities(').replace('let (host, controls) = host.into_threaded(self.origin, world_advanced)?;', 'v1_game::attach_cooked_scene(&mut host)?;\n        let (host, controls) = host.into_threaded(self.origin, world_advanced)?;').replace('let name = adapter.get_info().name;', 'let name = format!("{:?}; downlevel={:?}", adapter.get_info(), adapter.get_downlevel_capabilities());')
assert patched != original
report={'base_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(), 'source_before_sha256':hashlib.sha256(original.encode()).hexdigest(),'source_probe_sha256':hashlib.sha256(patched.encode()).hexdigest(),'target':'iPhone 18 Pro / iOS 27.2, simulator function only','device':args.device}
def run(args,env=None):
 p=subprocess.run(args,capture_output=True,text=True,timeout=90,env=env)
 report.setdefault('commands',[]).append({'argv':args,'exit':p.returncode,'stdout':p.stdout,'stderr':p.stderr})
 p.check_returncode();return p.stdout.strip()
try:
 if shutil.disk_usage(OUT).free < 1024**3: raise RuntimeError('probe needs 1 GiB free')
 report['host']=run(['sw_vers'])
 if not report['device']:
  report['device']=run(['xcrun','simctl','create','Thrum T2 prerequisite','com.apple.CoreSimulator.SimDeviceType.iPhone-18-Pro','com.apple.CoreSimulator.SimRuntime.iOS-27-2'])
  run(['xcrun','simctl','bootstatus',report['device'],'-b'])
 report['simulators']=json.loads(run(['xcrun','simctl','list','devices','-j']))
 SOURCE.write_text(patched)
 (OUT/'render-probe.rs').write_text(patched)
 (OUT/'probe.patch').write_text(subprocess.check_output(['git','diff','--',str(SOURCE)],text=True))
 run(['examples/v1-game-ios/ios/build-simulator.sh'])
 app=Path('target/v1-game-ios-sim/ThrumV1.app')
 report['binary_sha256']=hashlib.sha256((app/'ThrumV1').read_bytes()).hexdigest()
 report['cooked_hashes']={str(p.relative_to(args.store)):hashlib.sha256(p.read_bytes()).hexdigest() for p in args.store.rglob('*') if p.is_file()}
 report['console_before']=run(['ioreg','-n','Root','-d1','-l'])
 u=report['device']
 subprocess.run(['xcrun','simctl','terminate',u,'org.thrum.v1'],capture_output=True,timeout=30)
 run(['xcrun','simctl','install',u,str(app.resolve())])
 container=Path(run(['xcrun','simctl','get_app_container',u,'org.thrum.v1','data']))
 store=container/'tmp/t2-prerequisite-cooked'
 shutil.rmtree(store,ignore_errors=True)
 shutil.copytree(args.store,store)
 state=container/'tmp/thrum-v1-ios-state.txt';state.unlink(missing_ok=True)
 env=dict(os.environ,SIMCTL_CHILD_THRUM_V1_ROUTE='1')
 run(['xcrun','simctl','launch',u,'org.thrum.v1'],env=env)
 deadline=time.monotonic()+30
 while time.monotonic()<deadline:
  text=state.read_text() if state.exists() else ''
  if 'presented=' in text and int(dict(line.split('=',1) for line in text.splitlines() if '=' in line)['presented'])>=120:break
  time.sleep(.1)
 (OUT/'diagnostic-state.txt').write_text(text);report['diagnostic_state']=text
 subprocess.run(['xcrun','simctl','terminate',u,'org.thrum.v1'],capture_output=True,timeout=30)
 state.unlink(missing_ok=True)
 env.update(SIMCTL_CHILD_THRUM_SCENE='valley',SIMCTL_CHILD_THRUM_ASSET_STORE=str(store),SIMCTL_CHILD_THRUM_STARS='off')
 run(['xcrun','simctl','launch',u,'org.thrum.v1'],env=env)
 deadline=time.monotonic()+30
 while time.monotonic()<deadline:
  text=state.read_text() if state.exists() else ''
  if text.startswith('fault='):break
  time.sleep(.1)
 (OUT/'cooked-state.txt').write_text(text);report['cooked_state']=text
 report['console_after']=run(['ioreg','-n','Root','-d1','-l'])
 report['blocked']=text.startswith('fault=') and 'INDIRECT_EXECUTION' not in report['diagnostic_state'] and 'Apple_iOS_simulator_GPU' in report['diagnostic_state']
 report['interpretation']='Adapter omits INDIRECT_EXECUTION. Existing cooked meshlet draw requires INDIRECT buffers; cooked foreground faults before any T2 observer is enabled.'
 print(json.dumps({k:v for k,v in report.items() if k in ['binary_sha256','blocked','cooked_state']},indent=2))
finally:
 SOURCE.write_text(original)
 if report['device']:
  subprocess.run(['xcrun','simctl','terminate',report['device'],'org.thrum.v1'],capture_output=True,timeout=30)
  if not args.device:
   run(['xcrun','simctl','shutdown',report['device']])
   run(['xcrun','simctl','delete',report['device']])
 report['source_restored']=SOURCE.read_text()==original
 (OUT/'report.json').write_text(json.dumps(report,indent=2)+'\n')
