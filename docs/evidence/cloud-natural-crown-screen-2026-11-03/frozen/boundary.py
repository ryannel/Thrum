#!/usr/bin/env python3
"""Map candidate selection surfaces to the actual source220 BODY query."""
from pathlib import Path
import hashlib,importlib.util,json,os,resource,subprocess,time
import numpy as np
R=Path(__file__).resolve().parent;T=R.parent;ROOT=Path(__file__).resolve().parents[2]
PFILE=ROOT/'docs/evidence/cloud-crown-screen-2026-11-03/frozen/preflight.py'
spec=importlib.util.spec_from_file_location('old_preflight_helpers',PFILE);P=importlib.util.module_from_spec(spec);spec.loader.exec_module(P)
EARLY=112*2**20;HARD=160*2**20;PEAK=0;START=time.perf_counter()
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
def surface(pos):
 interior=pos.copy();interior[1:,:,:]&=pos[:-1,:,:];interior[:-1,:,:]&=pos[1:,:,:]
 interior[:,1:,:]&=pos[:,:-1,:];interior[:,:-1,:]&=pos[:,1:,:]
 interior[:,:,1:]&=pos[:,:,:-1];interior[:,:,:-1]&=pos[:,:,1:]
 return pos&~interior
def summarize(mask,part,bodies,body,noise,rho):
 inds=np.argwhere(mask);n=len(inds)
 if not n:return {'n':0}
 domain_out=body_zero=body005=body_less=0;lo=np.array([np.inf]*3);hi=-lo;example=[]
 for start in range(0,n,4096):
  chunk=inds[start:start+4096];a=np.stack([113+2*chunk[:,2],225+2*chunk[:,1],145+2*chunk[:,0]],axis=-1).astype(np.float64)
  w=P.world(part,a);bq=np.zeros(len(a),np.float32);inside=np.zeros(len(a),bool)
  for bp in bodies:
   ab=P.material(bp,w);inside|=np.all((ab>=body['origin'])&(ab<=body['origin']+body['span']),axis=-1)
   np.maximum(bq,P.query_body(bp,w,body,noise),out=bq)
  q=np.asarray(rho[chunk[:,0],chunk[:,1],chunk[:,2]],dtype=np.float32)*part['Aseed']
  domain_out+=int(np.count_nonzero(~inside));body_zero+=int(np.count_nonzero(bq==0));body005+=int(np.count_nonzero(bq<=.005));body_less+=int(np.count_nonzero(bq<q))
  ex=(bq==0)&(q>.005)
  if ex.any():lo=np.minimum(lo,w[ex].min(axis=0));hi=np.maximum(hi,w[ex].max(axis=0))
  for i in np.flatnonzero(ex)[:max(0,4-len(example))]:example.append({'local_xyz_m':a[i].tolist(),'world_xyz_m':w[i].tolist(),'crown_rho':float(q[i]),'BODY_q':float(bq[i])})
  guard('mapped chunk',8*2**20)
 return {'n':n,'outside_all_BODY_query_domains':domain_out,'BODY_q_exact_zero':body_zero,'BODY_q_le005':body005,'BODY_q_lt_crown_preDist':body_less,'uncovered_qgt005_world_bbox_xyz_m':[lo.tolist(),hi.tolist()]if np.isfinite(lo).all()else None,'uncovered_examples':example}
