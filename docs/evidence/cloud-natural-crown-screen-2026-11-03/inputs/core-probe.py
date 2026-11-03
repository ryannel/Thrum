#!/usr/bin/env python3
"""Exact six-connected upper-core diagnostic; never writes a density field."""
from pathlib import Path
import hashlib, json, os, resource, subprocess, time
import numpy as np
from PIL import Image, ImageDraw

R=Path(__file__).resolve().parent
SRC=R.parent/'cloud-authored-master/cloud-03-256.density.r16f'
META=R.parent/'cloud-authored-master/cloud-03-256.json'
EXPECTED='42431f9da2d76e253d649cc35802fd1be7b23ddb698e73c6c4247ac3c90a97c9'
ORIGIN=np.array([112,224,144],dtype=np.int32)
SPAN=np.array([192,128,144],dtype=np.int32)
DIMS=np.array([96,64,72],dtype=np.int32)
SEED=np.array([221,311,207],dtype=np.int32)
THRESHOLD=np.float32(0.041)
EARLY=112*2**20;HARD=160*2**20;t0=time.perf_counter();peak=0

def guard(name,reserve=0):
    global peak
    rss=int(subprocess.check_output(['ps','-o','rss=','-p',str(os.getpid())]).strip())*1024
    peak=max(peak,rss)
    if rss>=HARD or rss+reserve>=EARLY:raise RuntimeError(f'RSS guard {name}: {rss}+{reserve} >= early {EARLY}')
    return rss

