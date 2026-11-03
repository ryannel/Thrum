from pathlib import Path
import sys,json,hashlib,time,resource,math,os
import numpy as np
R=Path(__file__).resolve().parent;T=R.parent;O=np.array([112.,224.,144.]);SH=(72,64,96);CS=(71,63,95);ROOT=np.array([225.,265.,197.]);QMIN=.041;GUARD=.05
INPUT=T/'cloud-natural-crown-v1/natural-pre-distance.r16f';MASTER=T/'cloud-authored-master/cloud-03-256.density.r16f';SEL=T/'cloud-natural-crown-v1/selector.json';DS=T/'cloud-natural-crown-v1/dS.f64';START=time.perf_counter()
def sha(p):
 h=hashlib.sha256()
 with Path(p).open('rb')as f:
  for b in iter(lambda:f.read(1<<20),b''):h.update(b)
 return h.hexdigest()
def write(n,x):
 x.update(recipe_sha256=sha(__file__),rho_sha256=sha(INPUT),commission_sha256=sha(R/'COMMISSION.md'),elapsed_s=time.perf_counter()-START,process_highwater_bytes=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss)
 (R/n).write_text(json.dumps(x,indent=2)+'\n');print(n,json.dumps(x)[:400],flush=True)
def rho():
 assert sha(INPUT)=='6cf3de4e99ea6a5f79d32e1506e9375aadd1aaef6f770615ecd2b8341537ee09'
 return np.memmap(INPUT,dtype='<f2',mode='r',shape=SH)
def smooth(v):
 u=np.clip((v+8)/16,0,1);return u*u*(3-2*u)
def mincorners(a):
 out=np.full(CS,np.inf,np.float64)
 for z in(0,1):
  for y in(0,1):
   for x in(0,1):np.minimum(out,a[z:z+71,y:y+63,x:x+95],out=out)
 return out
def fields():
 d=np.memmap(R/'distance.r16f',dtype='<f2',mode='r',shape=SH);lb=np.memmap(R/'cell-lower.f64',dtype='<f8',mode='r',shape=CS);return d,lb
stage=sys.argv[1]
if stage in['inside','outside']:
 from scipy.ndimage import distance_transform_edt
 import scipy
 a=np.asarray(rho(),np.float32);inside=a>=np.float32(.001);mask=inside if stage=='inside'else~inside
 out=distance_transform_edt(mask,sampling=(2.,2.,2.));out.astype('<f8',copy=False).tofile(R/(stage+'.f64'));out.astype('<f4').tofile(R/(stage+'.f32'))
 write(stage+'.json',{'stage':stage,'class':'decodedFloat32rho>=Float32(.001)','inside_count':int(inside.sum()),'f64_sha256':sha(R/(stage+'.f64')),'f32_sha256':sha(R/(stage+'.f32')),'range':[float(out.min()),float(out.max())],'python':sys.version,'numpy':np.__version__,'scipy':scipy.__version__})
