"""Data counts only: this does not build or time a GPU atlas."""
from pathlib import Path
import argparse,gzip,hashlib,json
import numpy as np
import scipy
from scipy.ndimage import maximum_filter
ROOT=Path(__file__).resolve().parent
p=argparse.ArgumentParser();p.add_argument('--master-package',type=Path,default=ROOT.parent/'cloud-authored-masters-2026-11-03');args=p.parse_args()
expected={'03':('42431f9da2d76e253d649cc35802fd1be7b23ddb698e73c6c4247ac3c90a97c9',2218,2464),'08':('4c55318ec1664d40e73a0eed0bcfeefd5563a1b65bb60564f80b3ffbd5fad990',2636,2927)}
records=[]
for name,(sha,raw_count,guard_count) in expected.items():
 path=args.master_package/'instrument/assets'/f'cloud-{name}-256.density.r16f.gz'
 data=gzip.decompress(path.read_bytes());assert hashlib.sha256(data).hexdigest()==sha
 values=np.frombuffer(data,dtype='<f2').reshape(256,256,256);assert np.isfinite(values).all() and (values>=0).all()
 positive=values>0
 count=lambda a:int(a.reshape(32,8,32,8,32,8).any(axis=(1,3,5)).sum())
 raw=count(positive);admitted=count(maximum_filter(positive,size=3,mode='constant',cval=0))
 assert (raw,admitted)==(raw_count,guard_count)
 records.append({'master':name,'source_raw_sha256':sha,'nonzero_cores':raw,'guard_admitted_cores':admitted,'core_edge':8,'slot_edge_with_guards':10,'sampled_RG16_bytes_per_voxel':4,'useful_slot_payload_bytes':admitted*10**3*4,'index_bytes':32**3*4})
 del data,values,positive
print(json.dumps({'scope':'Exact positive-support/dilation counts, not rounded physical atlas allocation, sample equivalence or GPU cost','admission':'one-texel Chebyshev dilation of positive density, then any over each8cube core','numpy':np.__version__,'scipy':scipy.__version__,'records':records},indent=2))
