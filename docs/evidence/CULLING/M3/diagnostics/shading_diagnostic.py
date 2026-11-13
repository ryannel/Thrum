import os
import subprocess
from pathlib import Path

root = Path.cwd()
with open('tmp/culling-m3/shading-build.log', 'w') as log:
    subprocess.run(['rustup','run','1.99.1','cargo','test','--locked','--offline','--release','-p','v1-game','--test','valley_culling_gpu','--no-run'], stdout=log, stderr=subprocess.STDOUT, check=True, timeout=180)
for kind, selector in [('air-map', 0), ('sun-factor', 0), ('shade-red', 7), ('world-x', 4), ('normal-x', 5), ('ambient-x', 6), ('base-red', 8)]:
    env = dict(os.environ,
        THRUM_SCENE='valley',
        THRUM_ASSET_STORE=str(root/'tmp/g7-showcase/runs/culling-mac-metal-meshes-20261113-183637/store'),
        THRUM_CAPTURE_ROUTE=str(root/'examples/v1-game/routes/moving-v1.json'),
        THRUM_CULLING_OUTPUT=str(root/'tmp/culling-m3'/kind),
        THRUM_VALLEY_MESHES='on', THRUM_RECIPE='portable', THRUM_CULLING_SOURCE='scene-linear', THRUM_SHADE_DIAGNOSTIC=str(selector))
    if kind == 'air-map': env['THRUM_AIR_MAP_DIAGNOSTIC']='1'
    with open(root/'tmp/culling-m3'/f'{kind}.log','w') as log:
        result=subprocess.run(['target/release/deps/valley_culling_gpu-3358ff9ee4447fb0','--ignored','--nocapture'],env=env,stdout=log,stderr=subprocess.STDOUT,timeout=120)
    print(kind,result.returncode,flush=True)
