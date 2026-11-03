#!/usr/bin/env python3
"""One bounded authoring screen. No SciPy/EDT/runtime field changes."""
from pathlib import Path
import argparse, hashlib, json, math, os, resource, subprocess, time, platform
import numpy as np
from PIL import Image, ImageDraw
R=Path(__file__).resolve().parent; T=R.parent
EARLY=112*2**20; HARD=160*2**20; START=time.perf_counter(); PEAK=0
ORIGIN=np.array([104.,160.,128.]); DIMS=np.array([96]*3); SPAN=np.array([192.]*3)
NAMES=('raw crop diagnostic','old capsule rho','new structured rho')
SHA='42431f9da2d76e253d649cc35802fd1be7b23ddb698e73c6c4247ac3c90a97c9'

def guard(label,reserve=0):
 global PEAK
 rss=int(subprocess.check_output(['ps','-o','rss=','-p',str(os.getpid())]).strip())*1024
 PEAK=max(PEAK,rss)
 if rss>=HARD or rss+reserve>=EARLY: raise RuntimeError(f'RSS guard {label}: {rss} + reserve {reserve}; early={EARLY}, hard={HARD}')
 return rss

def sha(p):
 h=hashlib.sha256()
 with open(p,'rb') as f:
  while b:=f.read(1024*1024): h.update(b)
 return h.hexdigest()

def smooth(x):
 x=np.clip(x,0,1); return x*x*(3-2*x)

def distseg(x,a,b):
 d=b-a; t=np.clip(np.sum((x-a)*d,axis=-1)/np.dot(d,d),0,1)
 return np.linalg.norm(x-a-t[...,None]*d,axis=-1)

def simplify(path,eps=1):
 if len(path)<=2:return path
 d=distseg(path,path[0],path[-1]); i=int(np.argmax(d))
 return np.vstack((simplify(path[:i+1],eps)[:-1],simplify(path[i:],eps))) if d[i]>eps else path[[0,-1]]

def sampled_path(path,step=.125):
 return np.concatenate([a+np.linspace(0,1,max(2,math.ceil(np.linalg.norm(b-a)/step)+1))[:,None]*(b-a) for a,b in zip(path[:-1],path[1:])])

def deviation(a,b):
 x=sampled_path(a); d=np.full(len(x),np.inf)
 for p,q in zip(b[:-1],b[1:]):d=np.minimum(d,distseg(x,p,q))
 return float(d.max())+.0625

def capsule(x,a,b,r0,r1):
 d=b-a; L=float(np.linalg.norm(d)); e=d/L;k=(r1-r0)/L
 v=x-a; u=np.sum(v*e,axis=-1); h2=np.maximum(0,np.sum(v*v,axis=-1)-u*u)
 if abs(k)>=1:return np.maximum(r0-np.linalg.norm(v,axis=-1),r1-np.linalg.norm(x-b,axis=-1))
 s=np.clip(u+k*np.sqrt(h2/(1-k*k)),0,L)
 return r0+k*s-np.sqrt((u-s)**2+h2)

def smax(a,b,k=3):
 h=np.maximum(k-np.abs(a-b),0)/k
 return np.maximum(a,b)+h*h*k*.25

def mask(x,path):
 lengths=np.linalg.norm(np.diff(path,axis=0),axis=1);arc=np.r_[0,np.cumsum(lengths)];rad=12+12*arc/arc[-1]
 d=np.full(x.shape[:-1],-np.inf)
 for a,b,r0,r1 in zip(path[:-1],path[1:],rad[:-1],rad[1:]):d=np.maximum(d,capsule(x,a,b,r0,r1))
 d=smax(d,20-np.linalg.norm(x-[200,318,214],axis=-1))
 return smax(d,16-np.linalg.norm(x-[232,296,224],axis=-1))

def lower(a):
 out=np.full((95,95,95),np.inf,np.float32)
 for z in [0,1]:
  for y in [0,1]:
   for x in [0,1]:np.minimum(out,a[z:z+95,y:y+95,x:x+95],out=out)
 return out

