"""Reviewer audit of retained data; run from the repository root."""
import json
import math
import pathlib
import struct
import subprocess

ROOT = pathlib.Path.cwd()
OUT = ROOT / "docs/evidence/SIM-2D/M1-review"

def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]

def bits(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]

def adjacent(value, up):
    word = bits(value)
    if value == 0:
        word = 1 if up else 0x80000001
    else:
        word += 1 if (value > 0) == up else -1
    return struct.unpack('<f', struct.pack('<I', word))[0]

def converted(axis):
    value = axis * 32767
    integer = math.floor(value + 0.5) if value >= 0 else math.ceil(value - 0.5)
    return bits(f32(integer / 32767))

vectors = json.loads((ROOT / 'crates/thrum-core/src/profile/vectors.json').read_text())
expected = [converted(f32(k / 32767)) for k in range(-32767, 32768)]
for k in range(-32767, 32767):
    boundary = f32((k + 0.5) / 32767)
    expected.extend(converted(v) for v in [adjacent(boundary, False), boundary, adjacent(boundary, True)])
expected.extend(converted(f32(v)) for v in [-1, 1, -0.0, 0, 0.1, 0.5, -0.5, 0.3, -0.7, -0.9])
assert vectors[3790:] == expected
assert len(vectors) == 265937
old = json.loads(subprocess.check_output(['git', 'show', '11d307ef1:crates/thrum-core/src/profile/vectors.json']))
assert vectors[:3790] == old
crowd = ROOT / 'examples/crowd-2d/retained/crowd.txt'
assert crowd.read_bytes() == subprocess.check_output(['git', 'show', '61b84d7e3:examples/crowd-2d/retained/crowd.txt'])
folds, checkpoints = {}, {}
for line in crowd.read_text().splitlines():
    if not line or line.startswith('#'):
        continue
    kind, size, tick, value = line.split()
    if kind == 'fold':
        folds[int(size)] = value
    elif kind == 'checkpoint':
        checkpoints.setdefault(int(size), []).append([int(tick), value])
targets = {}
for target in ['mac-metal', 'chrome-webgpu', 'ios-sim']:
    record = json.loads((ROOT / f'tmp/sim2d/run-{target}.json').read_text())
    assert record['profile_vectors'] == 'pass' and not record['dirty']
    assert len(record['rows']) == (5 if target == 'mac-metal' else 4)
    for row in record['rows']:
        assert row['fold'] == folds[row['bodies']]
        assert row['checkpoints'] == checkpoints[row['bodies']]
        assert all(row[k] > 0 for k in ['reused', 'hits', 'rewinds', 'changed_rewinds'])
        if row['wire']:
            assert row['changed_axes'] > 0
    targets[target] = {k: v for k, v in record.items() if k != 'rows'}
cost_flags = {}
for target in ['mac-metal', 'chrome-webgpu']:
    record = json.loads((ROOT / f'docs/evidence/SIM-2D/M1/costs-{target}.json').read_text())
    flags = []
    for run in record['runs']:
        for index, row in enumerate(run['rows']):
            assert row['clock_tripwire'] == (row['tick_median_ms'] > 1000 / 60)
            scaling = index > 0 and row['tick_ms_per_neighbour'] > run['rows'][index - 1]['tick_ms_per_neighbour']
            assert row['super_neighbour_scaling_tripwire'] == scaling
            assert row['quarter_slower_tripwire'] == any(c['ratio'] > 1.25 for c in row['quarter_slower_comparisons'])
            assert row['quarter_slower_comparisons'][0]['origin'] == 'previous_record'
            flags.append([run['repeat'], row['bodies'], row['clock_tripwire'], row['quarter_slower_tripwire'], scaling])
    cost_flags[target] = flags
result = {'result': 'pass', 'reviewed_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
          'input_vector_words_independently_checked': len(expected), 'arithmetic_prefix_unchanged': True,
          'crowd_reference_unchanged_since_initial_implementation': True, 'script_target_receipts': targets,
          'cost_flags_repeat_size_clock_quarter_scaling': cost_flags}
(OUT / 'audit.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'result': 'pass', 'input_vector_words_checked': len(expected), 'targets': list(targets)}))
