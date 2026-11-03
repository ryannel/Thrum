#!/usr/bin/env python3
"""Verify compact retained proof and restore the selected pre-distance rho."""
from pathlib import Path
import argparse,gzip,hashlib,json,math,sys
import numpy as np
R=Path(__file__).resolve().parent
EXPECTED='6cf3de4e99ea6a5f79d32e1506e9375aadd1aaef6f770615ecd2b8341537ee09'
def sha(b):return hashlib.sha256(b).hexdigest()
def file_sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1<<20),b''):h.update(b)
 return h.hexdigest()
def lower(a):
 out=np.full((71,63,95),np.inf,np.float32)
 for z in (0,1):
  for y in (0,1):
   for x in (0,1):np.minimum(out,a[z:z+71,y:y+63,x:x+95],out=out)
 return out
def ballmin(lb,c,r=6):
 origin=np.array([112.,224.,144.]);c=np.array(c,float)
 lo=np.maximum(0,np.floor((c-r-origin-1)/2).astype(int)-1);hi=np.minimum([94,62,70],np.ceil((c+r-origin-1)/2).astype(int)+1)
 zz,yy,xx=np.ogrid[lo[2]:hi[2]+1,lo[1]:hi[1]+1,lo[0]:hi[0]+1];dist2=0
 for v,j in ((xx,0),(yy,1),(zz,2)):
  aa=origin[j]+1+2*v;dist2=dist2+np.maximum(np.maximum(aa-c[j],c[j]-aa-2),0)**2
 sub=lb[lo[2]:hi[2]+1,lo[1]:hi[1]+1,lo[0]:hi[0]+1]
 return float(sub[dist2<=r*r].min())
def main():
 parser=argparse.ArgumentParser();parser.add_argument('--verify-only',action='store_true');parser.add_argument('--output',type=Path);args=parser.parse_args()
 if bool(args.verify_only)==bool(args.output):parser.error('choose exactly --verify-only or --output NEW_DIR')
 passed=[];manifest=json.loads((R/'manifest.json').read_text())
 listed=set(manifest['files']);actual={str(p.relative_to(R))for p in R.rglob('*')if p.is_file() and p.name!='manifest.json'}
 assert listed==actual,(sorted(actual-listed),sorted(listed-actual))
 for rel,meta in manifest['files'].items():
  p=R/rel;assert p.stat().st_size==meta['bytes'] and file_sha(p)==meta['sha256'],rel
 passed.append('1 complete package file inventory/hash')
 rho=gzip.decompress((R/'selected/natural-pre-distance.r16f.gz').read_bytes())
 assert len(rho)==884736 and sha(rho)==EXPECTED and manifest['selected_rho']['sha256']==EXPECTED
 a=np.frombuffer(rho,dtype='<f2').reshape((72,64,96))
 passed.append('2 lossless selected R16 payload identity')
 native=json.loads((R/'observations/native.json').read_text())
 for face in [a[0],a[-1],a[:,0,:],a[:,-1,:],a[:,:,0],a[:,:,-1],a[1],a[-2],a[:,1,:],a[:,-2,:],a[:,:,1],a[:,:,-2]]:assert not np.any(face)
 passed.append('3 six faces plus adjacent sample planes zero')
 assert float(a[8,10,87])==native['known_skirt_rho']==0.
 passed.append('4 known lower skirt exact zero')
 stats=native['native'];assert float(a.sum(dtype=np.float64))==stats['sum_native']
 assert int(np.count_nonzero(a))==stats['positive'] and int(np.count_nonzero(a>.001))==stats['gt001'] and int(np.count_nonzero(a>.005))==stats['gt005'] and int(np.count_nonzero(a>.041))==stats['gt041']
 passed.append('5 native amount and positive counts')
 lb=lower(a.astype(np.float32));screens=native['raw_cell_lower_6m_screens']
 assert ballmin(lb,[225,265,197])==screens['root'] and ballmin(lb,[221,295,207])==screens['head']
 assert min(ballmin(lb,p)for p in native['path_vertices'])==screens['path_min_6m']
 passed.append('6 root/head/path pre-distance cell lower')
 frame=json.loads((R/'observations/framing-rev2.json').read_text())
 assert 2200<frame['raw_minimum_common_distance_m']<=frame['rounded_common_distance_m']==2300
 assert frame['image_pixels']==[608,342] and frame['max_query_points_per_tile']<=16384 and frame['new_focus_world_pixel_m']<=frame['old_2km512_world_pixel_m']
 passed.append('7 analytic revised framing and tile/pixel bound')
 for mode in ['isolated','composite220']:
  for view in ['front','side','oblique']:
   d=json.loads((R/f'observations/view-{mode}-{view}.json').read_text());pic=R/'observations'/d['image']['file']
   assert file_sha(pic)==d['image']['sha256'] and pic.stat().st_size==d['image']['bytes']
   assert d['input_sha256']['candidate']==EXPECTED and all(c['peak005_border_pixels']==0 for c in d['comparator'][1:])
   assert d['sampling']['pixels']==([608,342]if mode=='isolated'else[512,288])
   if mode=='isolated':assert abs(np.linalg.norm(np.array(d['camera']['position'])-np.array(d['camera']['target']))-2300)<1e-8
   else:assert d['camera']['distance']==2000
 passed.append('8 six image receipts, camera scopes and zero old/new borders')
 if args.output:
  if args.output.exists():raise RuntimeError(f'refusing existing output {args.output}')
  args.output.mkdir(parents=True)
  p=args.output/'natural-pre-distance.r16f';p.write_bytes(rho)
  assert file_sha(p)==EXPECTED
  receipt={'restored_file':str(p),'bytes':p.stat().st_size,'sha256':EXPECTED,'package_manifest_sha256':file_sha(R/'manifest.json'),'oracles':passed}
  (args.output/'restore.json').write_text(json.dumps(receipt,indent=2)+'\n')
 print(json.dumps({'status':'verified' if args.verify_only else 'restored','oracles':passed,'selected_rho_sha256':EXPECTED,'output':str(args.output)if args.output else None},indent=2))
if __name__=='__main__':main()
