"""Check actual integrated named-pass and source-accounting observations."""
import json
from pathlib import Path

root = Path(__file__).resolve().parent
capture = root / 'captures'
report = json.loads((capture / 'report.json').read_text())
assert report['passed']
result = {'revision': report['revision'], 'native': {}, 'chrome': {}}
for name in report['native']:
    runtime = json.loads((capture / name / 'runtime.json').read_text())
    maps = [sample['gpu_pass_ns'] for sample in runtime['gpu']]
    assert runtime['complete']
    assert maps and all(isinstance(m, dict) for m in maps)
    assert all(m.get('timestamp query resolve/readback', False) is None for m in maps)
    assert all(s['gpu_present_ns'] == s['gpu_pass_ns']['V1 surface present'] for s in runtime['gpu'])
    result['native'][name] = {
        'complete': True, 'samples': len(maps),
        'star_present': sum(any('star' in k for k in m) for m in maps),
        'star_absent': sum(not any('star' in k for k in m) for m in maps),
        'unavailable': sorted({k for m in maps for k, v in m.items() if v is None}),
    }
for name in report['chrome']:
    manifest = json.loads((capture / name / 'manifest.json').read_text())
    assert manifest['complete']
    result['chrome'][name] = {
        'complete': True, 'images': len(manifest['records']),
        'presenter_generations': report['chrome'][name]['presenter_generations'],
        'history_epochs': report['chrome'][name]['history_epochs'],
    }
(root / 'timing-audit.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result, indent=2))