def main():
 native=json.loads((R/'native.json').read_text());p=R/native['candidate']['file'];assert sha(p)==native['candidate']['sha256']
 rho=np.memmap(p,dtype='<f2',mode='r',shape=(72,64,96));pos=np.asarray(rho>0);pos005=np.asarray(rho>.005)
 rawpath=T/'cloud-authored-master/cloud-03-256.density.r16f';assert sha(rawpath)==native['source_sha256'];master=np.memmap(rawpath,dtype='<f2',mode='r',shape=(256,256,256));raw=master[72:144,112:176,56:152]
 cut=np.asarray((rho>0)&(rho<raw));master._mmap.close();del master,raw
 x=113+2*np.arange(96);y=225+2*np.arange(64);z=145+2*np.arange(72)
 Fpartial=(x[None,None,:]<128)|(x[None,None,:]>288)|(z[:,None,None]<160)|(z[:,None,None]>272)
 masks={'all_positive':pos,'positive_surface':surface(pos),'positive_cut_surface':surface(pos)&cut,'gt005_surface':surface(pos005),'gt041_surface':surface(np.asarray(rho>.041)),'H_partial_positive':pos&(y[None,:,None]<246),'F_partial_positive':pos&Fpartial,'lower_positive_Y_lt300':pos&(y[None,:,None]<300)}
 capfile=ROOT/'docs/evidence/cloud-crown-screen-2026-11-03/inputs/system-t220000-mode0-c3e0ebee9680337c-eea93588-4.json';cap=json.loads(capfile.read_text());parts=cap['source']['parts']
 crowns=[x for x in parts if x['templateID']=='crown-b' and x['Aseed']>0];assert sorted(x['b']for x in crowns)==[5,6] and all(x['ageE']==0 for x in crowns)
 bodies=[x for x in parts if x['templateID']=='body' and x['Aseed']>0]
 bm=json.loads((T/'cloud-component-content-v2/body.json').read_text());body={'origin':np.array(bm['domain_origin_global_xyz_local_m']), 'span':np.array(bm['domain_span_local_m'])}
 for key,mkey in [('rho','density'),('d','distance')]:
  f=T/'cloud-component-content-v2'/bm[mkey]['file'];assert sha(f)==bm[mkey]['sha256'];body[key]=np.memmap(f,dtype='<f2',mode='r',shape=tuple(bm['dimensions_xyz'][::-1]))
 npth=T/'cloud-system-proof/assets/noise/channels-32.r16f';assert sha(npth)==cap['source']['noiseSha256'];noise=np.memmap(npth,dtype='<f2',mode='r',shape=(32,32,32))
 guard('before samples',20*2**20)
 results={}
 for part in crowns:
  results[str(part['ID'])]={'b_world_per_local_m':part['b'],'C_world_m':part['C'],'masks':{name:summarize(mask,part,bodies,body,noise,rho)for name,mask in masks.items()}}
  guard('after part')
 out={'scope':'pre-distance candidate native-site overlap with actual BODY source220; finite sampled site counts, no visibility/opacity occlusion proof','candidate_sha256':native['candidate']['sha256'],'source_capture_sha256':sha(capfile),'code_sha256':sha(Path(__file__)),'BODY_query':'max over source220 BODY parts using frozen shared rho/d/age/material noise, trilinear source helper','mask_definitions':{'positive_surface':'positive native site with >=1 six-neighbour zero','positive_cut_surface':'positive surface site where candidate<raw at that site','gt005_surface':'rho>.005 native site with >=1 six-neighbour not>.005','gt041_surface':'rho>.041 native site with >=1 six-neighbour not>.041','H_partial_positive':'positive native site in bottom H transition, Y<246','F_partial_positive':'positive native site in any X/Z F transition outside full plateau','lower_positive_Y_lt300':'positive native site with native centre Y<300'},'results':results,'wall_s':time.perf_counter()-START,'sampled_peak_rss_bytes':PEAK,'process_highwater_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss}
 (R/'boundary.json').write_text(json.dumps(out,indent=2)+'\n')
 print(json.dumps({'wall_s':out['wall_s'],'peakMiB':PEAK/2**20,'parts':{k:{m:v['BODY_q_exact_zero']for m,v in x['masks'].items()}for k,x in results.items()}}))
if __name__=='__main__':
 try:main()
 except Exception as e:
  (R/'boundary-adverse.json').write_text(json.dumps({'error':repr(e),'wall_s':time.perf_counter()-START,'peak':PEAK})+'\n');raise
