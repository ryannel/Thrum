import subprocess,os,json,time
from pathlib import Path
out=Path('docs/evidence/APPLE-TILE/M3-review')
env=dict(os.environ,RUSTC_WRAPPER='',RUST_TEST_THREADS='1')
steps=[('render-tests',['cargo','test','--locked','-p','thrum-render','--lib']),('native',['python3',str(out/'run-native.py')]),('moving',['python3','scripts/run_v1_showcase.py','moving']),('chrome',['python3',str(out/'run-chrome.py')]),('dev-check',['./dev','check'])]
results=[]
for name,cmd in steps:
 print('RUN',name,flush=True);start=time.time()
 with (out/(name+'.log')).open('w') as f:
  r=subprocess.run(cmd,stdout=f,stderr=subprocess.STDOUT,env=env)
 results.append(dict(name=name,command=cmd,exit=r.returncode,seconds=time.time()-start))
 (out/'checks.json').write_text(json.dumps(results,indent=2)+'\n')
 print('EXIT',name,r.returncode,flush=True)
