"""Reproduce the retained Mac M2 trace, load and clock observations."""

from pathlib import Path
import csv, json, re, statistics, xml.etree.ElementTree as E
from decimal import Decimal
ROOT = Path('tmp/mac-m2/final-window-20261110')

def table(path):
    with path.open() as f:
        return list(csv.DictReader(f, delimiter='\t'))

def quantiles(values):
    v = sorted(values)
    return {'count': len(v), 'min': v[0], 'median': statistics.median(v), 'p95': v[min(len(v) - 1, int(0.95 * len(v)))], 'max': v[-1]} if v else {'count': 0}

def calibration(p):
    pid = int(re.search('^pid=(\\d+)$', (p / 'window.txt').read_text(), re.M)[1])
    r = E.parse(p / 'signposts.xml').getroot()
    by = {e.get('id'): e for e in r.iter() if e.get('id')}

    def resolved(e):
        while e is not None and e.get('ref'):
            e = by[e.get('ref')]
        return e
    points = {}
    duplicates = 0
    for row in r.iter('row'):
        f = {e.tag: resolved(e) for e in row}
        name = f.get('signpost-name')
        process = f.get('process')
        kind = f.get('event-type')
        stamp = f.get('event-time')
        if name is None or name.text not in ('MacM2Whole', 'MacM2Pan') or process is None or (process.get('fmt') != f'v1-game ({pid})'):
            continue
        key = (name.text, kind.text)
        value = int(stamp.text)
        if key in points:
            assert points[key] == value
            duplicates += 1
        points[key] = value
    keys = [('MacM2Whole', 'Begin'), ('MacM2Pan', 'Begin'), ('MacM2Pan', 'End'), ('MacM2Whole', 'End')]
    anchors = []
    rows = table(p / 'signposts.tsv')
    for row, key in zip(rows, keys, strict=True):
        trace = Decimal(points[key]) / Decimal(10 ** 9)
        anchors.append({'stage': int(row['stage']), 'name': key[0], 'kind': key[1], 'trace_ns': points[key], 'offset_low_s': str(Decimal(row['media_before']) - trace), 'offset_high_s': str(Decimal(row['media_after']) - trace)})
    low = max((Decimal(a['offset_low_s']) for a in anchors))
    high = min((Decimal(a['offset_high_s']) for a in anchors))
    return {'pid': pid, 'anchors': anchors, 'duplicates_identical': duplicates, 'offset_intersection_s': [str(low), str(high)], 'offset_bound_us': float((high - low) * 10 ** 6), 'consistent': low <= high}

def row_counts(p, name):
    schemas = E.parse(p / 'toc.xml').getroot().findall('./run/data/table')
    r = E.parse(p / name).getroot()
    result = {}
    for n in r.findall('node'):
        i = int(re.search('table\\[(\\d+)\\]', n.get('xpath'))[1]) - 1
        result[schemas[i].get('schema')] = len(n.findall('row'))
    return result

def gpu_rows(p):
    result = {k: [] for k in ('render_cpu_ms', 'encode_cpu_ms', 'gpu_scene_ms', 'gpu_present_ms', 'gpu_contribution_ms')}
    missing = 0
    for block in (p / 'gpu-samples.txt').read_text().split('FrameSample {')[1:]:
        for field, label in [('render_cpu', 'render_cpu_ms'), ('encode_cpu', 'encode_cpu_ms')]:
            m = re.search('\\b' + field + ': ([\\d.]+)(ns|µs|ms|s),', block)
            if m:
                result[label].append(float(m[1]) * {'ns': 1e-06, 'µs': 0.001, 'ms': 1, 's': 1000}[m[2]])
        m = re.search('gpu_pass_ns: Some\\(\\s*\\[([^]]*)\\]', block)
        if m:
            result['gpu_scene_ms'].append(sum((float(x) for x in re.findall('[\\d.]+', m[1]))) / 1000000.0)
        else:
            missing += 1
        for field, label in [('gpu_present_ns', 'gpu_present_ms'), ('gpu_contribution_ns', 'gpu_contribution_ms')]:
            m = re.search(field + ': Some\\(\\s*([\\d.]+)', block)
            if m:
                result[label].append(float(m[1]) / 1000000.0)
    return {'statistics': {k: quantiles(v) for k, v in result.items()}, 'missing_gpu_scene_samples': missing, 'scope': 'render_cpu/encode_cpu are host method elapsed durations, including waits; GPU query intervals are separate; all samples include warmup; no isolated causal overhead claim'}
report = {}
for name in ('on-untraced-current', 'on-traced', 'off-traced'):
    p = ROOT / name
    s = table(p / 'surface.tsv')
    images = table(p / 'images.tsv')
    load = json.loads((p / 'load.json').read_text())
    witness = [json.loads(x) for x in (p / 'panel-witness.jsonl').read_text().splitlines()]
    r = {'surface_presentations': len(s), 'all_presented': all((x['presented'] == 'true' for x in s)), 'image_rows': len(images), 'tap_flags': sorted({x['tap_enabled'] for x in s}), 'spool_exists': (p / 'frames.zst').exists(), 'all_panel_samples_available': all((x['console_locked'] is False and x['builtin'] and x['active'] and (not x['asleep']) for x in witness)), 'panel_samples': len(witness), 'load': load, 'gpu_cpu': gpu_rows(p)}
    controls = table(p / 'controls.tsv')
    r['control_yaw_range'] = [min((float(x['accepted_yaw_degrees']) for x in controls)), max((float(x['accepted_yaw_degrees']) for x in controls))]
    samples = []
    for line in (p / 'system-iostat.txt').read_text().splitlines():
        try:
            row = [float(x) for x in line.split()]
        except ValueError:
            continue
        if len(row) >= 9:
            samples.append(row[-6:])
    r['host_idle_percent_one_second_samples'] = quantiles([x[2] for x in samples[1:]])
    if name != 'on-untraced-current':
        r['clock'] = calibration(p)
        (p / 'clock-calibration.json').write_text(json.dumps(r['clock'], indent=2) + '\n')
        r['swap_rows'] = row_counts(p, 'swaps.xml')
        r['raw_display_rows'] = row_counts(p, 'raw-displays.xml')
        r['tooling_qualified'] = False
    if (p / 'window-manifest.json').exists():
        m = json.loads((p / 'window-manifest.json').read_text())
        rows = m['records']
        r['images_verified'] = len(rows)
        r['missing_submissions'] = m['missing_submissions']
        r['max_image_gap_ms'] = max(((b['submit_ns'] - a['submit_ns']) / 1000000.0 for a, b in zip(rows, rows[1:])))
        r['preview_pts_max_error_s'] = m.get('preview_pts_max_error_seconds')
    report[name] = r
Path('tmp/mac-m2/analysis-20261111.json').write_text(json.dumps(report, indent=2) + '\n')
for name, r in report.items():
    print(name, {k: r[k] for k in ('surface_presentations', 'image_rows', 'all_panel_samples_available')}, 'swaps', r.get('swap_rows'), 'raw', r.get('raw_display_rows'), 'calibration us', r.get('clock', {}).get('offset_bound_us'))
