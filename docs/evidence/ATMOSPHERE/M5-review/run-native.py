import subprocess,json,os
from pathlib import Path
out=Path('docs/evidence/ATMOSPHERE/M5-review'); results=[]
for scene in ['noon','low','west']:
 route=json.loads((out/f'{scene}-route.json').read_text());route['render_extent']=[1920,1080]
 routepath=out/f'metal-{scene}-route.json';routepath.write_text(json.dumps(route,indent=2)+'\n')
 for provider in ['off','on']:
  name=f'metal-{scene}-{provider}-realtime-r2';dest=Path('tmp/m5-independent-review')/name
  cmd=['python3','scripts/capture_v1_runtime.py','--binary','target/release/v1-game','--store','tmp/atmosphere-m5/store','--output',str(dest),'--mode','realtime','--case','route','--route',str(routepath),'--defer']
  with (out/f'{name}.log').open('w') as f:result=subprocess.run(cmd,env=dict(os.environ,THRUM_DIAGNOSTIC_CLOUDS=provider),stdout=f,stderr=subprocess.STDOUT)
  results.append(dict(name=name,command=cmd,exit=result.returncode,path=str(dest)))
  (out/'native-captures.json').write_text(json.dumps(results,indent=2)+'\n');print(name,result.returncode,flush=True)
  if result.returncode==0:subprocess.run(['python3',str(out/'select-capture.py'),str(dest)],check=True)
# Re-select fixed captures by scheduled sample ordinal, retaining the initial one-second hold.
for scene,provider in [('noon','off'),('noon','on'),('low','off')]:
 name=f'chrome-{scene}-{provider}-fixed-r2';dest=Path('tmp/m5-independent-review')/name
 cmd=['node','scripts/capture_v1_browser.mjs','--output',str(dest),'--mode','fixed','--scene','day','--overlay','off','--diagnostic-clouds',provider,'--route-file',str(out/f'{scene}-route.json')]
 with (out/f'{name}.log').open('w') as f:result=subprocess.run(cmd,stdout=f,stderr=subprocess.STDOUT)
 results.append(dict(name=name,command=cmd,exit=result.returncode,path=str(dest)))
 (out/'native-captures.json').write_text(json.dumps(results,indent=2)+'\n');print(name,result.returncode,flush=True)
 if result.returncode==0:subprocess.run(['python3',str(out/'select-capture.py'),str(dest)],check=True)
