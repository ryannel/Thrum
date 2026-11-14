"""Retain compact M3 proof and hash local masters, excluding bulk captures."""
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import xml.etree.ElementTree as ET

root = Path('tmp/mac-m3')
out = Path('docs/evidence/PLATFORM-RUNTIME-MAC/M3')


def archive(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    if source.suffix in ('.json', '.tsv', '.txt', '.xml', '.log', '.py'):
        destination.with_suffix(destination.suffix + '.gz').write_bytes(gzip.compress(source.read_bytes(), mtime=0))
    else:
        shutil.copy2(source, destination)


def arm(source, destination):
    destination.mkdir(parents=True, exist_ok=True)
    for path in source.iterdir():
        if path.is_file() and path.suffix in ('.json', '.tsv', '.txt', '.xml', '.log', '.py', '.png', '.mp4'):
            archive(path, destination / path.name)
    for folder in ('png', 'native-selections'):
        if (source / folder).is_dir():
            shutil.copytree(source / folder, destination / folder, dirs_exist_ok=True)


for name in ('analysis.json', 'cadence.json', 'recording-commission.json'):
    archive(root / name, out / name)
for name in ('grid-replacement', 'valley-replacement', 'off-probe', 'on-untraced-current'):
    arm(root / 'window' / name, out / 'window' / name)
for name in ('identity.json', 'preflight.json', 'display.txt'):
    archive(root / 'window' / name, out / 'window' / name)
for name in ('off-traced', 'on-untraced', 'on-traced'):
    arm(root / 'presented' / name, out / 'presented' / name)
    directory = root / 'presented' / name
    if name != 'on-untraced':
        counts = {}
        for filename in ('presented.xml', 'displays.xml', 'raw-displays.xml', 'swaps.xml'):
            tree = ET.parse(directory / filename)
            counts[filename] = [{'schema': node.get('schema'), 'rows': len(node.findall('row'))}
                                for node in tree.findall('.//node') if node.find('schema') is not None]
            if not counts[filename]:
                counts[filename] = [{'rows': len(tree.findall('.//row'))}]
        (out / 'presented' / name / 'row-counts.json').write_text(json.dumps(counts, indent=2) + '\n')
for name in ('source-audit.json', 'window-acceptance.json'):
    archive(root / 'sitting' / name, out / 'sitting' / name)
for path in (root / 'sitting').glob('*.json'):
    if path.name not in ('state.json', 'failure.json', 'ready-to-merge.json'):
        archive(path, out / 'sitting' / path.name)
for name in ('playback-motion', 'playback-window-pan', 'playback-on-untraced', 'playback-on-traced', 'playback-motion-clean'):
    directory = root / 'sitting' / name
    destination = out / 'review' / name
    destination.mkdir(parents=True, exist_ok=True)
    archive(directory / 'playback.json', destination / 'playback.json')
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-pattern_type', 'glob', '-i', str(directory / 'sample-*.png'),
                    '-vf', 'scale=480:270,tile=4x4', '-frames:v', '1', str(destination / 'sample-strip.png')], check=True)
archive(root / 'playback-motion/playback.json', out / 'review/headless-diagnostic.json')
merge_log = Path('/Users/ryannel/Workspace/thrum/.git/orchestrate/jobs/20261114-044357-3527.log')
(out / 'source-merge.json').write_text(merge_log.read_text())
proof = next(Path('tmp/baseline-guard/proofs').glob('4becb6b204af736a69fc259c0f51773d0c602141a8a9467ddda15984ac04464c.json'))
archive(proof, out / 'exact-baselines.json')
masters = [root / 'v1-game', root / 'motion/session/frames.zst']
masters += list((root / 'valley').rglob('*'))
masters += list((root / 'window').glob('*/frames.zst'))
masters += list((root / 'presented').glob('*/frames.zst'))
for trace in (root / 'presented').glob('*/pan.trace'):
    masters += list(trace.rglob('*'))
records = []
for path in sorted(set(p for p in masters if p.is_file())):
    with path.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    records.append({'path': str(path), 'bytes': path.stat().st_size, 'sha256': digest})
(out / 'retained-masters.json').write_text(json.dumps(records, indent=2) + '\n')
print('Retained', len(records), 'local master identities and compact M3 proof.')
