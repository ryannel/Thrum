#!/usr/bin/env python3
"""One pinned pre-distance natural-core rho, native centres only."""
from pathlib import Path
import hashlib,json,math,os,resource,subprocess,time
import numpy as np
R=Path(__file__).resolve().parent;T=R.parent;MASTER=T/'cloud-authored-master/cloud-03-256.density.r16f'
OLD=T/'cloud-component-content-v2/crown-b.json';CORE=T/'cloud-natural-core-probe/result.json'
SHA='42431f9da2d76e253d649cc35802fd1be7b23ddb698e73c6c4247ac3c90a97c9'
EARLY=112*2**20;HARD=160*2**20;PEAK=0;START=time.perf_counter();ORIGIN=np.array([112.,224.,144.]);DIMS=(72,64,96)
def guard(label,reserve=0):
 global PEAK
 rss=int(subprocess.check_output(['ps','-o','rss=','-p',str(os.getpid())]).strip())*1024;PEAK=max(PEAK,rss)
 if rss>=HARD or rss+reserve>=EARLY:raise RuntimeError(f'RSS guard {label}: {rss}+{reserve}, early {EARLY}')
 return rss
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1<<20),b''):h.update(b)
 return h.hexdigest()
def s01(t):
 t=np.clip(t,0,1);return t*t*(3-2*t)
def pathdistance(pts,path):
 # Native centre to finite original-23-vertex polyline, all Float64.
 best=np.full(pts.shape[:-1],np.inf,np.float64)
 for p,q in zip(path[:-1],path[1:]):
  v=q-p;t=np.clip(np.sum((pts-p)*v,axis=-1)/float(np.dot(v,v)),0,1)
  d2=np.sum((pts-p-t[...,None]*v)**2,axis=-1);np.minimum(best,d2,out=best)
 return np.sqrt(best)
def lower(a):
 out=np.full((71,63,95),np.inf,np.float32)
 for z in (0,1):
  for y in (0,1):
   for x in (0,1):np.minimum(out,a[z:z+71,y:y+63,x:x+95],out=out)
 return out
def ballmin(lb,c,r):
 c=np.array(c,float);lo=np.maximum(0,np.floor((c-r-ORIGIN-1)/2).astype(int)-1);hi=np.minimum([94,62,70],np.ceil((c+r-ORIGIN-1)/2).astype(int)+1)
 zz,yy,xx=np.ogrid[lo[2]:hi[2]+1,lo[1]:hi[1]+1,lo[0]:hi[0]+1];dist2=0
 for v,j in ((xx,0),(yy,1),(zz,2)):
  aa=ORIGIN[j]+1+2*v;dist2=dist2+np.maximum(np.maximum(aa-c[j],c[j]-aa-2),0)**2
 sub=lb[lo[2]:hi[2]+1,lo[1]:hi[1]+1,lo[0]:hi[0]+1]
 return float(sub[dist2<=r*r].min()) if np.any(dist2<=r*r) else 0.
def stats(a):
 out={'sum_native':float(a.sum(dtype=np.float64)),'max':float(a.max()),'positive':int(np.count_nonzero(a)),'gt001':int(np.count_nonzero(a>.001)),'gt005':int(np.count_nonzero(a>.005)),'gt041':int(np.count_nonzero(a>.041))}
 hit=np.argwhere(a>0);out['bbox_native_centres_xyz_m']=[(ORIGIN+2*hit.min(axis=0)[::-1]+1).tolist(),(ORIGIN+2*hit.max(axis=0)[::-1]+1).tolist()]if len(hit) else None
 out['face_nonzero']={'X_min':int(np.count_nonzero(a[:,:,0])),'X_max':int(np.count_nonzero(a[:,:,-1])),'Y_min':int(np.count_nonzero(a[:,0,:])),'Y_max':int(np.count_nonzero(a[:,-1,:])),'Z_min':int(np.count_nonzero(a[0])),'Z_max':int(np.count_nonzero(a[-1]))}
 out['adjacent_face_nonzero']={'X_min':int(np.count_nonzero(a[:,:,1])),'X_max':int(np.count_nonzero(a[:,:,-2])),'Y_min':int(np.count_nonzero(a[:,1,:])),'Y_max':int(np.count_nonzero(a[:,-2,:])),'Z_min':int(np.count_nonzero(a[1])),'Z_max':int(np.count_nonzero(a[-2]))}
 return out
