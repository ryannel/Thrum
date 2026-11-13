"""Package selected proof without deleting the current-build source masters."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import xml.etree.ElementTree as ET


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def compressed(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    with source.open('rb') as stream, gzip.GzipFile(filename=str(destination), mode='wb', mtime=0) as archive:
        shutil.copyfileobj(stream, archive)


def copied(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)


def arm(source, destination):
    destination.mkdir(parents=True, exist_ok=True)
    tables = ('surface.tsv', 'requests.tsv', 'controls.tsv', 'images.tsv', 'signposts.tsv',
              'clock-anchor.tsv', 'panel-witness.jsonl', 'system-iostat.txt', 'gpu-samples.txt',
              'retirements.tsv', 'hashes.txt', 'window-manifest.json', 'toc.xml', 'presented.xml',
              'signposts.xml', 'displays.xml', 'raw-displays.xml', 'swaps.xml', 'witness.json')
    for name in tables:
        if (source / name).exists():
            compressed(source / name, destination / (name + '.gz'))
    metadata = ('invocation.json', 'recording-options.json', 'collection.json', 'load.json',
                'process.log', 'process.timing.json', 'window.txt', 'environment.json',
                'stale-recycle.txt', 'window-2s.json', 'window-6s.json', 'window-2s.png', 'window-6s.png')
    for name in metadata:
        if (source / name).exists():
            copied(source / name, destination / name)
    if (source / 'frames.zst').exists():
        (destination / 'source-archive.json').write_text(json.dumps({
            'path': str(source / 'frames.zst'), 'bytes': (source / 'frames.zst').stat().st_size,
            'sha256': sha(source / 'frames.zst'), 'encoding': 'ZstdDelta',
            'verification': 'package_mac_m2_window.py independently decoded and checked every original against hashes.txt',
        }, indent=2) + '\n')
    else:
        assert not (source / 'png').exists(), 'OFF must not produce images'
    if (source / 'toc.xml').exists():
        toc = ET.parse(source / 'toc.xml').getroot()
        schemas = toc.findall('./run/data/table')
        counts = {'recording_mode': toc.findtext('.//recording-mode'), 'tables': {}}
        for name in ('presented.xml', 'displays.xml', 'raw-displays.xml', 'swaps.xml'):
            for node in ET.parse(source / name).getroot().findall('node'):
                index = int(re.search(r'table\[(\d+)\]', node.get('xpath'))[1]) - 1
                counts['tables'][schemas[index].get('schema')] = len(node.findall('row'))
        (destination / 'row-counts.json').write_text(json.dumps(counts, indent=2) + '\n')
    if (source / 'window-manifest.json').exists():
        copied(source / 'preview-1x.mp4', destination / 'preview-1x.mp4')
        manifest = json.loads((source / 'window-manifest.json').read_text())
        records = manifest['records']
        selected = []
        for seconds in (3, 4.5, 5, 5.5, 6.5):
            row = min(records, key=lambda item: abs(item['submit_ns'] / 1e9 - seconds))
            name = f'full-{seconds:g}.png'
            copied(source / 'png' / f"{row['submission_id']:06}.png", destination / name)
            selected.append(dict(row, full=name))
        (destination / 'selected.json').write_text(json.dumps(selected, indent=2) + '\n')
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-ss', '3', '-i', str(source / 'preview-1x.mp4'),
                        '-t', '4.1', '-vf', 'fps=10,scale=384:216,tile=5x9', '-frames:v', '1',
                        str(destination / 'pan-strip.png')], check=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    root, output = args.root, args.output
    output.mkdir(parents=True, exist_ok=True)
    for name in ('build.json', 'cadence.json', 'known-off.json', 'known-on.json'):
        compressed(root / name, output / (name + '.gz'))
    for name in ('analysis.json', 'tool-tests.log'):
        copied(root / name, output / name)
    for path in root.glob('playback-*.json'):
        copied(path, output / path.name)
    copied(root / 'review_server.py', output / 'review_server.py')
    copied(root / 'window/identity.json', output / 'identity.json')
    copied(root / 'window/window-acceptance.json', output / 'window-acceptance.json')
    copied(root.parent / 'presented-commission-20261111.json', output / 'recording-commission.json')
    motion = root / 'motion/session'
    for name in ('manifest.json', 'runtime.json'):
        compressed(motion / name, output / 'motion' / (name + '.gz'))
    for name in ('preview-1x.mp4', 'contact.png'):
        copied(motion / name, output / 'motion' / name)
    for path in (root / 'selected-motion').iterdir():
        copied(path, output / 'motion' / path.name)
    for name in ('launcher.json', 'invocation.json'):
        copied(root / 'motion' / name, output / 'motion' / name)
    for name in ('grid-replacement', 'valley-replacement', 'off-probe'):
        arm(root / 'window' / name, output / 'window' / name)
    for name in ('off-traced', 'on-untraced', 'on-traced'):
        arm(root / 'presented' / name, output / 'presented' / name)
    inventory = []
    for path in root.rglob('*'):
        if path.is_file() and (path.name in ('frames.zst', 'v1-game') or 'pan.trace' in path.parts):
            inventory.append({'path': str(path), 'bytes': path.stat().st_size, 'sha256': sha(path)})
    (output / 'retained-masters.json').write_text(json.dumps(inventory, indent=2) + '\n')
    print('Promoted compact evidence. Compressed originals, trace bundles, executable and cooked store remain in', root)