def ballmin(lb,c,r):
 # Interpolation cells whose closed boxes intersect the closed ball. Overestimates set at borders.
 lo=np.maximum(0,np.floor((np.array(c)-r-ORIGIN-1)/2).astype(int)-1)
 hi=np.minimum(94,np.ceil((np.array(c)+r-ORIGIN-1)/2).astype(int)+1)
 z,y,x=np.ogrid[lo[2]:hi[2]+1,lo[1]:hi[1]+1,lo[0]:hi[0]+1]
 ds=0
 for v,j in [(x,0),(y,1),(z,2)]:
  aa=ORIGIN[j]+1+2*v; ds=ds+np.maximum(np.maximum(aa-c[j],c[j]-aa-2),0)**2
 vals=lb[lo[2]:hi[2]+1,lo[1]:hi[1]+1,lo[0]:hi[0]+1]
 return float(vals[ds<=r*r].min()) if np.any(ds<=r*r) else 0.

def native_stats(a):
 count={};ext={}
 for t in [0,.001,.005,.041]:
  bx=[]
  for axis in range(3):
   hit=np.any(a>t,axis=tuple(i for i in range(3) if i!=axis));ids=np.flatnonzero(hit)
   bx.append([int(ids[0]),int(ids[-1])] if len(ids) else None)
  count[str(t)]=int(np.count_nonzero(a>t));ext[str(t)]=bx[::-1]
 return {'sum_native':float(a.sum(dtype=np.float64)),'integral_local_m3':float(a.sum(dtype=np.float64))*8,'max':float(a.max()),'counts':count,'bbox_native_xyz':ext,'faces_nonzero':[int(np.count_nonzero(v)) for v in [a[0],a[-1],a[:,0],a[:,-1],a[:,:,0],a[:,:,-1]]]}

def sample(a,xyz,origin,span):
 # Hardware trilinear contract; avoid work on points outside the declared domain.
 shape=xyz.shape[:-1];valid=np.all((xyz>=origin)&(xyz<=origin+span),axis=-1);out=np.zeros(shape,np.float32)
 if not valid.any():return out
 dims=np.array(a.shape[::-1]);p=(xyz[valid]-origin)/span*dims-.5
 lo=np.floor(p).astype(np.int32);f=p-lo;v=np.zeros(len(p),np.float32)
 for z in [0,1]:
  for y in [0,1]:
   for x in [0,1]:
    ix=np.clip(lo[:,0]+x,0,dims[0]-1);iy=np.clip(lo[:,1]+y,0,dims[1]-1);iz=np.clip(lo[:,2]+z,0,dims[2]-1)
    v+=a[iz,iy,ix].astype(np.float32)*(f[:,0] if x else 1-f[:,0])*(f[:,1] if y else 1-f[:,1])*(f[:,2] if z else 1-f[:,2])
 out[valid]=v;return out

def material(p,w):
 vy=(w[...,1]-p['C'][1])/p['b'];v=(w-np.array(p['C'])-vy[...,None]*np.array(p['K']))/p['b']
 return np.array(p['root'])+v@np.array(p['Qcolumns']).T

def world(p,a):
 v=(np.array(a)-p['root'])@np.linalg.inv(np.array(p['Qcolumns'])).T
 return np.array(p['C'])+p['b']*v+np.array(p['K'])*v[...,1,None]

def query_body(p,w,body,noise):
 a=material(p,w);q=sample(body['rho'],a,body['origin'],body['span']);hit=q>0
 if not hit.any():return q
 aa=a[hit];d=sample(body['d'],aa,body['origin'],body['span'])
 if p['ageE']:
  M=.3+.7*sample(noise,np.clip(aa,0,512),np.zeros(3),np.array([512]*3));d-=p['ageE']*M
 q[hit]*=p['Aseed']*smooth((d+8)/16)
 return q

