#!/usr/bin/env python3
"""One fresh-process source-only view of raw, frozen old B and one candidate B."""
from pathlib import Path
import argparse,hashlib,importlib.util,json,math,os,resource,subprocess,time
import numpy as np
from PIL import Image,ImageDraw
R=Path(__file__).resolve().parent;T=R.parent;ROOT=Path(__file__).resolve().parents[2]
PFILE=ROOT/'docs/evidence/cloud-crown-screen-2026-11-03/frozen/preflight.py'
spec=importlib.util.spec_from_file_location('old_preflight_helpers',PFILE);P=importlib.util.module_from_spec(spec);spec.loader.exec_module(P)
EARLY=112*2**20;HARD=160*2**20;PEAK=0;START=time.perf_counter()
W,H=512,288;LABELS=('raw crop (clipped diagnostic)','old v2 B (old domain)','natural S/O (pre-distance)')
NEW_ORIGIN=np.array([112.,224.,144.]);NEW_SPAN=np.array([192.,128.,144.]);OLD_ORIGIN=np.array([104.,160.,128.]);OLD_SPAN=np.array([192.,192.,192.])
def guard(label,reserve=0):
 global PEAK
 rss=int(subprocess.check_output(['ps','-o','rss=','-p',str(os.getpid())]).strip())*1024;PEAK=max(PEAK,rss)
 if rss>=HARD or rss+reserve>=EARLY:raise RuntimeError(f'RSS guard {label}: {rss}+{reserve}, early {EARLY}')
 return rss
def sha(path):
 h=hashlib.sha256()
 with path.open('rb') as f:
  for b in iter(lambda:f.read(1<<20),b''):h.update(b)
 return h.hexdigest()
def corners(p,origin,span):
 return P.world(p,np.array([[x,y,z]for x in (origin[0],origin[0]+span[0])for y in (origin[1],origin[1]+span[1])for z in (origin[2],origin[2]+span[2])]))
