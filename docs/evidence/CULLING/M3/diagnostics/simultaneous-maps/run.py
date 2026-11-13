import hashlib
import json
import os
from pathlib import Path
import subprocess

root = Path.cwd()
binary = root / 'tmp/culling-m3/simultaneous-maps-v2-oracle'
base = root / 'tmp/culling-m3/simultaneous-maps-v2'
base.mkdir(exist_ok=True)
report = {'source': 'fa3cc02a plus simultaneous-maps.patch',
          'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'runs': []}
for number in range(1, 9):
    output = base / str(number)
    env = dict(os.environ, THRUM_SCENE='valley',
        THRUM_ASSET_STORE=str(root / 'tmp/g7-showcase/runs/culling-mac-metal-meshes-20261113-202120/store'),
        THRUM_CAPTURE_ROUTE=str(root / 'examples/v1-game/routes/moving-v1.json'),
        THRUM_CULLING_OUTPUT=str(output), THRUM_VALLEY_MESHES='on',
        THRUM_RECIPE='portable', THRUM_CULLING_SOURCE='scene-linear')
    with output.with_suffix('.log').open('w') as log:
        result = subprocess.run([str(binary), '--ignored', '--nocapture'], env=env,
                                stdout=log, stderr=subprocess.STDOUT, timeout=240)
    row = {'run': number, 'exit': result.returncode}
    if result.returncode:
        files = sorted(output.glob('failure-*-0-Direct.raw'))
        first = files[0]
        tick = first.name.split('-')[1]
        direct = first.read_bytes()
        row['tick'] = int(tick)
        row['differences'] = {}
        for host, mode in [(1, 'Frustum'), (2, 'Occlusion'), (3, 'Direct')]:
            other = (output / f'failure-{tick}-{host}-{mode}.raw').read_bytes()
            spans = [('colour', 0, 7_372_800, 8),
                     ('near', 7_372_800, 11_567_104, 4),
                     ('far', 11_567_104, 15_761_408, 4),
                     ('height', 15_761_408, 19_955_712, 4),
                     ('air', 19_955_712, 22_052_864, 8)]
            row['differences'][str(host)] = {name: sum(direct[i:i+pixel] != other[i:i+pixel]
                for i in range(start, end, pixel)) for name, start, end, pixel in spans}
    report['runs'].append(row)
    (base / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(row), flush=True)
    if result.returncode:
        break
