#!/usr/bin/env python3
"""Fresh non-SciPy validation of separately computed selector distances."""
from pathlib import Path
import hashlib,json,resource,time
import numpy as np
R=Path(__file__).resolve().parent;t=time.perf_counter()
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1<<20),b''):h.update(b)
 return h.hexdigest()
a=json.loads((R/'selector-S.json').read_text());b=json.loads((R/'selector-O.json').read_text())
for k in ['source_sha256','commission_sha256','core_result_sha256','S_voxels','O_voxels','component_count','S_bbox_xyz_m','crop_shape_zyx']:
 assert a[k]==b[k],k
for x in [a,b]:
 p=R/x['distance']['file'];assert p.stat().st_size==x['distance']['bytes']==72*64*96*8 and sha(p)==x['distance']['sha256']
ds=np.memmap(R/'dS.f64',dtype='<f8',mode='r',shape=(72,64,96));do=np.memmap(R/'dO.f64',dtype='<f8',mode='r',shape=(72,64,96))
assert np.isfinite(ds).all() and np.isfinite(do).all() and np.all(ds>=0) and np.all(do>=0)
assert int(np.count_nonzero(ds==0))==19208 and int(np.count_nonzero(do==0))==2779
assert not np.any((ds==0)&(do==0)) and float(np.min(ds+do))>0
for rec,field in [(a,ds),(b,do)]:
 for v in rec['independent_oracle']:
  z,y,x=v['index_zyx'];key='d'+rec['stage']+'_m';assert float(field[z,y,x])==v[key]==v['brute_'+key]
out={'scope':'fresh selector validation; no SciPy/image/native candidate','source_sha256':a['source_sha256'],'commission_sha256':a['commission_sha256'],'stage_sha256':{'S':sha(R/'selector-S.json'),'O':sha(R/'selector-O.json')},'S_voxels':19208,'O_voxels':2779,'S_bbox_xyz_m':a['S_bbox_xyz_m'],'dS':a['distance'],'dO':b['distance'],'denominator_min_m':float(np.min(ds+do)),'independent_oracle':a['independent_oracle']+b['independent_oracle'],'elapsed_s':time.perf_counter()-t,'process_highwater_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,'script_sha256':sha(Path(__file__))}
(R/'selector.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({'denominator_min_m':out['denominator_min_m'],'peakMiB':out['process_highwater_bytes']/2**20}))