def main():
 global W,H
 arg=argparse.ArgumentParser();arg.add_argument('view',choices=['front','side','oblique']);arg.add_argument('mode',choices=['isolated','composite220']);args=arg.parse_args()
 frame=json.loads((R/'framing-rev2.json').read_text())
 if args.mode=='isolated':W,H=frame['image_pixels'];rows=frame['rows_per_tile'];assert frame['rounded_common_distance_m']==2300
 else:W,H=512,288;rows=4
 assert rows*W*8<=16384
 native=json.loads((R/'native.json').read_text());sel=json.loads((R/'selector.json').read_text())
 assert sha(R/native['candidate']['file'])==native['candidate']['sha256'] and sha(R/'native.py')==native['script_sha256']
 assert native['source_sha256']==sel['source_sha256']
 inputs=T/'cloud-crown-preflight-v3' # sentinel only; exact committed captures below
 capdir=ROOT/'docs/evidence/cloud-crown-screen-2026-11-03/inputs';num={'front':4,'side':5,'oblique':6}[args.view]
 capfile=capdir/f'system-t220000-mode0-c3e0ebee9680337c-eea93588-{num}.json';cap=json.loads(capfile.read_text())
 parts=cap['source']['parts'];assert cap['source']['sourceTime']==220 and cap['source']['stats']['controlPointUpdates']==0
 chosen=next(p for p in parts if p['ID']==5)
 for p in parts:
  if p['templateID']=='crown-b' and p['Aseed']>0:assert p['ageE']==0
 meta=json.loads((T/'cloud-component-content-v2/body.json').read_text());body={'origin':np.array(meta['domain_origin_global_xyz_local_m'],float),'span':np.array(meta['domain_span_local_m'],float)}
 pins={}
 for key,mkey in [('rho','density'),('d','distance')]:
  p=T/'cloud-component-content-v2'/meta[mkey]['file'];assert sha(p)==meta[mkey]['sha256'];pins[str(p.relative_to(T))]=sha(p)
  body[key]=np.memmap(p,dtype='<f2',mode='r',shape=tuple(meta['dimensions_xyz'][::-1]))
 npth=T/'cloud-system-proof/assets/noise/channels-32.r16f';assert sha(npth)==cap['source']['noiseSha256'];pins[str(npth.relative_to(T))]=sha(npth)
 noise=np.memmap(npth,dtype='<f2',mode='r',shape=(32,32,32))
 oldmeta=json.loads((T/'cloud-component-content-v2/crown-b.json').read_text());oldpath=T/'cloud-component-content-v2'/oldmeta['density']['file'];assert sha(oldpath)==oldmeta['density']['sha256']
 old=np.memmap(oldpath,dtype='<f2',mode='r',shape=(96,96,96))
 master=T/'cloud-authored-master/cloud-03-256.density.r16f';assert sha(master)==native['source_sha256']
 mm=np.memmap(master,dtype='<f2',mode='r',shape=(256,256,256));raw=mm[72:144,112:176,56:152]
 new=np.memmap(R/native['candidate']['file'],dtype='<f2',mode='r',shape=(72,64,96))
 arrays=((raw,NEW_ORIGIN,NEW_SPAN),(old,OLD_ORIGIN,OLD_SPAN),(new,NEW_ORIGIN,NEW_SPAN))
 pins.update({'raw_master':sha(master),'old_B':sha(oldpath),'candidate':native['candidate']['sha256'],'frozen_preflight_helper':sha(PFILE),'capture':sha(capfile)})
 cam=dict(cap['presentation']['camera'])
 if args.mode=='isolated':
  cam['target']=P.world(chosen,(np.array(chosen['root'])+np.array([221,295,207]))/2).tolist()
  yaw=cam['yaw'];pitch=cam['pitch'];cam['position']=(np.array(cam['target'])+frame['rounded_common_distance_m']*np.array([math.sin(yaw)*math.cos(pitch),math.sin(pitch),math.cos(yaw)*math.cos(pitch)])).tolist()
 active=[chosen] if args.mode=='isolated' else parts
 allcorn=[]
 for p in active:
  if p['templateID']=='body':allcorn.extend(corners(p,body['origin'],body['span']))
  else:
   for _,origin,span in arrays:allcorn.extend(corners(p,origin,span))
 blo=np.min(allcorn,axis=0)-1;bhi=np.max(allcorn,axis=0)+1
 eye=np.array(cam['position']);front=np.array(cam['target'])-eye;front/=np.linalg.norm(front);right=np.cross(front,[0,1,0]);right/=np.linalg.norm(right);up=np.cross(right,front)
 sums=np.zeros((3,H,W),np.float64);peaks=np.zeros((3,H,W),np.float32);tile_count=0;point_count=0
 guard('before rays',20*2**20)
 for yy in range(0,H,rows):
  guard(f'rows {yy}',18*2**20)
  py,px=np.mgrid[yy:min(yy+rows,H),:W];rr=py.shape[0];xx=((px+.5)/W*2-1)*(W/H)*math.tan(math.radians(24));y=-((py+.5)/H*2-1)*math.tan(math.radians(24))
  ray=front+xx[...,None]*right+y[...,None]*up;ray/=np.linalg.norm(ray,axis=-1,keepdims=True)
  with np.errstate(divide='ignore',invalid='ignore'):t0=(blo-eye)/ray;t1=(bhi-eye)/ray
  near=np.maximum(np.min(np.stack([t0,t1]),axis=0).max(axis=-1),0);far=np.max(np.stack([t0,t1]),axis=0).min(axis=-1);far=np.maximum(near,far)
  steps=np.ceil((far-near)/8).astype(np.int32);step=(far-near)/np.maximum(steps,1)
  for start in range(0,int(steps.max()),8):
   ids=np.arange(start,start+8);valid=ids[:,None,None]<steps
   tt=near[None]+(ids[:,None,None]+.5)*step[None];w=eye+ray[None]*tt[...,None]
   base=np.zeros(w.shape[:-1],np.float32)
   if args.mode!='isolated':
    for p in active:
     if p['templateID']=='body' and p['Aseed']>0:np.maximum(base,P.query_body(p,w,body,noise),out=base)
   q=np.repeat(base[None],3,axis=0)
   for p in active:
    if p['templateID']!='crown-b' or p['Aseed']<=0:continue
    a=P.material(p,w)
    for j,(arr,origin,span) in enumerate(arrays):np.maximum(q[j],p['Aseed']*P.sample(arr,a,origin,span),out=q[j])
   q*=valid[None]
   sums[:,yy:yy+rr]+=np.sum(q*step[None,None],axis=1)
   peaks[:,yy:yy+rr]=np.maximum(peaks[:,yy:yy+rr],q.max(axis=1))
   tile_count+=1;point_count+=int(np.count_nonzero(valid))
   guard('tile')
  guard('row complete')
 opacity=-np.expm1(-.019*sums)
 stats=[]
 for j in range(3):
  hit=peaks[j]>=.005
  border=int(np.count_nonzero(hit[0])+np.count_nonzero(hit[-1])+np.count_nonzero(hit[:,0])+np.count_nonzero(hit[:,-1]))
  stats.append({'tau_max':float((.019*sums[j]).max()),'opacity_max':float(opacity[j].max()),'opacity_mean':float(opacity[j].mean()),'projected_peak005_pixels':int(hit.sum()),'projected_peak041_pixels':int(np.count_nonzero(peaks[j]>=.041)),'peak005_border_pixels':border})
 if args.mode=='isolated' and (stats[1]['peak005_border_pixels'] or stats[2]['peak005_border_pixels']):
  (R/f'view-{args.mode}-{args.view}-clipping.json').write_text(json.dumps({'view':args.view,'mode':args.mode,'camera':cam,'stats':stats,'source_time':220,'wall_s':time.perf_counter()-START,'peak_bytes':PEAK},indent=2)+'\n')
  raise RuntimeError(f'old or new comparator clipped in image: border counts {[s["peak005_border_pixels"] for s in stats]}')
 guard('before image',10*2**20)
 canvas=Image.new('RGB',(3*W,H+76),(18,24,32));draw=ImageDraw.Draw(canvas)
 for j in range(3):
  gray=np.clip(opacity[j]*235+10,0,255).astype(np.uint8);rgb=np.stack((gray,gray,gray),axis=-1)
  for th,col in ((.005,[40,220,255]),(.041,[255,150,40])):
   hit=peaks[j]>=th;edge=hit&(~np.roll(hit,1,0)|~np.roll(hit,-1,0)|~np.roll(hit,1,1)|~np.roll(hit,-1,1));rgb[edge]=col
  canvas.paste(Image.fromarray(rgb),(j*W,34));draw.text((j*W+5,8),LABELS[j],fill='white')
 draw.text((7,H+40),f'{args.mode} {args.view}; {W}x{H}, 48deg, 8m midpoint; kappa .019; point MAX then Beer; pre-distance B upper bound',fill='white')
 image=R/f'view-{args.mode}-{args.view}.png';canvas.save(image,optimize=True)
 guard('complete')
 out={'scope':'one actual-source220 source-only comparison, not final distance/lighting/art admission','view':args.view,'mode':args.mode,'camera':cam,'source_time':220,'part_ids':[p['ID']for p in active],'source_part_tuple_sha256':hashlib.sha256(json.dumps(active,sort_keys=True).encode()).hexdigest(),'arrays':[{'name':LABELS[j],'origin_xyz_m':origin.tolist(),'span_xyz_m':span.tolist(),'dims_xyz':list(arr.shape[::-1])}for j,(arr,origin,span)in enumerate(arrays)],'body_domain':{'origin':body['origin'].tolist(),'span':body['span'].tolist()},'world_ray_bounds':[blo.tolist(),bhi.tolist()],'sampling':{'pixels':[W,H],'vertical_fov_degrees':48,'max_world_step_m':8,'rows_per_tile':rows,'depth_per_tile':8,'query_points_per_tile':rows*W*8,'tile_count':tile_count,'valid_ray_samples':point_count,'framing_rev2_sha256':sha(R/'framing-rev2.json')},'comparator':stats,'input_sha256':pins,'code_sha256':sha(Path(__file__)),'native_receipt_sha256':sha(R/'native.json'),'selector_receipt_sha256':sha(R/'selector.json'),'image':{'file':image.name,'bytes':image.stat().st_size,'sha256':sha(image)},'wall_s':time.perf_counter()-START,'sampled_peak_rss_bytes':PEAK,'process_highwater_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss}
 (R/f'view-{args.mode}-{args.view}.json').write_text(json.dumps(out,indent=2)+'\n')
 print(json.dumps({'view':args.view,'mode':args.mode,'wall_s':out['wall_s'],'peakMiB':PEAK/2**20,'opacity_max':[s['opacity_max']for s in stats]}))
if __name__=='__main__':
 try:main()
 except Exception as e:
  (R/'view-adverse.json').write_text(json.dumps({'error':repr(e),'wall_s':time.perf_counter()-START,'sampled_peak_rss_bytes':PEAK})+'\n');raise
