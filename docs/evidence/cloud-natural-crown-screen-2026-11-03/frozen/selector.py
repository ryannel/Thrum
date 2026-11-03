#!/usr/bin/env python3
"""The one pinned S/O selector and two native-centre Euclidean distances."""
from pathlib import Path
import hashlib,json,os,resource,subprocess,time,sys
import numpy as np
R=Path(__file__).resolve().parent;T=R.parent;MASTER=T/'cloud-authored-master/cloud-03-256.density.r16f'
CORE=T/'cloud-natural-core-probe/result.json';COMMISSION=R/'COMMISSION.md'
SHA='42431f9da2d76e253d649cc35802fd1be7b23ddb698e73c6c4247ac3c90a97c9'
CSHA='3239afc598ddf2dafc249a7e718d9f64318a189cf8a92efc6dfde61c6fa135f3'
EARLY=112*2**20;HARD=160*2**20;PEAK=0;START=time.perf_counter()
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
def main():
 if len(sys.argv)!=2 or sys.argv[1] not in ('S','O'):raise RuntimeError('run exactly one selector EDT stage: S or O')
 stage=sys.argv[1]
 assert sha(COMMISSION)==CSHA and sha(MASTER)==SHA and MASTER.stat().st_size==256**3*2
 core=json.loads(CORE.read_text());assert core['source_sha256']==SHA
 guard('before crop',10*2**20)
 master=np.memmap(MASTER,dtype='<f2',mode='r',shape=(256,256,256))
 raw=np.array(master[72:144,112:176,56:152],dtype=np.float32,copy=True)
 master._mmap.close();del master
 assert raw.shape==(72,64,96)
 active=raw>=np.float32(.041);active[:,:38,:]=False # y native centre >=301
 assert bool(active[31,43,54])
 guard('before SciPy',35*2**20)
 from scipy import ndimage as ndi
 cross=np.zeros((3,3,3),bool);cross[1,1,:]=True;cross[1,:,1]=True;cross[:,1,1]=True
 labels,n=ndi.label(active,structure=cross);slabel=int(labels[31,43,54]);S=labels==slabel;O=active & ~S
 assert n==12 and int(S.sum())==19208 and int(O.sum())==2779 and slabel>0
 hit=np.where(S);bb=[[113+2*int(hit[2].min()),225+2*int(hit[1].min()),145+2*int(hit[0].min())],[113+2*int(hit[2].max()),225+2*int(hit[1].max()),145+2*int(hit[0].max())]]
 assert bb==[[131,301,171],[271,335,261]] and bb==core['S_bbox_whole_crop_centres_xyz_m']
 assert not any((S[0].any(),S[-1].any(),S[:,0,:].any(),S[:,-1,:].any(),S[:,:,0].any(),S[:,:,-1].any()))
 assert bb[0][0]>=128 and bb[1][0]<=288 and bb[0][2]>=160 and bb[1][2]<=272
 del labels,active,raw,hit
 selected=S if stage=='S' else O
 del S,O
 guard('before single EDT',28*2**20)
 dist=ndi.distance_transform_edt(~selected,sampling=(2.,2.,2.))
 assert dist.dtype==np.float64 and dist.shape==(72,64,96) and np.all(dist[selected]==0)
 dst=R/('d'+stage+'.f64');np.asarray(dist,dtype='<f8').tofile(dst);dstsha=sha(dst)
 # Independent nearest-centre reference at fixed, representative points.
 coords=np.argwhere(selected).astype(np.int16)
 samp=[(31,43,54),(0,38,0),(20,50,40),(71,63,95),(31,43,80),(35,42,52)]
 oracle=[]
 for z,y,x in samp:
  p=np.array([z,y,x],dtype=np.int32)
  ref=float(2*np.sqrt(np.min(np.sum((coords.astype(np.int32)-p)**2,axis=1))))
  assert abs(ref-dist[z,y,x])<1e-12
  oracle.append({'index_zyx':[z,y,x],'d'+stage+'_m':float(dist[z,y,x]),'brute_d'+stage+'_m':ref})
 guard('selector single stage complete')
 out={'scope':'one native S/O selector EDT stage only, no optical distance/density candidate','stage':stage,'source_sha256':SHA,'commission_sha256':CSHA,'core_result_sha256':sha(CORE),'script_sha256':sha(Path(__file__)),'crop_origin_xyz_m':[112,224,144],'crop_dims_xyz':[96,64,72],'crop_shape_zyx':[72,64,96],'threshold_float32_hex':'0x3d27ef9e','native_y_min_m':300,'seed_index_zyx':[31,43,54],'S_voxels':19208,'O_voxels':2779,'component_count':n,'S_bbox_xyz_m':bb,'distance':{'file':dst.name,'bytes':dst.stat().st_size,'sha256':dstsha},'distance_definition':'Float64 SciPy ndimage distance_transform_edt(~set,sampling=(2,2,2)); exact zero at source native centres; shape[Z,Y,X]','independent_oracle':oracle,'wall_s':time.perf_counter()-START,'sampled_peak_rss_bytes':PEAK,'process_highwater_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss}
 assert out['distance']['bytes']==72*64*96*8
 (R/('selector-'+stage+'.json')).write_text(json.dumps(out,indent=2)+'\n')
 print(json.dumps({'stage':stage,'S':19208,'O':2779,'peakMiB':PEAK/2**20,'wall_s':out['wall_s']}))
if __name__=='__main__':
 try:main()
 except Exception as e:
  (R/'selector-adverse.json').write_text(json.dumps({'error':repr(e),'wall_s':time.perf_counter()-START,'peak':PEAK})+'\n');raise
