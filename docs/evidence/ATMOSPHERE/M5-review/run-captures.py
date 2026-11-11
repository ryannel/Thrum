import subprocess,json,os
from pathlib import Path
out=Path('docs/evidence/ATMOSPHERE/M5-review')
scratch=Path('tmp/m5-independent-review');scratch.mkdir(exist_ok=True)
base=json.loads(Path('examples/v1-game/routes/moving-v1.json').read_text())
# Same route on both targets; the default route includes approach, turn and cut.
for name,hour in [('noon',12),('low',17.3)]:
    route=json.loads(json.dumps(base));route['overlay']='off';route['start']['solar_hours']=hour
    (out/f'{name}-route.json').write_text(json.dumps(route,indent=2)+'\n')
(out/'west-route.json').write_bytes(Path('docs/evidence/ATMOSPHERE/m5/west-route.json').read_bytes())
results=[]
for target in ['metal','chrome']:
  for scene in ['noon','low','west']:
    for provider in ['off','on']:
      for mode in (['realtime'] if target=='metal' else ['fixed','realtime']):
        name=f'{target}-{scene}-{provider}-{mode}'
        dest=scratch/name
        if target=='metal':
          cmd=['python3','scripts/capture_v1_runtime.py','--binary','target/release/v1-game','--store','tmp/atmosphere-m5/store','--output',str(dest),'--mode',mode,'--case','route','--route',str(out/f'{scene}-route.json'),'--defer']
        else:
          cmd=['node','scripts/capture_v1_browser.mjs','--output',str(dest),'--mode',mode,'--scene','day','--overlay','off','--diagnostic-clouds',provider,'--route-file',str(out/f'{scene}-route.json')]
        with (out/f'{name}.log').open('w') as f:
          result=subprocess.run(cmd,env=dict(os.environ,THRUM_DIAGNOSTIC_CLOUDS=provider),stdout=f,stderr=subprocess.STDOUT)
        results.append(dict(name=name,command=cmd,exit=result.returncode,path=str(dest)))
        (out/'captures.json').write_text(json.dumps(results,indent=2)+'\n')
        print(name,result.returncode,flush=True)
        if result.returncode==0:
          subprocess.run(["python3",str(out/"select-capture.py"),str(dest)],check=True)