def render(cameras,parts,arrays,body,noise,isolated=False):
 W,H=256,144;result={};chosen=next(p for p in parts if p['ID']==5)
 # Source domains, held fixed across candidates; no support truncation by the old box.
 corners=[]
 for p in ([chosen] if isolated else parts):
  o=ORIGIN if p['templateID']=='crown-b' else body['origin'];s=SPAN if p['templateID']=='crown-b' else body['span']
  corners.extend(world(p,np.array([[x,y,z]for x in[o[0],o[0]+s[0]]for y in[o[1],o[1]+s[1]]for z in[o[2],o[2]+s[2]]])))
 blo=np.min(corners,axis=0)-1;bhi=np.max(corners,axis=0)+1
 for view,cam0 in cameras.items():
  cam=dict(cam0)
  if isolated:
   cam['target']=world(chosen,(np.array(chosen['root'])+np.array([221,295,207]))/2).tolist();yaw=cam['yaw'];pitch=cam['pitch'];cam['position']=(np.array(cam['target'])+1200*np.array([math.sin(yaw)*math.cos(pitch),math.sin(pitch),math.cos(yaw)*math.cos(pitch)])).tolist()
  o=np.array(cam['position']);f=np.array(cam['target'])-o;f/=np.linalg.norm(f);right=np.cross(f,[0,1,0]);right/=np.linalg.norm(right);up=np.cross(right,f)
  sums=np.zeros((3,H,W),np.float64);peaks=np.zeros((3,H,W),np.float32)
  for yy in range(0,H,8):
   guard(f'{view} rows{yy}',12*2**20)
   py,px=np.mgrid[yy:min(yy+8,H),:W];xx=((px+.5)/W*2-1)*(W/H)*math.tan(math.radians(24));y=-((py+.5)/H*2-1)*math.tan(math.radians(24))
   ray=f+xx[...,None]*right+y[...,None]*up;ray/=np.linalg.norm(ray,axis=-1,keepdims=True)
   with np.errstate(divide='ignore',invalid='ignore'):t0=(blo-o)/ray;t1=(bhi-o)/ray
   near=np.maximum(np.min(np.stack([t0,t1]),axis=0).max(axis=-1),0);far=np.max(np.stack([t0,t1]),axis=0).min(axis=-1);far=np.maximum(near,far)
   steps=np.ceil((far-near)/8).astype(np.int32);step=(far-near)/np.maximum(steps,1)
   for start in range(0,int(steps.max()),8):
    ids=np.arange(start,start+8);valid=ids[:,None,None]<steps
    tt=near[None]+(ids[:,None,None]+.5)*step[None];w=o+ray[None]*tt[...,None]
    base=np.zeros(w.shape[:-1],np.float32)
    if not isolated:
     for p in parts:
      if p['templateID']=='body' and p['Aseed']>0:np.maximum(base,query_body(p,w,body,noise),out=base)
    qs=np.repeat(base[None],3,axis=0)
    for p in ([chosen] if isolated else parts):
     if p['templateID']=='crown-b' and p['Aseed']>0:
      if p['ageE']!=0:raise RuntimeError('B nonzero age loss requires uncommissioned new distance; stop')
      a=material(p,w)
      for j,arr in enumerate(arrays):np.maximum(qs[j],p['Aseed']*sample(arr,a,ORIGIN,SPAN),out=qs[j])
    for j,q in enumerate(qs):
     q*=valid;sums[j,yy:yy+8]+=np.sum(q*step[None],axis=0);peaks[j,yy:yy+8]=np.maximum(peaks[j,yy:yy+8],q.max(axis=0))
    guard('raytile')
   guard('rows complete')
  result[view]=(sums,peaks,cam)
 return result

def contact(name,sums,peaks,labels,notes):
 opacity=-np.expm1(-.019*sums);H,W=opacity.shape[1:];scale=2;canvas=Image.new('RGB',(3*W*scale, H*scale+96),(18,24,32));draw=ImageDraw.Draw(canvas)
 for j in range(3):
  gray=(opacity[j]*235+10).clip(0,255).astype(np.uint8);rgb=np.stack([gray]*3,axis=-1)
  for t,color in [(.005,[40,220,255]),(.041,[255,150,40])]:
   hit=peaks[j]>=t;edge=hit&(~np.roll(hit,1,0)|~np.roll(hit,-1,0)|~np.roll(hit,1,1)|~np.roll(hit,-1,1));rgb[edge]=color
  image=Image.fromarray(rgb).resize((W*scale,H*scale),Image.Resampling.NEAREST);canvas.paste(image,(j*W*scale,40));draw.text((j*W*scale+8,8),labels[j],fill='white')
 draw.text((8,H*scale+46),notes,fill='white');canvas.save(R/(name+'.png'))
 return [{'tau_max':float((.019*sums[j]).max()),'opacity_max':float(opacity[j].max()),'opacity_mean':float(opacity[j].mean()),'pixels_opacity_gt_01':int(np.count_nonzero(opacity[j]>.01)),'projected_peak005_pixels':int(np.count_nonzero(peaks[j]>=.005)),'projected_peak041_pixels':int(np.count_nonzero(peaks[j]>=.041)),'projected_peak005_border_pixels':int(np.count_nonzero(peaks[j][0]>=.005)+np.count_nonzero(peaks[j][-1]>=.005)+np.count_nonzero(peaks[j][:,0]>=.005)+np.count_nonzero(peaks[j][:,-1]>=.005))}for j in range(3)]