def main():
 sel=json.loads((R/'selector.json').read_text());core=json.loads(CORE.read_text());meta=json.loads(OLD.read_text())
 assert sel['source_sha256']==core['source_sha256']==SHA
 for k in ['dS','dO']:
  m=sel[k];p=R/m['file'];assert p.stat().st_size==m['bytes'] and sha(p)==m['sha256']
 assert sha(MASTER)==SHA and meta['source_sha256']==SHA
 ds=np.memmap(R/'dS.f64',dtype='<f8',mode='r',shape=DIMS);do=np.memmap(R/'dO.f64',dtype='<f8',mode='r',shape=DIMS)
 assert np.all(ds+do>0)
 path=np.array([v['point_global_xyz_local_m']for v in meta['path_certificate']['vertices']],dtype=np.float64)
 assert len(path)==23 and np.allclose(path[0],[225,265,197]) and np.allclose(path[-1],[221,295,207])
 guard('before candidate',8*2**20)
 master=np.memmap(MASTER,dtype='<f2',mode='r',shape=(256,256,256));dst=R/'natural-pre-distance.r16f';new=np.memmap(dst,dtype='<f2',mode='w+',shape=DIMS)
 preservedS=preservedRoot=0;preservedRootTotal=0;Oraw=Onew=0;Sraw=0;fmin=1.;fmax=0.;wmin=1.;wmax=0.;mindist=1e9;maxdist=0.
 for z in range(0,72,4):
  guard('native slab',7*2**20)
  zhi=min(z+4,72);zz,yy,xx=np.mgrid[z:zhi,:64,:96]
  pts=np.stack([113+2*xx,225+2*yy,145+2*zz],axis=-1).astype(np.float64)
  raw=np.array(master[72+z:72+zhi,112:176,56:152],dtype=np.float32,copy=True)
  dss=np.asarray(ds[z:zhi]);doo=np.asarray(do[z:zhi]);den=dss+doo
  assert np.all(den>0)
  Wdata=s01(doo/den)*(1-s01((dss-8)/16))
  dg=pathdistance(pts,path);Wroot=1-s01((dg-18)/12)
  H=s01((pts[...,1]-238)/8)
  F=s01((pts[...,0]-116)/12)*(1-s01((pts[...,0]-288)/12))*s01((pts[...,2]-148)/12)*(1-s01((pts[...,2]-272)/12))
  W=np.maximum(Wdata,Wroot)
  assert np.isfinite(W).all() and np.isfinite(H).all() and np.isfinite(F).all()
  assert 0<=W.min()<=W.max()<=1 and 0<=H.min()<=H.max()<=1 and 0<=F.min()<=F.max()<=1
  rho=np.float32(raw*H*F*W).astype('<f2')
  new[z:zhi]=rho
  S=dss==0;O=doo==0;root=dg<=18
  assert np.array_equal(rho[S].view('u2'),raw[S].astype('<f2').view('u2'))
  assert np.array_equal(rho[root].view('u2'),raw[root].astype('<f2').view('u2'))
  assert not np.any(rho.astype(np.float32)>raw) and np.all(np.isfinite(rho))
  preservedS+=int(S.sum());preservedRoot+=int(np.count_nonzero(root));preservedRootTotal+=int(root.sum());Sraw+=float(raw[S].sum(dtype=np.float64));Oraw+=float(raw[O].sum(dtype=np.float64));Onew+=float(rho[O].sum(dtype=np.float64))
  fmin=min(fmin,float(F.min()));fmax=max(fmax,float(F.max()));wmin=min(wmin,float(W.min()));wmax=max(wmax,float(W.max()));mindist=min(mindist,float(dg.min()));maxdist=max(maxdist,float(dg.max()))
  del zz,yy,xx,pts,raw,dss,doo,den,Wdata,Wroot,H,F,W,rho,S,O,root,dg
 new.flush();master._mmap.close();del master
 assert preservedS==19208 and preservedRoot>0
 skirt=(287,245,161);idx=tuple(int((skirt[i]-ORIGIN[i]-1)/2)for i in (2,1,0))
 assert float(new[idx])==0.0
 native=stats(new)
 assert all(v==0 for v in native['face_nonzero'].values())
 assert all(v==0 for v in native['adjacent_face_nonzero'].values())
 # Raw-cell lower screens (pre-distance only), no optical/q certification.
 lb=lower(new.astype(np.float32));guides={'root':[225,265,197],'head':[221,295,207]}
 screens={name:ballmin(lb,c,6)for name,c in guides.items()};screens['path_min_6m']=min(ballmin(lb,c,6)for c in path)
 # Conservative native support with trilinear interpolation-cell halo.
 halo=2*math.sqrt(3);bb=native['bbox_native_centres_xyz_m'];support=[(np.array(bb[0])-halo).tolist(),(np.array(bb[1])+halo).tolist()]
 guard('complete')
 out={'scope':'ONE fixed pre-distance rho source candidate, not runtime/final signed optical d','script_sha256':sha(Path(__file__)),'commission_sha256':sha(R/'COMMISSION.md'),'source_sha256':SHA,'old_metadata_sha256':sha(OLD),'selector_sha256':sha(R/'selector.json'),'core_sha256':sha(CORE),'path_vertices':path.tolist(),'formula':'Float16(Float32(raw*H*F*max(Wdata,Wroot))); exact commission functions, native centres','S_preserved_raw_bytes':preservedS,'Gamma_protected_raw_bytes':preservedRootTotal,'known_skirt_xyz_m':list(skirt),'known_skirt_rho':float(new[idx]),'selector_stats':{'S_raw_sum':Sraw,'O_raw_sum':Oraw,'O_candidate_sum':Onew,'W_min_max':[wmin,wmax],'F_min_max':[fmin,fmax],'Gamma_distance_min_max':[mindist,maxdist]},'native':native,'raw_cell_lower_6m_screens':screens,'conservative_trilinear_support_bbox_xyz_m':support,'candidate':{'file':dst.name,'bytes':dst.stat().st_size,'sha256':sha(dst),'role':'reproducible temporary source-only pre-distance view input, NOT runtime asset'},'wall_s':time.perf_counter()-START,'sampled_peak_rss_bytes':PEAK,'process_highwater_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss}
 (R/'native.json').write_text(json.dumps(out,indent=2)+'\n')
 print(json.dumps({'status':'native pass','sum':native['sum_native'],'positive':native['positive'],'S':preservedS,'skirt':out['known_skirt_rho'],'screens':screens,'peakMiB':PEAK/2**20,'wall_s':out['wall_s']}))
if __name__=='__main__':
 try:main()
 except Exception as e:
  (R/'native-adverse.json').write_text(json.dumps({'error':repr(e),'wall_s':time.perf_counter()-START,'sampled_peak_rss_bytes':PEAK})+'\n');raise
