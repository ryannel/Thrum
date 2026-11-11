import subprocess,json,time
from pathlib import Path
out=Path('docs/evidence/ATMOSPHERE/M5-review')
commands=[['node','scripts/check_v1_browser.mjs','--case','environment'], *[['python3','scripts/environment_products.py',case,'--target',target] for case in ['off-parity','cost'] for target in ['mac-metal','chrome-webgpu']]]
commands.append(['cargo','test','--locked','--offline','-p','thrum-render','metal_h2_sampler_precision_report','--','--nocapture'])
results=[]
for i,c in enumerate(commands):
    name=f'target-check-{i}.log'
    with (out/name).open('w') as f:
        p=subprocess.run(c,stdout=f,stderr=subprocess.STDOUT)
    results.append(dict(command=c,exit=p.returncode,log=name))
    (out/'commands.json').write_text(json.dumps(results,indent=2)+'\n')
    print(i,p.returncode,flush=True)
