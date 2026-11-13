import os
import subprocess
from pathlib import Path

root = Path.cwd()
for layer in (1, 2):
    name = f'shadow-map-{layer}'
    env = dict(os.environ,
        THRUM_SCENE='valley',
        THRUM_ASSET_STORE=str(root / 'tmp/g7-showcase/runs/culling-mac-metal-meshes-20261113-183637/store'),
        THRUM_CAPTURE_ROUTE=str(root / 'examples/v1-game/routes/moving-v1.json'),
        THRUM_CULLING_OUTPUT=str(root / 'tmp/culling-m3' / name),
        THRUM_VALLEY_MESHES='on', THRUM_RECIPE='portable',
        THRUM_CULLING_SOURCE='scene-linear', THRUM_SHADOW_DIAGNOSTIC=str(layer))
    with open(root / 'tmp/culling-m3' / f'{name}.log', 'w') as log:
        result = subprocess.run(['target/release/deps/valley_culling_gpu-3358ff9ee4447fb0', '--ignored', '--nocapture'], env=env, stdout=log, stderr=subprocess.STDOUT)
    print(name, result.returncode, flush=True)
