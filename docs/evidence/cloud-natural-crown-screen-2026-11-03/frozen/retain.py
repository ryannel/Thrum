#!/usr/bin/env python3
"""Stage one compact, hash-pinned pre-distance proof package."""
from pathlib import Path
import gzip,hashlib,json,shutil
ROOT=Path(__file__).resolve().parents[2];SCRATCH=Path(__file__).resolve().parent
OUT=ROOT/'docs/evidence/cloud-natural-crown-screen-2026-11-03'
assert (OUT/'README.md').exists() and (OUT/'verify.py').exists()
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1<<20),b''):h.update(b)
 return h.hexdigest()
def copy(src,rel):
 dst=OUT/rel;dst.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(src,dst)
 return dst
files={
 'frozen/COMMISSION.md':SCRATCH/'COMMISSION.md',
 'frozen/PREFLIGHT.md':SCRATCH/'PREFLIGHT.md',
 'frozen/FRAMING-REV2-PREFLIGHT.md':SCRATCH/'FRAMING-REV2-PREFLIGHT.md',
 'frozen/RESULT.md':SCRATCH/'RESULT.md',
 'frozen/selector-first-attempt.py':SCRATCH/'selector-first-attempt.py',
 'frozen/selector.py':SCRATCH/'selector.py',
 'frozen/selector_validate.py':SCRATCH/'selector_validate.py',
 'frozen/native.py':SCRATCH/'native.py',
 'frozen/framing.py':SCRATCH/'framing.py',
 'frozen/view-rev1.py':SCRATCH/'view-rev1.py',
 'frozen/view.py':SCRATCH/'view.py',
 'frozen/boundary.py':SCRATCH/'boundary.py',
 'frozen/retain.py':SCRATCH/'retain.py',
 'inputs/core-probe.py':ROOT/'tmp/cloud-natural-core-probe/core_probe.py',
 'inputs/core-result.json':ROOT/'tmp/cloud-natural-core-probe/result.json',
 'inputs/cloud-03-256.json':ROOT/'tmp/cloud-authored-master/cloud-03-256.json',
 'inputs/body.json':ROOT/'tmp/cloud-component-content-v2/body.json',
 'inputs/crown-b.json':ROOT/'tmp/cloud-component-content-v2/crown-b.json',
 'inputs/LICENSE.txt':ROOT/'tmp/cloud-authored-master/LICENSE.txt',
 'inputs/provenance.json':ROOT/'tmp/cloud-authored-master/provenance.json',
}
for n in [4,5,6]:
 name=f'system-t220000-mode0-c3e0ebee9680337c-eea93588-{n}.json'
 files['inputs/'+name]=ROOT/'docs/evidence/cloud-crown-screen-2026-11-03/inputs'/name
for name in ['selector-first-adverse.json','selector-S.json','selector-O.json','selector.json','native.json','framing-rev2.json','boundary.json','view-adverse.json','view-isolated-front-clipping.json']:
 files['observations/'+name]=SCRATCH/name
for mode in ['isolated','composite220']:
 for view in ['front','side','oblique']:
  for ext in ['png','json']:
   name=f'view-{mode}-{view}.{ext}';files['observations/'+name]=SCRATCH/name
for rel,src in files.items():
 assert src.exists(),src
 copy(src,rel)
raw=SCRATCH/'natural-pre-distance.r16f';expected='6cf3de4e99ea6a5f79d32e1506e9375aadd1aaef6f770615ecd2b8341537ee09'
assert raw.stat().st_size==884736 and sha(raw)==expected
gz=OUT/'selected/natural-pre-distance.r16f.gz';gz.parent.mkdir(parents=True,exist_ok=True)
with gz.open('wb') as f:
 with gzip.GzipFile(filename='',mode='wb',fileobj=f,compresslevel=9,mtime=0)as g:
  with raw.open('rb')as r:shutil.copyfileobj(r,g)
assert gzip.decompress(gz.read_bytes())==raw.read_bytes()
external={
 'raw03_master':ROOT/'tmp/cloud-authored-master/cloud-03-256.density.r16f',
 'old_B_rho':ROOT/'tmp/cloud-component-content-v2/crown-b.density.r16f',
 'old_B_distance':ROOT/'tmp/cloud-component-content-v2/crown-b.distance.r16f',
 'BODY_rho':ROOT/'tmp/cloud-component-content-v2/body.density.r16f',
 'BODY_distance':ROOT/'tmp/cloud-component-content-v2/body.distance.r16f',
 'shared_noise':ROOT/'tmp/cloud-system-proof/assets/noise/channels-32.r16f',
 'inherited_sample_helper':ROOT/'docs/evidence/cloud-crown-screen-2026-11-03/frozen/preflight.py',
}
dep={'schema':1,'scope':'External byte dependencies for exact historical rerun; selected rho restores directly from this package without them','inputs':{name:{'worktree_relative':str(p.relative_to(ROOT)),'bytes':p.stat().st_size,'sha256':sha(p)}for name,p in external.items()},'retained_provenance_packages':['../cloud-authored-masters-2026-11-03/','../cloud-component-proof-2026-11-03/','../cloud-crown-screen-2026-11-03/'],'selected_rho_original_sha256':expected}
(OUT/'dependencies.json').write_text(json.dumps(dep,indent=2)+'\n')
manifest={'schema':1,'scope':'One natural-core pre-distance screen selected only for fresh-distance diagnostic; no art/runtime qualification','selected_rho':{'file':str(gz.relative_to(OUT)),'compression':'gzip deterministic mtime0','uncompressed_bytes':raw.stat().st_size,'sha256':expected},'files':{}}
for p in sorted(OUT.rglob('*')):
 if not p.is_file() or p.name=='manifest.json':continue
 manifest['files'][str(p.relative_to(OUT))]={'bytes':p.stat().st_size,'sha256':sha(p)}
(OUT/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
size=sum(p.stat().st_size for p in OUT.rglob('*')if p.is_file())
assert size<4*2**20,size
print(json.dumps({'package_bytes':size,'files':len(manifest['files']),'selected_rho_sha256':expected,'manifest_sha256':sha(OUT/'manifest.json')}))
