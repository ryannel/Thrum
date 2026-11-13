import hashlib
import json
import os
from pathlib import Path
import subprocess

root = Path.cwd()
base = root / 'tmp/culling-m3/pre-sample'
base.mkdir(exist_ok=True)
binary = root / 'tmp/culling-m3/pre-sample-oracle'
game = root / 'tmp/culling-m3/pre-sample-game'
report = {'source': 'c98b6578',
          'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
          'game_sha256': hashlib.sha256(game.read_bytes()).hexdigest(), 'runs': []}

def run(name, command, env=None):
    with (base / f'{name}.log').open('w') as log:
        result = subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=1200)
    row = {'name': name, 'exit': result.returncode}
    report['runs'].append(row)
    (base / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(row), flush=True)
    return result.returncode == 0

store = root / 'tmp/g7-showcase/runs/culling-mac-metal-meshes-20261113-202120/store'
if not run('native-matrix', ['python3', 'scripts/apple_tile_m1.py', '--binary', str(game),
           '--store', str(store), '--output', str(base / 'native-matrix')]):
    raise SystemExit(1)
for number in range(1, 5):
    for recipe in ['portable', 'apple-tile']:
        name = f'{number}-{recipe}'
        env = dict(os.environ, THRUM_SCENE='valley', THRUM_ASSET_STORE=str(store),
            THRUM_CAPTURE_ROUTE=str(root / 'examples/v1-game/routes/moving-v1.json'),
            THRUM_CULLING_OUTPUT=str(base / name), THRUM_VALLEY_MESHES='on',
            THRUM_RECIPE=recipe, THRUM_CULLING_SOURCE='scene-linear')
        if not run(name, [str(binary), '--ignored', '--nocapture'], env):
            raise SystemExit(1)