elif stage=='merge':
 a=np.asarray(rho(),np.float32);inside=a>=np.float32(.001);d=np.memmap(R/'distance.r16f',dtype='<f2',mode='w+',shape=SH);mismatch=0;maxerr=0.;sumerr=0.
 for name in['inside','outside']:
  j=json.loads((R/(name+'.json')).read_text());assert j['rho_sha256']==sha(INPUT)
  assert j['f64_sha256']==sha(R/(name+'.f64'))and j['f32_sha256']==sha(R/(name+'.f32'))
 i64=np.memmap(R/'inside.f64',dtype='<f8',mode='r',shape=SH);o64=np.memmap(R/'outside.f64',dtype='<f8',mode='r',shape=SH);i32=np.memmap(R/'inside.f32',dtype='<f4',mode='r',shape=SH);o32=np.memmap(R/'outside.f32',dtype='<f4',mode='r',shape=SH)
 for z in range(SH[0]):
  exact=i64[z]-o64[z];staged=i32[z]-o32[z];direct=exact.astype('<f2');half=staged.astype('<f2');mismatch+=int(np.count_nonzero(direct.view('u2')!=half.view('u2')));d[z]=half;err=np.abs(half.astype(np.float64)-exact);maxerr=max(maxerr,float(err.max()));sumerr+=float(err.sum())
 d.flush();dd=np.asarray(d,np.float64);lb=mincorners(a)*smooth(mincorners(dd))-2e-12;lb.astype('<f8').tofile(R/'cell-lower.f64');q=a.astype(np.float64)*smooth(dd);pos=q>0;native=a>0;positivebbox=[]
 for arr in[native,pos]:
  bounds=[]
  for axis in[2,1,0]:
   ix=np.flatnonzero(np.any(arr,axis=tuple(k for k in range(3)if k!=axis)));bounds.append([int(ix[0]),int(ix[-1])])
  positivebbox.append(bounds)
 support=[]
 for bb in positivebbox:support.append([[float(O[k]+(bb[k][0]-.5)*2)for k in range(3)],[float(O[k]+(bb[k][1]+1.5)*2)for k in range(3)]])
 possible=(-mincorners(-a)>0)&(-mincorners(-dd)>-8);cb=[]
 for axis in[2,1,0]:
  ix=np.flatnonzero(np.any(possible,axis=tuple(k for k in range(3)if k!=axis)));cb.append([int(ix[0]),int(ix[-1])])
 support[1]=[[float(O[k]+1+2*cb[k][0])for k in range(3)],[float(O[k]+3+2*cb[k][1])for k in range(3)]]
 endpoint=math.ceil((float(dd.max())+8+.05)/.3*1024)/1024;assert not np.any(a*smooth(dd-.3*endpoint)>0)
 collar=min(min(np.flatnonzero(np.any(native,axis=tuple(j for j in range(3)if j!=k)))[0],SH[k]-1-np.flatnonzero(np.any(native,axis=tuple(j for j in range(3)if j!=k)))[-1])for k in range(3))
 write('field.json',{'rounding_contract':'F64EDT sign ->F32scratch ->F32disjoint signed merge ->R16','direct_f64_to_half_bit_differences':mismatch,'distance_half_error_max_local_m':maxerr,'distance_half_error_mean_local_m':sumerr/a.size,'distance_range_local_m':[float(dd.min()),float(dd.max())],'raw_integral_local_m3':float(a.sum(dtype=np.float64)*8),'reference_integral_local_m3':float(q.sum()*8),'reference_attenuation_fraction':1-float(q.sum()/a.sum(dtype=np.float64)),'rho_positive_native_count':int(native.sum()),'q_positive_native_count':int(pos.sum()),'strict_positive_trilinear_bounds_rho_q':support,'q_bound_method':'unionofclosedcellswithmaxrho>0ANDmaxd>-8; notnativeqpositivebbox','exact_zero_collar_native_voxels':int(collar),'endpoint_local_m':endpoint,'endpoint_all_native_zero_Mmin':True,'distance_sha256':sha(R/'distance.r16f'),'cell_lower_sha256':sha(R/'cell-lower.f64')})
elif stage=='radius':
 from scipy.ndimage import distance_transform_edt
 d,lb=fields();bad=lb<QMIN+1e-6;badvertex=np.zeros(SH,bool)
 for z in(0,1):
  for y in(0,1):
   for x in(0,1):badvertex[z:z+71,y:y+63,x:x+95]|=bad
 badvertex[0]=True;badvertex[-1]=True;badvertex[:,0]=True;badvertex[:,-1]=True;badvertex[:,:,0]=True;badvertex[:,:,-1]=True
 dist=distance_transform_edt(~badvertex,sampling=(2.,2.,2.));safe=np.maximum(0,dist-math.sqrt(3)-.0501).astype('<f4');safe.tofile(R/'safe-radius.f32')
 write('radius.json',{'definition':'EDT distance to native vertices ofbadclosedcells minus sqrt3 and .0501; safe lower localradius','candidate_6m_centres':int(np.count_nonzero(safe>=6)),'safe_radius_sha256':sha(R/'safe-radius.f32')})
