"""Fresh M3 target observations after the registered exact matrix passes."""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path.cwd()
sys.path.insert(0, str(ROOT / 'scripts'))
from apple_tile_m0 import ROOT as REPO, ROUTE, digest, finish_t2, run_t2, save
from package_v1_browser import run as package_browser

parser = argparse.ArgumentParser()
parser.add_argument('--verification', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
a = parser.parse_args()
proof = json.loads((a.verification / 'record.json').read_text())
assert proof['passed'] is True
output = a.output.resolve()
output.mkdir(exist_ok=True)
args = argparse.Namespace(binary=(ROOT / 'tmp/apple-tile-m3-integrated/qualified-v1-game'), store=a.verification.resolve() / 'valley')
report = {'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
          'binary_sha256': digest(args.binary), 'exact_content_key': proof['content_key'],
          'commands': [], 'native': {}, 'chrome': {}, 'passed': False}
if (output / 'report.json').exists():
    previous = json.loads((output / 'report.json').read_text())
    assert previous['binary_sha256'] == report['binary_sha256']
    assert previous['exact_content_key'] == report['exact_content_key']
    report = previous
report.setdefault('wrapper_sha256', []).append(digest(Path(__file__)))

def inspection_strips(directory):
    command = ['ffmpeg', '-v', 'error', '-i', str(directory / 'preview-1x.mp4'),
               '-vf', 'fps=10,scale=384:216,tile=5x8', '-fps_mode', 'vfr',
               str(directory / 'inspection-%02d.png')]
    subprocess.run(command, check=True)
    return [p.name for p in sorted(directory.glob('inspection-*.png'))]

def focused_tests():
    if len(report.get('tests', [])) == 3 and all(row['exit'] == 0 for row in report['tests']):
        return
    report['tests'] = []
    for name, selection in [('input-tests', 'synthetic::display::tests::input'),
                             ('timing-tests', 'timing::tests'),
                             ('frame-stats-tests', 'synthetic::frame_stats::tests')]:
        command = ['rustup', 'run', '1.99.1', 'cargo', 'test', '--locked', '--offline',
                   '-p', 'thrum-render', '--lib', selection, '--', '--test-threads=1']
        with (output / (name + '.log')).open('w') as log:
            completed = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
        report['tests'].append({'name': name, 'command': command, 'exit': completed.returncode})
        save(output / 'report.json', report)
        if completed.returncode:
            raise SystemExit(completed.returncode)

assert report['binary_sha256'] == proof['native_binary_sha256']
for name, expected in proof['wasm_artifact_hashes'].items():
    assert digest(ROOT / 'target/v1-game-web' / name) == expected, name
route = json.loads(ROUTE.read_text())
route_path = output / 'route.json'
save(route_path, dict(route, render_extent=[1920, 1080]))
for recipe in ['portable', 'apple-tile']:
    os.environ['THRUM_RECIPE'] = recipe
    for case in ['route', 'variants', 'render-stall']:
        name = recipe + '-' + case
        if name in report['native']:
            assert report['native'][name]['complete']
            continue
        options = {'mode': 'realtime', 'case': case}
        if case == 'route':
            options['route'] = route_path
        row = run_t2(args, output, name, options)
        row.update(finish_t2(output / name, viewing=True))
        row['inspection_strips'] = inspection_strips(output / name)
        report['native'][name] = row
        save(output / 'report.json', report)
os.environ.pop('THRUM_RECIPE', None)
focused_tests()
for scene, control, name in [('day', 'ordinary', 'chrome-day'), ('night', 'ordinary', 'chrome-night'),
                             ('day', 'resize', 'chrome-resize')]:
    dest = output / name
    if name in report['chrome']:
        assert report['chrome'][name]['complete']
        continue
    observer = ('docs/evidence/APPLE-TILE/M3-review/chrome-resize-observer.mjs'
                if control == 'resize' else 'scripts/capture_v1_browser.mjs')
    command = ['node', observer, '--mode', 'realtime', '--scene', scene, '--control', control,
               '--source', 'sdr', '--cook', 'target/release/thrum-cook', '--output', str(dest)]
    with (output / (name + '.log')).open('w') as log:
        subprocess.run(command, check=True, stdout=log, stderr=subprocess.STDOUT)
    report['commands'].append(command)
    package_browser(dest)
    manifest = json.loads((dest / 'manifest.json').read_text())
    rows = manifest['records']
    keep = {0, len(rows) // 2, len(rows) - 1}
    for i in range(1, len(rows)):
        if any(rows[i].get(key) != rows[i - 1].get(key) for key in ['history_epoch', 'presenter_generation']):
            keep.update(range(max(0, i - 2), min(len(rows), i + 3)))
    retained = {f"{rows[i]['submission_id']:06}.png" for i in keep}
    for path in (dest / 'png').glob('*.png'):
        if path.name not in retained:
            path.unlink()
    report['chrome'][name] = {'complete': manifest['complete'], 'images': len(rows),
                              'presenter_generations': sorted({row['presenter_generation'] for row in rows}),
                              'history_epochs': sorted({row['history_epoch'] for row in rows}),
                              'retained_pngs': sorted(retained), 'manifest_sha256': digest(dest / 'manifest.json')}
    report['chrome'][name]['inspection_strips'] = inspection_strips(dest)
    save(output / 'report.json', report)
report['native_target_audit'] = {}
for name in report['native']:
    runtime = json.loads((output / name / 'runtime.json').read_text())
    expected = 'Ok(Portable)' if name.startswith('portable-') else 'Ok(AppleTile)'
    assert runtime['recipe'] == expected
    assert runtime['backend'] == 'Metal' and runtime['adapter'] == 'Apple M3 Max'
    report['native_target_audit'][name] = {
        'recipe': runtime['recipe'], 'adapter': runtime['adapter'], 'backend': runtime['backend'],
        'history_epochs': sorted({image['history_epoch'] for image in runtime['images']})}
assert len(report['native']) == 6 and len(report['chrome']) == 3
report['passed'] = True
save(output / 'report.json', report)
print(json.dumps({'passed': True, 'output': str(output)}))
