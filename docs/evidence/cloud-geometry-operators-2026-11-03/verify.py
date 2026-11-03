#!/usr/bin/env python3
from pathlib import Path
import hashlib,json
root=Path(__file__).resolve().parent
manifest=json.loads((root/'manifest.json').read_text())
for name,record in manifest['files'].items():
 p=root/name
 if p.stat().st_size!=record['bytes'] or hashlib.sha256(p.read_bytes()).hexdigest()!=record['sha256']:raise RuntimeError('Evidence mismatch: '+name)
actual={str(p.relative_to(root)) for p in root.rglob('*') if p.is_file() and '__pycache__' not in p.parts and p.name!='manifest.json'}
if actual!=set(manifest['files']):raise RuntimeError('Unexpected evidence file set')
print('Verified retained evidence files:',len(actual))
