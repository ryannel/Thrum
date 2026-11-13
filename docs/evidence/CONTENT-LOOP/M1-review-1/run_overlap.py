"""Build the reviewed checkout and run the independent M1.3 overlap reproduction.
Run from the repository root. Needs Metal access. Expected exit on reviewed code: 101.
"""
import fcntl
import json
from pathlib import Path
import subprocess
import tempfile

root = Path.cwd()
folder = root / 'docs/evidence/CONTENT-LOOP/M1-review'
build = subprocess.run(
    ['cargo', 'bench', '--locked', '--offline', '-p', 'v1-game', '--bench',
     'cooked_edit', '--no-run', '--message-format=json'],
    text=True, stdout=subprocess.PIPE, check=True)
artifacts = {}
for line in build.stdout.splitlines():
    row = json.loads(line)
    if row.get('reason') == 'compiler-artifact':
        for filename in row['filenames']:
            if filename.endswith('.rlib'):
                artifacts[row['target']['name'].replace('-', '_')] = filename
with tempfile.TemporaryDirectory(prefix='content-loop-review-') as scratch:
    binary = str(Path(scratch) / 'overlap')
    command = ['rustc', '--edition=2024', str(folder / 'overlap.rs'),
               '-L', 'dependency=' + str(root / 'target/release/deps'), '-o', binary]
    for name in ['v1_game', 'thrum_asset', 'thrum_asset_gpu', 'thrum_cook',
                 'thrum_render', 'wgpu', 'pollster']:
        command.extend(['--extern', name + '=' + artifacts[name]])
    subprocess.run(command, check=True)
    common = subprocess.check_output(['git', 'rev-parse', '--git-common-dir'], text=True).strip()
    with open(Path(common) / 'orchestrate-gpu.lock', 'w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        result = subprocess.run([binary])
raise SystemExit(result.returncode)
