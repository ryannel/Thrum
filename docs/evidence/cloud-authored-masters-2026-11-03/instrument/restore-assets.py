from pathlib import Path
import gzip,json,hashlib
root=Path(__file__).resolve().parent
for compressed in (root/'assets').glob('*.r16f.gz'):
    target=compressed.with_suffix('')
    data=gzip.decompress(compressed.read_bytes())
    target.write_bytes(data)
identity=json.loads((root.parent/'identity.json').read_text())
for name,expected in identity['files'].items():
    path=root/name
    actual=hashlib.sha256(path.read_bytes()).hexdigest()
    if actual!=expected:raise RuntimeError('Exact build mismatch: '+name)
import importlib.util
spec=importlib.util.spec_from_file_location('frozen_serve',root/'serve.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
_,actual=module.payload()
if actual!=identity:raise RuntimeError('Unexpected payload set/build after restore')
print('All exact browser payloads restored and hash-verified:',identity['build'])