elif stage=='certs':
 d,lb=fields();a=np.asarray(rho(),np.float32);bad=lb<QMIN+1e-6;safe=np.memmap(R/'safe-radius.f32',dtype='<f4',mode='r',shape=SH)
 def badDistance(c):
  best=min(float(np.min(c-(O+1))),float(np.min(O+np.array(SH[::-1])*2-1-c)))
  dx=np.maximum(np.maximum(O[0]+1+2*np.arange(95)-c[0],c[0]-(O[0]+3+2*np.arange(95))),0)
  dy=np.maximum(np.maximum(O[1]+1+2*np.arange(63)-c[1],c[1]-(O[1]+3+2*np.arange(63))),0)
  xy=dy[:,None]**2+dx[None,:]**2
  for z in range(71):
   dz=max(O[2]+1+2*z-c[2],c[2]-(O[2]+3+2*z),0);v=np.where(bad[z],xy+dz*dz,np.inf);best=min(best,math.sqrt(float(v.min())))
  return best
 def ballCheck(c,r):
  lo=np.maximum(0,np.floor((c-r-O-3)/2).astype(int));hi=np.minimum(np.array(CS[::-1])-1,np.ceil((c+r-O-1)/2).astype(int));minimum=1.;cells=0
  assert np.all(c-r>O+1)and np.all(c+r<O+np.array(SH[::-1])*2-1)
  for z in range(lo[2],hi[2]+1):
   for y in range(lo[1],hi[1]+1):
    for x in range(lo[0],hi[0]+1):
     v=O+1+2*np.array([x,y,z]);delta=np.maximum(np.maximum(v-c,c-v-2),0)
     if float(delta@delta)<=r*r:minimum=min(minimum,float(lb[z,y,x]));cells+=1
  assert cells and minimum>=QMIN+1e-6;return {'intersected_cells':cells,'minimum_cell_lower':minimum}
 rr=badDistance(ROOT)-GUARD;rootcheck=ballCheck(ROOT,rr);assert rr>=6
 si=json.loads(SEL.read_text());assert sha(DS)==si['dS']['sha256'];S=np.memmap(DS,dtype='<f8',mode='r',shape=SH)==0;assert int(S.sum())==19208
 assert sha(MASTER)=='42431f9da2d76e253d649cc35802fd1be7b23ddb698e73c6c4247ac3c90a97c9';master=np.memmap(MASTER,dtype='<f2',mode='r',shape=(256,256,256));raw=master[72:144,112:176,56:152].astype(np.float64);weights=np.where(S,raw,0);total=float(weights.sum());centroid=np.array([float((weights.sum(axis=(0,1))*(O[0]+1+2*np.arange(96))).sum()),float((weights.sum(axis=(0,2))*(O[1]+1+2*np.arange(64))).sum()),float((weights.sum(axis=(1,2))*(O[2]+1+2*np.arange(72))).sum())])/total
 good=np.asarray(safe>=6);rootijk=np.rint((ROOT-O-1)/2).astype(int);ridx=int(np.ravel_multi_index(tuple(rootijk[::-1]),SH));good.flat[ridx]=True
 pred=np.full(a.size,-2,np.int32);queue=np.empty(a.size,np.int32);pred[ridx]=-1;queue[0]=ridx;head=0;tail=1
 while head<tail:
  i=int(queue[head]);head+=1;z,y,x=np.unravel_index(i,SH)
  for zz,yy,xx in[(z,y,x-1),(z,y,x+1),(z,y-1,x),(z,y+1,x),(z-1,y,x),(z+1,y,x)]:
   if 0<=zz<72 and 0<=yy<64 and 0<=xx<96:
    j=(zz*64+yy)*96+xx
    if good[zz,yy,xx]and pred[j]==-2:pred[j]=i;queue[tail]=j;tail+=1
 np.asarray(pred!=-2,dtype='u1').reshape(SH).tofile(R/'reachable.u8');pred.astype('<i4').tofile(R/'predecessor.i32');
 ids=np.flatnonzero(S.ravel()&(pred!=-2));assert len(ids)>0;zyx=np.array(np.unravel_index(ids,SH)).T;coords=O+1+2*zyx[:,::-1];dsq=((coords-centroid)**2).sum(axis=1);order=np.lexsort((coords[:,2],coords[:,1],coords[:,0],dsq));principal=int(order[0]);selected=[];witnesses=[];rejected=[]
 def chain(j):
  path=[]
  while j>=0:path.append(j);j=int(pred[j])
  path=path[::-1];pts=np.array([O+1+2*np.array(np.unravel_index(i,SH))[::-1]for i in path]);out=[0];k=0
  while k<len(pts)-1:
   limit=rr+6-1 if k==0 else 11.;eligible=np.flatnonzero(((pts[k+1:]-pts[k])**2).sum(axis=1)<=limit*limit);assert len(eligible);k=k+1+int(eligible[-1]);out.append(k)
  return pts[out]
 remaining=set(range(len(ids)))
 for attempt in range(min(len(ids),128)):
  if not selected:k=principal
  else:
   candidates=sorted(remaining);spread=np.min(np.stack([((coords[candidates]-coords[s])**2).sum(axis=1)for s in selected]),axis=0);ix=np.lexsort((coords[candidates,2],coords[candidates,1],coords[candidates,0],-spread));k=candidates[int(ix[0])]
  remaining.discard(k);ch=chain(int(ids[k]));
  if len(ch)>16:
   rejected.append({'center':coords[k].tolist(),'chain_balls':len(ch)})
   if not selected:raise RuntimeError('Principal chain exceeds16')
   if not remaining:break
   continue
  radius=badDistance(coords[k])-GUARD;check=ballCheck(coords[k],radius);assert radius>=6
  balls=[]
  for n,c in enumerate(ch):
   rad=rr if n==0 else(radius if n==len(ch)-1 else 6.);cert=ballCheck(c,rad);balls.append({'centerLocal':c.tolist(),'radiusLocal':rad,'qMin':QMIN,'cellCheck':cert})
  gaps=[float(np.linalg.norm(ch[n+1]-ch[n]))for n in range(len(ch)-1)];assert all(v<balls[n]['radiusLocal']+balls[n+1]['radiusLocal']for n,v in enumerate(gaps))
  witnesses.append({'id':'natural-head-'+str(len(selected)),'centerLocal':coords[k].tolist(),'radiusLocal':radius,'qMin':QMIN,'rootChain':balls,'chain_max_separation_local_m':max(gaps,default=0),'cellCheck':check});selected.append(k)
  if len(selected)==4 or not remaining:break
 Y=(coords[selected[0]]-ROOT);Y/=np.linalg.norm(Y);X=np.cross(Y,[0.,0.,1.]);assert np.linalg.norm(X)>.01;X/=np.linalg.norm(X);Z=np.cross(X,Y);frame=np.stack([X,Y,Z],axis=1);assert np.max(np.abs(frame.T@frame-np.eye(3)))<1e-12 and np.linalg.det(frame)>.999999999
 oldY=np.array([221.,295.,207.])-ROOT;oldY/=np.linalg.norm(oldY)
 write('certificates.json',{'status':'HASHED UNQUALIFIED optical diagnostic','rootBall':{'centerLocal':ROOT.tolist(),'radiusLocal':rr,'qMin':QMIN,'cellCheck':rootcheck},'S_voxels':int(S.sum()),'original_S_raw_rho_weighted_centroid':centroid.tolist(),'principal_selection':'nearestreachable certified6m S nativecentre; squaredDistance/XYZ stabletie','reachable_certified_S_centres':len(ids),'headWitnesses':witnesses,'rejected_spread_candidates':rejected,'root_to_crown_frame':{'root_to_crown_orthoframe_columns':frame.T.tolist(),'root_head_length_local_m':float(np.linalg.norm(coords[selected[0]]-ROOT)),'old_axis_change_degrees':math.degrees(math.acos(float(np.clip(oldY@Y,-1,1))))},'selector_sha256':sha(SEL),'dS_sha256':sha(DS),'original_master_sha256':sha(MASTER),'cell_lower_sha256':sha(R/'cell-lower.f64'),'radius_sha256':sha(R/'safe-radius.f32'),'chain_mask_definition':'6-neighbour native-centre BFS of conservative>=6m spheres then deterministicoverlap compression; everyretainedsphereindependentlycell-intersectionchecked'})
else:raise SystemExit(stage)