def main():
 parser=argparse.ArgumentParser();parser.add_argument('--run',action='store_true');args=parser.parse_args()
 if not args.run:raise SystemExit('Prepared only. Root CPU quiet-window release required, then --run.')
 guard('imports');pins={};inputs=[T/'cloud-authored-master/cloud-03-256.density.r16f',T/'cloud-component-content-v2/crown-b.json',T/'cloud-component-content-v2/body.json']
 for p in inputs:pins[str(p.relative_to(T))]=sha(p)
 if pins['cloud-authored-master/cloud-03-256.density.r16f']!=SHA:raise RuntimeError('Master hash mismatch')
 meta=json.loads(inputs[1].read_text());bmeta=json.loads(inputs[2].read_text());original=np.array([v['point_global_xyz_local_m']for v in meta['path_certificate']['vertices']],float);path=simplify(original);dev=max(deviation(path,original),deviation(original,path))
 if dev>2:raise RuntimeError('Simplification exceeds2m')
 master=np.memmap(inputs[0],dtype='<f2',mode='r',shape=(256,256,256));guard('candidate arrays',7*2**20);arrays=[np.empty((96,96,96),'<f2')for _ in range(3)]
 for z in range(0,96,4):
  guard('native slab',2*2**20);zz,yy,xx=np.mgrid[z:min(z+4,96),:96,:96];xyz=np.stack([105+2*xx,161+2*yy,129+2*zz],axis=-1).astype(float);raw=master[64+z:64+min(z+4,96),80:176,52:148].astype(np.float32)
  arrays[0][z:z+4]=raw;old=capsule(xyz,np.array([225,265,197]),np.array([221,295,207]),33,40)
  oldrho=raw.copy();oldrho*=smooth((old+8)/16);newrho=raw.copy();newrho*=smooth((mask(xyz,path)+8)/16);arrays[1][z:z+4]=oldrho.astype('<f2');arrays[2][z:z+4]=newrho.astype('<f2')
 del xyz,raw,old,oldrho,newrho,zz,yy,xx
 oldpath=T/'cloud-component-content-v2'/meta['density']['file'];old=np.memmap(oldpath,dtype='<f2',mode='r',shape=(96,96,96))
 pins[str(oldpath.relative_to(T))]=sha(oldpath)
 if pins[str(oldpath.relative_to(T))]!=meta['density']['sha256']:raise RuntimeError('Frozen old rho SHA mismatch')
 if not np.array_equal(arrays[1],old):raise RuntimeError('Old-mask reconstruction differs from frozen density')
 native=[];screens=[];guides=[('root',[225,265,197]),('primary',[221,295,207]),('upper',[200,318,214]),('depth',[232,296,224])]
 for arr in arrays:
  native.append(native_stats(arr));guard('cell lower',4*2**20);lb=lower(arr);screen={}
  for name,c in guides:screen[name]={'raw_rho_cell_lower_on6m_ball':ballmin(lb,c,6),'expanded_support001_min':ballmin(lb,c,6+8+2*math.sqrt(3)+.13)}
  screen['path_min6m_lower']=min(ballmin(lb,c,6)for c in original);screens.append(screen);del lb
 for view,axis in [('native-front',0),('native-side',2)]:
  sums=np.stack([a.sum(axis=axis,dtype=np.float64)*12 for a in arrays]);peaks=np.stack([a.max(axis=axis)for a in arrays])
  # SummingZ gives[Y,X]; summingX gives[Z,Y], transpose to[Y,Z].
  if axis==2:sums=sums.transpose(0,2,1);peaks=peaks.transpose(0,2,1)
  sums=sums[:,::-1];peaks=peaks[:,::-1];contact(view,sums,peaks,NAMES,'Native axis rays; local2m, world scale6; kappa .019. Cyan peak .005 / orange .041. B rho UPPER BOUND.')
 capbase=T/'cloud-system-proof/captures';caps={view:json.loads((capbase/f'system-t220000-mode0-c3e0ebee9680337c-eea93588-{i}.json').read_text())for view,i in [('front',4),('side',5),('oblique',6)]};cap=caps['front'];parts=cap['source']['parts']
 if cap['source']['stats']['controlPointUpdates']!=0 or cap['source']['sourceTime']!=220:raise RuntimeError('Not the zero-weather220 fixture')
 for p in parts:
  if p['templateID']=='crown-b' and p['Aseed']>0 and p['ageE']!=0:raise RuntimeError('Unsupported B age exposure')
 body={'origin':np.array(bmeta['domain_origin_global_xyz_local_m']), 'span':np.array(bmeta['domain_span_local_m'])}
 for key,mkey in [('rho','density'),('d','distance')]:
  p=T/'cloud-component-content-v2'/bmeta[mkey]['file'];pins[str(p.relative_to(T))]=sha(p)
  if pins[str(p.relative_to(T))]!=bmeta[mkey]['sha256']:raise RuntimeError('BODY dependency mismatch')
  body[key]=np.memmap(p,dtype='<f2',mode='r',shape=tuple(bmeta['dimensions_xyz'][::-1]))
 npth=T/'cloud-system-proof/assets/noise/channels-32.r16f'
 if not npth.exists():raise RuntimeError('Pinned noise path missing')
 pins[str(npth.relative_to(T))]=sha(npth)
 if pins[str(npth.relative_to(T))]!=cap['source']['noiseSha256']:raise RuntimeError('Noise mismatch')
 noise=np.memmap(npth,dtype='<f2',mode='r',shape=(32,32,32));print(json.dumps({'stage':'before-perspective-projections','rss_bytes':guard('before projections'),'peak_bytes':PEAK,'new_native':native[2],'new_screens':screens[2]}),flush=True);cams={k:v['presentation']['camera']for k,v in caps.items()};projections={}
 for i in [4,5,6]:
  pp=capbase/f'system-t220000-mode0-c3e0ebee9680337c-eea93588-{i}.json';pins[str(pp.relative_to(T))]=sha(pp)
 for isolated in [True,False]:
  mode='isolated'if isolated else'composite220';results=render(cams,parts,arrays,body,noise,isolated)
  for view,(sums,peaks,cam)in results.items():
   stats=contact(mode+'-'+view,sums,peaks,NAMES,f'{mode} {view};256x144,48deg,8m midpoint; kappa .019; pointMAX then Beer; B pre-distance UPPER BOUND.')
   projections[mode+'-'+view]={'camera':cam,'candidates':stats}
 guard('complete');out={'scope':'one pinned pre-EDT masked-rho geometry screen; no lighting/art/runtime qualification','tool':{'script_sha256':sha(Path(__file__)),'test_sha256':sha(R/'test_preflight.py'),'preflight_sha256':sha(R/'PREFLIGHT.md'),'python':platform.python_version(),'numpy':np.__version__,'platform':platform.platform()},'sampling':{'image':[256,144],'vertical_fov_degrees':48,'world_step_max_m':8,'isolated_camera_distance':1200,'isolated_target':'midpoint of actual root and primary head, ID5 pose','composite_camera':'exact recorded220 front/side/oblique'},'input_sha256':pins,'source_capture':str((capbase/'system-t220000-mode0-c3e0ebee9680337c-eea93588-4.json').relative_to(T)),'source_capture_sha256':sha(capbase/'system-t220000-mode0-c3e0ebee9680337c-eea93588-4.json'),'path':path.tolist(),'path_original_vertices':len(original),'path_simplified_segments':len(path)-1,'path_symmetric_deviation_guarded_local_m':dev,'native':native,'screens':screens,'screen_scope':'raw masked-rho only; expanded support sufficient/failure inconclusive for actual post-EDT6m balls','projections':projections,'wall_seconds':time.perf_counter()-START,'sampled_peak_rss_bytes':PEAK,'process_highwater_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,'native_candidate_array_bytes':sum(a.nbytes for a in arrays),'no_native_payload_written':True}
 (R/'results.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'status':'complete','seconds':out['wall_seconds'],'peakMiB':PEAK/2**20,'pathSegments':len(path)-1}))

if __name__=='__main__':
 try:main()
 except Exception as e:
  (R/'adverse.json').write_text(json.dumps({'error':str(e),'seconds':time.perf_counter()-START,'sampled_peak_rss_bytes':PEAK},indent=2)+'\n');raise
