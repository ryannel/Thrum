"""Finite native radial and matched-image contour diagnostics; no EDT/mesh."""
import math
import numpy as np
import midshape as M

def stats(v):
 a=np.asarray(v,float)
 return {'count':len(a),'min':float(a.min())if len(a)else None,'mean':float(a.mean())if len(a)else None,'rms':float(np.sqrt(np.mean(a*a)))if len(a)else None,'p95_abs':float(np.quantile(np.abs(a),.95))if len(a)else None,'max':float(a.max())if len(a)else None}

def project(w,cam,W=512,H=288):
 o=np.array(cam['position']);f=np.array(cam['target'])-o;f/=np.linalg.norm(f);r=np.cross(f,[0,1,0]);r/=np.linalg.norm(r);u=np.cross(r,f);v=w-o;depth=float(v@f)
 if depth<=0:return None
 return np.array([(1+float(v@r)/depth/((W/H)*math.tan(math.radians(24))))*W/2,(1-float(v@u)/depth/math.tan(math.radians(24)))*H/2])

def radial_profiles(arrays,parts,cameras,sample,world,origin,span):
 U=np.cross(M.E,[0,0,1]);U/=np.linalg.norm(U);V=np.cross(M.E,U);charts=[];seen=set()
 def add(label,ss,n):
  key=tuple(np.round(np.r_[ss,n],11))
  if key not in seen:charts.append((label,ss,n));seen.add(key)
 for ss in np.linspace(0,M.L,5):
  for angle in np.arange(24)*2*math.pi/24:add('side',ss,-M.K*M.E+M.H*(math.cos(angle)*U+math.sin(angle)*V))
 for label,ss,mus in [('root-cap',0,[-1,-.8,-.5,-M.K]),('head-cap',M.L,[-M.K,0,.5,1])]:
  for mu in mus:
   for angle in np.arange(12)*2*math.pi/12:add(label,ss,mu*M.E+math.sqrt(max(0,1-mu*mu))*(math.cos(angle)*U+math.sin(angle)*V))
 records=[];missing={'old':0,'new':0,'both':0};local=[];offset=[];physical={str(p['ID']):[]for p in parts if p['templateID']=='crown-b'and p['Aseed']>0};pixels={k:[]for k in cameras};primary=next(p for p in parts if p['ID']==5)
 for label,ss,n in charts:
  c=M.ROOT+ss*M.E;rr=33+M.K*ss;vv=np.linspace(0,rr+16,math.ceil((rr+16)/.25)+1);xyz=c+vv[:,None]*n
  crossings=[];counts=[]
  for arr in arrays[1:]:
   values=sample(arr,xyz,origin,span);ix=np.flatnonzero((values[:-1]>=.041)&(values[1:]<.041));counts.append(len(ix))
   if not len(ix):crossings.append(None);continue
   lo=float(vv[ix[-1]]);hi=float(vv[ix[-1]+1])
   for _ in range(24):
    middle=(lo+hi)/2;q=float(sample(arr,(c+middle*n)[None],origin,span)[0])
    if q>=.041:lo=middle
    else:hi=middle
   crossings.append((lo+hi)/2)
  rec={'chart':label,'s':float(ss),'direction':n.tolist(),'crossing_counts':counts,'old_radius_local':crossings[0],'new_radius_local':crossings[1]}
  if crossings[0]is None:missing['old']+=1
  if crossings[1]is None:missing['new']+=1
  if None in crossings:
   if crossings==[None,None]:missing['both']+=1
   records.append(rec);continue
  delta=crossings[1]-crossings[0];local.append(delta);a=c+crossings[0]*n;b=c+crossings[1]*n;op=M.operator(a[None]);off=float(op['offset'][0]);offset.append(off);rec['chiDelta_at_old041_local']=off;rec['signed_delta_local']=delta;rec['signed_world_shift_by_partID']={}
  for p in parts:
   if str(p['ID'])not in physical:continue
   wa,wb=world(p,a),world(p,b);d=math.copysign(float(np.linalg.norm(wb-wa)),delta)if delta else 0.;physical[str(p['ID'])].append(d);rec['signed_world_shift_by_partID'][str(p['ID'])]=d
  rec['ID5_projected_pixel_shift']={}
  for view,cam in cameras.items():
   pa,pb=project(world(primary,a),cam),project(world(primary,b),cam)
   if pa is None or pb is None:rec['ID5_projected_pixel_shift'][view]=None
   else:d=float(np.linalg.norm(pb-pa));pixels[view].append(d);rec['ID5_projected_pixel_shift'][view]=d
  records.append(rec)
 return {'scope':'Finite active-chart unoccluded single-B radial profiles of trilinear pre-distance rho=.041; outermost inside-to-outside crossing sampled at<=.25localm/refined24bisections. Not allsurface distance or guaranteed visible composite contour; narrow crossings can be missed by finite sampling.','profile_count':len(charts),'matched':len(local),'missing':missing,'signed_local_displacement':stats(local),'chiDelta_at_old041_local':stats(offset),'signed_world_displacement_by_partID':{k:stats(v)for k,v in physical.items()},'ID5_projected_pixel_shift':{k:stats(v)for k,v in pixels.items()},'records':records}

def image_contour(old,new,cam):
 def edge(v):
  a=v>=.041;return a&(~np.roll(a,1,0)|~np.roll(a,-1,0)|~np.roll(a,1,1)|~np.roll(a,-1,1))
 a,b=edge(old),edge(new);na,nb=int(a.sum()),int(b.sum());height=old.shape[0];distance=float(np.linalg.norm(np.array(cam['target'])-cam['position']));worldpixel=2*distance*math.tan(math.radians(24))/height
 out={'scope':'Matched-camera projected peak-rho .041 contours; finite pixel-centre nearest distances, not3Dsurface distance. World distances are focal-plane estimates, not depth-correct.','old_edge_pixels':na,'new_edge_pixels':nb,'world_m_per_pixel_at_target':worldpixel,'new_only_threshold_pixels':int(np.count_nonzero((new>=.041)&(old<.041))),'removed_threshold_pixels':int(np.count_nonzero((old>=.041)&(new<.041)))}
 if not na or not nb or max(na,nb)>4096:out['unavailable']='empty contour or4096point bounded-work cap';return out
 A=np.column_stack(np.nonzero(a)).astype(float);B=np.column_stack(np.nonzero(b)).astype(float)
 def nearest(X,Y):
  result=[]
  for i in range(0,len(X),32):
   d=X[i:i+32,None,:]-Y[None];result.extend(np.sqrt(np.sum(d*d,axis=-1).min(axis=1)).tolist())
  return result
 forward=nearest(A,B);back=nearest(B,A);out['old_to_new_pixel_distance']=stats(forward);out['new_to_old_pixel_distance']=stats(back);out['hausdorff_pixels']=max(max(forward),max(back));out['hausdorff_world_m_focal_plane']=out['hausdorff_pixels']*worldpixel
 return out
