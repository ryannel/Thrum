import subprocess,json,shutil
from pathlib import Path
out=Path('docs/evidence/APPLE-TILE/M3-review').resolve()
subprocess.run(['python3',str(out/'compact-native.py')],check=True)
scratch=Path('tmp/apple-tile-m3-review-chrome').resolve()
scratch.mkdir(exist_ok=True)
entries=json.loads(Path('.config/exact-baselines.json').read_text())['baselines'][1:]
results=[]
for e in entries:
 p=scratch/e['id']
 cmd=['node','scripts/capture_v1_browser.mjs','--mode','fixed','--scene',e['scene'],'--source',e['source'],'--cook','target/release/thrum-cook','--output',str(p)]
 r=subprocess.run(cmd)
 if r.returncode: raise SystemExit(r.returncode)
 cmd=['python3','scripts/apple_tile_m1_compare_browser.py','--reference-digest','--reference',e['reference'],'--candidate',str(p),'--output',str(out/(e['id']+'-comparison.json'))]
 if e['source']=='scene-linear': cmd.append('--linear')
 r=subprocess.run(cmd)
 results.append(dict(id=e['id'],exit=r.returncode))
 (out/'chrome-comparisons.json').write_text(json.dumps(results,indent=2)+'\n')
 if r.returncode: raise SystemExit(r.returncode)
 subprocess.run(['python3',str(out/'compact-browser.py'),str(p)],check=True)
 shutil.move(str(p),str(out/e['id']))
for scene in ['day','night']:
 p=scratch/('chrome-'+scene+'-realtime')
 subprocess.run(['node','scripts/capture_v1_browser.mjs','--mode','realtime','--scene',scene,'--cook','target/release/thrum-cook','--output',str(p)],check=True)
 subprocess.run(['python3','scripts/package_v1_browser.py','--output',str(p)],check=True)
 subprocess.run(['python3',str(out/'compact-browser.py'),str(p)],check=True)
 shutil.move(str(p),str(out/p.name))
p=scratch/'chrome-resize'
subprocess.run(['node','scripts/capture_v1_browser.mjs','--mode','realtime','--control','resize','--cook','target/release/thrum-cook','--output',str(p)],check=True)
shutil.move(str(p),str(out/p.name))
scratch.rmdir()
