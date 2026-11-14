"""Reviewer capture of the checked commit; run under the shared GPU lock."""
import json
import os
from pathlib import Path
import subprocess

root = Path.cwd()
out = root / 'tmp/culling-m3/review-21aa3fb7'
out.mkdir(parents=True, exist_ok=True)
def run(args):
    print('RUN', *args, flush=True)
    subprocess.run(args, check=True)
run(['cargo', 'build', '--locked', '--offline', '--release', '-p', 'v1-game', '-p', 'thrum-cook'])
run(['sh', 'examples/v1-game/web/build.sh'])
# Browser default reproduction retained separately in stars-default.json.
run(['target/release/thrum-cook', 'cook', 'examples/v1-game/assets/showcase', str(out/'store')])
for mode, route in [('fixed', 'moving-v1'), ('realtime', 'culling-fast-yaw')]:
    data = json.loads((root/f'examples/v1-game/routes/{route}.json').read_text())
    data['render_extent'] = [1920,1080]
    dest = out/f'{route}-1080.json'
    dest.write_text(json.dumps(data, indent=2)+'\n')
    if mode == 'fixed':
        env = dict(os.environ, THRUM_CAPTURE_SCENE=str(out/'native-fixed-route'), THRUM_ASSET_STORE=str(out/'store'), THRUM_SCENE='valley', THRUM_CAPTURE_ROUTE=str(dest), THRUM_ROUTE_OVERLAY='proof')
        print('RUN native fixed route through submitted-image capture', flush=True)
        subprocess.run(['target/release/v1-game'], env=env, check=True)
    else:
        run(['python3','scripts/capture_v1_runtime.py','--binary','target/release/v1-game','--store',str(out/'store'),'--case','route','--route',str(dest),'--mode',mode,'--output',str(out/f'native-{mode}')])
    run(['node','scripts/capture_v1_browser.mjs','--cook','target/release/thrum-cook','--mode',mode,'--route-file',f'examples/v1-game/routes/{route}.json','--output',str(out/f'chrome-{mode}')])
    run(['python3','scripts/package_v1_browser.py','--output',str(out/f'chrome-{mode}')])
print('CAPTURE COMPLETE',out,flush=True)
with open('docs/evidence/CULLING/M3-review/camera-culling-bench.log','w') as log:
    subprocess.run(['cargo','bench','--locked','--offline','-p','thrum-render','--bench','camera_culling'],stdout=log,stderr=subprocess.STDOUT,check=True)
