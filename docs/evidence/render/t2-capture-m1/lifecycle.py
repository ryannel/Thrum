import subprocess,json,time
from pathlib import Path
root=Path('tmp/t2-capture-m1');order=[]
for case in ['tabs','resize','device-loss','device-recovery','moving-route-controls']:
 name='legacy-'+case
 command=['node','scripts/check_v1_browser.mjs','--case',case,'--output',str(root/name)]
 with (root/(name+'.log')).open('w') as log: result=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
 order.append({'case':case,'command':command,'exit_code':result.returncode})
 (root/'lifecycle-order.json').write_text(json.dumps(order,indent=2)+'\n')
 print(name,result.returncode,flush=True)
