import json
import os
from pathlib import Path
import subprocess

root = Path.cwd()
base = root / 'tmp/culling-m3/metal-validation'
base.mkdir(exist_ok=True)
binary = root / 'target/release/deps/valley_culling_gpu-3358ff9ee4447fb0'
for number in range(5):
    env = dict(os.environ, MTL_DEBUG_LAYER='1', MTL_SHADER_VALIDATION='1',
               THRUM_SCENE='valley',
               THRUM_ASSET_STORE=str(root / 'tmp/g7-showcase/runs/culling-mac-metal-meshes-20261113-202120/store'),
               THRUM_CAPTURE_ROUTE=str(root / 'examples/v1-game/routes/moving-v1.json'),
               THRUM_CULLING_OUTPUT=str(base / str(number)), THRUM_VALLEY_MESHES='on',
               THRUM_RECIPE='portable', THRUM_CULLING_SOURCE='scene-linear')
    with (base / f'{number}.log').open('w') as log:
        result = subprocess.run([str(binary), '--ignored', '--nocapture'], env=env,
                                stdout=log, stderr=subprocess.STDOUT, timeout=240)
    print(json.dumps({'run': number, 'exit': result.returncode}), flush=True)
    if result.returncode:
        break
