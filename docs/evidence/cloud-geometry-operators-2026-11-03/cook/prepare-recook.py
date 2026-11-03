#!/usr/bin/env python3
"""Prepare unchanged numerical recipes and SHA-pinned dense inputs in a fresh workspace."""
from pathlib import Path
import argparse,gzip,hashlib,json,shutil
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--workspace',type=Path,required=True);p.add_argument('--master-package',type=Path,default=ROOT.parent/'cloud-authored-masters-2026-11-03');a=p.parse_args()
if a.workspace.exists():raise RuntimeError('Refusing existing workspace')
source=a.workspace/'cloud-authored-master';source.mkdir(parents=True);cook=a.workspace/'cloud-distance-proof';cook.mkdir()
for r in json.loads((ROOT/'builds.json').read_text())['binary_inputs']:
 if r['kind']!='referenced-density':continue
 compressed=(a.master_package/r['master_package_path']).read_bytes();assert hashlib.sha256(compressed).hexdigest()==r['compressed_sha256'];raw=gzip.decompress(compressed);assert hashlib.sha256(raw).hexdigest()==r['sha256'];name=Path(r['file']).name;(source/name).write_bytes(raw);shutil.copy2(ROOT/'common/assets'/name.replace('.density.r16f','.json'),source/name.replace('.density.r16f','.json'))
for name in ['cook_distance.py','cook_curves.py']:shutil.copy2(ROOT/'cook'/name,cook/name)
print('Use recorded Python3.14.10 / NumPy2.5.4 / SciPy1.18.1 runtime (not installed by this script):')
print('PYTHON',cook/'cook_distance.py');print('PYTHON',cook/'cook_curves.py')
print('Compare raw output SHAs with builds.json; exact retained-byte restoration does not require these numerical tools.')
