#!/usr/bin/env python3
"""Restore the observed ae5533ec natural-crown diagnostic from pinned packages."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import zipfile

ROOT = Path(__file__).resolve().parent
BASE = ROOT.parent / 'cloud-source-integration-2026-11-03'
OPTICAL = ROOT.parent / 'cloud-natural-crown-cook-2026-11-03'
OVERLAY = (
    'assets/admission/admission.json', 'assets/input-index.json',
    'assets/templates/crown-b.json', 'assets/templates/original-FROZEN-INPUTS.json',
    'assets/templates/original-crown-b.json', 'index.html',
)
FIELDS = ('assets/templates/crown-b.density.r16f', 'assets/templates/crown-b.distance.r16f')


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(65536), b''):
            h.update(block)
    return h.hexdigest()


def contained(root, name):
    rel = PurePosixPath(name)
    if rel.is_absolute() or '..' in rel.parts or str(rel) != name:
        raise ValueError('Unsafe package path: ' + name)
    p = root.joinpath(*rel.parts)
    if not p.is_relative_to(root):
        raise ValueError('Escaping package path: ' + name)
    return p


def verify_package():
    manifest = json.loads((ROOT / 'manifest.json').read_text())
    for name, info in manifest['files'].items():
        p = contained(ROOT, name)
        if not p.is_file() or p.is_symlink() or p.stat().st_size != info['bytes'] or digest(p) != info['sha256']:
            raise ValueError('Package file mismatch: ' + name)
    deps = json.loads((ROOT / 'dependencies.json').read_text())
    for record in deps['external'].values():
        p = (ROOT / record['relative_path']).resolve()
        if not p.is_file() or digest(p) != record['sha256']:
            raise ValueError('External dependency mismatch: ' + record['relative_path'])
    assembly = json.loads((ROOT / 'frozen/ASSEMBLY.json').read_text())
    frozen = json.loads((ROOT / 'frozen/FREEZE.json').read_text())
    if frozen['build'] != 'ae5533ec5335b4ca' or len(frozen['files']) != 39:
        raise ValueError('Wrong frozen identity')
    if any(assembly['files'][name] != frozen['files'][name] for name in frozen['files']):
        raise ValueError('Assembly identity differs from freeze')
    with zipfile.ZipFile(ROOT / 'observations/receipts-and-stills.zip') as z:
        names = set(z.namelist())
        for record in json.loads((ROOT / 'observations/inventory.json').read_text())['files']:
            name = record['name']
            if 'standalone' in record['retention']:
                p = contained(ROOT / 'observations', name)
                if not p.is_file() or p.stat().st_size != record['bytes'] or digest(p) != record['sha256']:
                    raise ValueError('Standalone observation mismatch: ' + name)
            if not name.endswith('.webm'):
                if name not in names or '..' in PurePosixPath(name).parts:
                    raise ValueError('Missing/unsafe ZIP observation: ' + name)
                b = z.read(name)
                if len(b) != record['bytes'] or hashlib.sha256(b).hexdigest() != record['sha256']:
                    raise ValueError('ZIP observation mismatch: ' + name)
        if names != {r['name'] for r in json.loads((ROOT / 'observations/inventory.json').read_text())['files'] if not r['name'].endswith('.webm')}:
            raise ValueError('Extra ZIP observations')
    return frozen, len(manifest['files'])


def stream_field(source, dest, expected):
    h = hashlib.sha256()
    count = 0
    with gzip.open(source, 'rb') as src, dest.open('wb') as dst:
        while block := src.read(65536):
            h.update(block)
            count += len(block)
            dst.write(block)
    if count != 884736 or h.hexdigest() != expected:
        raise ValueError('Selected optical field mismatch: ' + str(source))


def restore(output, frozen):
    out = output.expanduser().resolve(strict=False)
    if out.exists() or not out.parent.is_dir() or out.is_relative_to(ROOT):
        raise ValueError('Choose a new output directory outside this package')
    subprocess.run([sys.executable, str(BASE / 'restore.py'), '--output', str(out)], check=True)
    for name in OVERLAY:
        dest = contained(out, name)
        shutil.copyfile(contained(ROOT / 'overlay', name), dest)
    shutil.copyfile(ROOT / 'frozen/serve.py', out / 'serve.py')
    for name in FIELDS:
        src = OPTICAL / 'selected' / (Path(name).name.replace('crown-b.', '') + '.gz')
        stream_field(src, contained(out, name), frozen['files'][name])
    files = {}
    for p in out.rglob('*'):
        if p.is_file() and (p.name in ('index.html', 'style.css') or
            (p.relative_to(out).parts[0] == 'src' and p.suffix == '.js') or
            (p.relative_to(out).parts[0] == 'shaders' and p.suffix == '.wgsl') or
            (p.relative_to(out).parts[0] == 'assets' and p.suffix in ('.r16f', '.json'))):
            files[p.relative_to(out).as_posix()] = digest(p)
    files = dict(sorted(files.items()))
    build = hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()[:16]
    if files != frozen['files'] or build != frozen['build']:
        raise ValueError('Restored served identity mismatch: ' + build)
    if digest(out / 'serve.py') != json.loads((ROOT / 'frozen/ASSEMBLY.json').read_text())['files']['serve.py']:
        raise ValueError('Server source mismatch')
    return out, build, len(files)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output', type=Path, help='new directory for restored diagnostic')
    args = p.parse_args()
    frozen, retained = verify_package()
    if args.output:
        out, build, served = restore(args.output, frozen)
        print(json.dumps({'result': 'PASS', 'output': str(out), 'build': build, 'served_files': served, 'package_files': retained, 'scope': 'Diagnostic restore only; no tests or acceptance'}))
    else:
        print(json.dumps({'result': 'PASS', 'build': frozen['build'], 'served_files': len(frozen['files']), 'package_files': retained, 'scope': 'Package verify only; no runtime or acceptance'}))


if __name__ == '__main__':
    main()
