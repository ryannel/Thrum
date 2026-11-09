import subprocess,json,time
from pathlib import Path
root=Path('tmp/t2-capture-m1')
order=[]
cases=[('fixed-final','fixed','ordinary')]+[(f'final-{i:02}','realtime','ordinary') for i in range(5)]+[(f'final-off-{i:02}','realtime','off') for i in range(3)]+[(name+'-final','realtime',name) for name in ['stall','resize','saturation','slow','loss']]
for name,mode,control in cases:
 command=['node','scripts/capture_v1_browser.mjs','--mode',mode,'--control',control,'--output',str(root/name)]
 begin=time.time()
 with (root/(name+'.log')).open('w') as log:
  result=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
 order.append({'name':name,'command':command,'started':begin,'ended':time.time(),'exit_code':result.returncode})
 (root/'final-order.json').write_text(json.dumps(order,indent=2)+'\n')
 print(name,result.returncode,flush=True)
