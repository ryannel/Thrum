"""Audit the current source seam and actual named optional-pass records."""
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

root = Path.cwd()
output = root / 'tmp/apple-tile-m3-resume'
verification = root / 'tmp/baseline-guard/verify-20261113T234610.700747Z'
old = json.loads((root / 'docs/evidence/render/apple-tile-m3/provenance.json').read_text())
changed = [p for p, expected in old['source_files'].items()
           if not (root / p).is_file() or hashlib.sha256((root / p).read_bytes()).hexdigest() != expected]
consumers = subprocess.check_output(['rg', '-l', 'FrameSample|gpu_pass_ns', 'crates', 'examples', 'scripts',
                                     '-g', '*.rs', '-g', '*.rs.in'], text=True).splitlines()
source = {'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
          'old_source_revision': old['revision'], 'changed_since_original_proof': changed,
          'timing_consumers': consumers,
          'display_shader_diff_since_original': subprocess.check_output([
              'git', 'diff', old['revision'], 'HEAD', '--',
              'crates/thrum-render/src/synthetic_display.wgsl', 'crates/thrum-render/src/present.wgsl'], text=True)}
(output / 'source-audit.json').write_text(json.dumps(source, indent=2) + '\n')
result = {}
for path in verification.glob('*-motion-records.json.gz'):
    archive = json.loads(gzip.decompress(path.read_bytes()))
    runtime = archive['runtime.json']
    samples = runtime['gpu']
    maps = [sample['gpu_pass_ns'] for sample in samples]
    assert all(isinstance(m, dict) and 'timestamp query resolve/readback' in m
               and m['timestamp query resolve/readback'] is None for m in maps)
    assert all(sample['gpu_present_ns'] == sample['gpu_pass_ns']['V1 surface present'] for sample in samples)
    assert runtime['complete'] and archive['manifest.json']['complete']
    result[path.name] = {
        'adapter': runtime['adapter'], 'backend': runtime['backend'], 'samples': len(samples),
        'named_samples': len(maps), 'star_present': sum(any('star' in k for k in m) for m in maps),
        'star_absent': sum(not any('star' in k for k in m) for m in maps),
        'nullable_keys': sorted({k for m in maps for k, v in m.items() if v is None}),
        'all_pass_names': sorted({k for m in maps for k in m}), 'example': samples[0],
        'source_records_complete': runtime['complete']}
(output / 'timing-audit.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'changed_sources': len(changed), 'consumer_files': len(consumers), 'motion_runs': len(result)}))
