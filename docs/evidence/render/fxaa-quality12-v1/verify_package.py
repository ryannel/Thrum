"""Verify selected source/notice bytes and offline Naga30 validation; no GPU required."""
from pathlib import Path
import hashlib,json,subprocess,sys
root=Path(__file__).resolve().parent
p=json.loads((root/'product.json').read_text())
for name,expected in p['hashes'].items():
 actual=hashlib.sha256((root/name).read_bytes()).hexdigest()
 if actual!=expected:raise SystemExit(f'package hash mismatch: {name}')
naga=Path(sys.argv[1]) if len(sys.argv)>1 else Path.home()/'.cargo/bin/naga'
version=subprocess.check_output([str(naga),'--version'],text=True).strip()
if version!='30.0.1':raise SystemExit(f'expected pinned Naga30.0.1, got {version}')
subprocess.run([str(naga),'--input-kind','wgsl',str(root/'fxaa.wgsl')],check=True)
print('Standalone package source/notice hashes and Naga30 validation passed.')