def sha(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for b in iter(lambda:f.read(1<<20),b''):h.update(b)
    return h.hexdigest()

assert SRC.stat().st_size==256**3*2
assert sha(SRC)==EXPECTED
meta=json.loads(META.read_text());assert meta['output']['sha256']==EXPECTED
guard('before mmap',8*2**20)
master=np.memmap(SRC,dtype='<f2',mode='r',shape=(256,256,256))
lo=(ORIGIN//2).astype(int);hi=lo+DIMS
assert np.all(hi<=256)
crop=np.asarray(master[lo[2]:hi[2],lo[1]:hi[1],lo[0]:hi[0]],dtype=np.float32)
assert crop.shape==(72,64,96) and np.all(np.isfinite(crop)) and np.all(crop>=0)
guard('after crop',6*2**20)
active=crop>=THRESHOLD
ycentres=ORIGIN[1]+2*np.arange(DIMS[1])+1
active[:,ycentres<300,:]=False
seed_idx=(SEED-ORIGIN-1)//2
assert np.all(ORIGIN+2*seed_idx+1==SEED)
sx,sy,sz=map(int,seed_idx)
seed_q=float(crop[sz,sy,sx])
if not active[sz,sy,sx]:raise RuntimeError(f'exact native seed is not positive: {seed_q}')
guard('before labels',6*2**20)
flat=active.ravel();labels=np.zeros(flat.size,dtype=np.int32);shape=(72,64,96);stride_y=96;stride_z=64*96
components=[];lab=0
for start in np.flatnonzero(flat):
    start=int(start)
    if labels[start]:continue
    lab+=1;labels[start]=lab;queue=[start];head=0;count=0
    minx=miny=minz=10**9;maxx=maxy=maxz=-1
    while head<len(queue):
        i=queue[head];head+=1;count+=1
        z=i//stride_z;rem=i-z*stride_z;y=rem//stride_y;x=rem-y*stride_y
        minx=min(minx,x);maxx=max(maxx,x);miny=min(miny,y);maxy=max(maxy,y);minz=min(minz,z);maxz=max(maxz,z)
        for j,valid in ((i-1,x>0),(i+1,x<95),(i-stride_y,y>0),(i+stride_y,y<63),(i-stride_z,z>0),(i+stride_z,z<71)):
            if valid and flat[j] and labels[j]==0:labels[j]=lab;queue.append(j)
    components.append({'label':lab,'voxels':count,'bbox_index_xyz':[[minx,miny,minz],[maxx,maxy,maxz]],'bbox_centres_xyz_m':[(ORIGIN+2*np.array([minx,miny,minz])+1).tolist(),(ORIGIN+2*np.array([maxx,maxy,maxz])+1).tolist()]})
guard('after labels',4*2**20)
seed_flat=sz*stride_z+sy*stride_y+sx;Slabel=int(labels[seed_flat]);assert Slabel>0
S=labels.reshape(shape)==Slabel;O=active & ~S
seedcomp=components[Slabel-1];edge={
 'X_min':bool(S[:,:,0].any()),'X_max':bool(S[:,:,-1].any()),
 'Y_min':bool(S[:,0,:].any()),'Y_max':bool(S[:,-1,:].any()),
 'Z_min':bool(S[0,:,:].any()),'Z_max':bool(S[-1,:,:].any())}
bb=seedcomp['bbox_centres_xyz_m'];plateau=bb[0][0]>=128 and bb[1][0]<=288 and bb[0][2]>=160 and bb[1][2]<=272 and bb[0][1]>=246

def panel(name,s,o,axis_h,axis_v,hmin,vmin,seed_h,seed_v):
    # Arrays are native [vertical, horizontal] with vertical coordinate increasing upward.
    H,W=s.shape;rgb=np.zeros((H,W,3),dtype=np.uint8)
    rgb[s]=[15,220,235];rgb[o]=[250,154,45];rgb[s&o]=[249,110,229]
    im=Image.fromarray(rgb,'RGB').transpose(Image.Transpose.FLIP_TOP_BOTTOM).resize((W*4,H*4),Image.Resampling.NEAREST)
    canvas=Image.new('RGB',(W*4+82,H*4+73),(15,21,29));canvas.paste(im,(52,20));d=ImageDraw.Draw(canvas)
    for i in range(0,W,16):
        x=52+i*4;d.line((x,20,x,20+H*4),fill=(55,73,88));d.text((x+1,21+H*4),str(hmin+2*i),fill=(192,215,227))
    for i in range(0,H,16):
        y=20+(H-1-i)*4;d.line((52,y,52+W*4,y),fill=(55,73,88));d.text((4,y-4),str(vmin+2*i),fill=(192,215,227))
    xx=52+(seed_h-hmin)/2*4;yy=20+(H-1-(seed_v-vmin)/2)*4
    if 52<=xx<52+W*4 and 20<=yy<20+H*4:d.ellipse((xx-5,yy-5,xx+5,yy+5),outline=(245,255,245),width=2)
    d.text((52,2),name,fill=(235,241,248));d.text((52,47+H*4),f'{axis_h} horizontal; {axis_v} up; cyan S / orange O',fill=(205,222,232))
    path=R/(name.split(' ')[0].lower()+'.png');canvas.save(path,optimize=True);return path.name

lv=labels.reshape(shape)
images=[]
images.append(panel('front S/O',S.any(axis=0),O.any(axis=0),'X','Y',113,225,SEED[0],SEED[1]))
images.append(panel('side S/O',S.any(axis=2).T,O.any(axis=2).T,'Z','Y',145,225,SEED[2],SEED[1]))
images.append(panel('top S/O',S.any(axis=1),O.any(axis=1),'X','Z',113,145,SEED[0],SEED[2]))
guard('complete')
out={
 'scope':'native raw upper-core connectivity diagnostic only; no selector/mask/EDT/assets',
 'source_sha256':EXPECTED,'source_metadata_sha256':sha(META),'script_sha256':sha(Path(__file__)),
 'domain':{'origin_xyz_m':ORIGIN.tolist(),'span_xyz_m':SPAN.tolist(),'dimensions_xyz':DIMS.tolist(),'pitch_m':2,'native_shape_zyx':list(shape)},
 'decode':'R16F→Float32; compare >= np.float32(0.041) and native Y centre >=300m; no Float64 threshold ambiguity',
 'threshold_float32_hex':hex(int(np.frombuffer(THRESHOLD.tobytes(),dtype=np.uint32)[0])),
 'connectivity':6,'seed_xyz_m':SEED.tolist(),'seed_index_xyz':seed_idx.tolist(),'seed_q_float32':seed_q,
 'S_label':Slabel,'S_voxels':int(S.sum()),'O_voxels':int(O.sum()),'O_nonempty':bool(O.any()),'positive_total':int(active.sum()),
 'S_bbox_whole_crop_centres_xyz_m':bb,'S_edge_contact':edge,'S_touches_any_crop_edge':any(edge.values()),
 'S_inside_plateau_X128_288_Z160_272_Yge246':bool(plateau),
 'component_count':len(components),'components':components,'images':images,
 'elapsed_s':time.perf_counter()-t0,'sampled_peak_rss_bytes':peak,'process_highwater_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
 'planned_early_rss_bytes':EARLY,'hard_rss_bytes':HARD,
}
assert sum(c['voxels'] for c in components)==int(active.sum()) and out['S_voxels']+out['O_voxels']==int(active.sum())
(R/'result.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({k:out[k] for k in ['S_label','S_voxels','O_voxels','component_count','S_bbox_whole_crop_centres_xyz_m','S_edge_contact','S_inside_plateau_X128_288_Z160_272_Yge246','elapsed_s','process_highwater_bytes']}))
