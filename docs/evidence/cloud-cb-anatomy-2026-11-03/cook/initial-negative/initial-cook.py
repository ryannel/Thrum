from pathlib import Path
import json,hashlib,gc,resource,platform,time
import numpy as np
import scipy
from scipy import ndimage
from PIL import Image,ImageDraw
R=Path(__file__).resolve().parent
BMIN=np.array([-6000.,0.,-6000.]);EXT=np.array([12000.,14000.,12000.]);N=384;PITCH=EXT/N
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def smooth(a,b,v):
 t=np.clip((v-a)/(b-a),0,1);return t*t*(3-2*t)
def rotation(yaw,pitch,roll):
 a,b,c=np.deg2rad([yaw,pitch,roll]);ry=np.array([[np.cos(a),0,np.sin(a)],[0,1,0],[-np.sin(a),0,np.cos(a)]]);rx=np.array([[1,0,0],[0,np.cos(b),-np.sin(b)],[0,np.sin(b),np.cos(b)]]);rz=np.array([[np.cos(c),-np.sin(c),0],[np.sin(c),np.cos(c),0],[0,0,1]])
 return rz@ry@rx
# All anatomy placements authored in metres; no animated deformation or per-frame detail.
nodes=[
 ('body-main','03',[-600,2250,300],[15,12,16],-12,0,2,1.,'body'),
 ('body-rear','08',[1750,2400,-1100],[12,10,13],32,0,-4,.9,'body'),
 ('body-front','03',[-2200,2050,1600],[10,11,12],-28,0,3,.9,'body'),
 ('body-left','08',[-2800,2350,-1000],[10,10,11],58,0,-2,.8,'body'),
 ('main-root','03',[-950,5400,200],[9,12,14],-15,90,-5,1.,'tower'),
 ('main-upper','08',[-450,7700,150],[10,13,12],26,85,7,.9,'tower'),
 ('left-tower','03',[-2700,5200,700],[8,12,11],57,80,-12,.9,'tower'),
 ('left-shoulder','08',[-2200,6900,300],[9,12,9],-23,72,10,.85,'tower'),
 ('rear-tower','03',[1500,6000,-1400],[9,13,13],-34,95,10,.9,'tower'),
 ('rear-root','08',[2000,3900,-1300],[10,12,10],52,78,-10,.9,'tower'),
 ('front-shoulder','03',[-800,4200,2400],[10,14,9],34,65,15,.8,'tower'),
 ('cap-main','08',[1000,10300,0],[14,14,20],90,0,4,.65,'cap'),
 ('cap-root','03',[-900,9600,-300],[12,14,13],70,0,-8,.8,'cap'),
 ('cap-front','03',[1800,10100,1600],[11,12,13],105,0,-6,.55,'cap'),
 ('cap-downwind','08',[3300,10400,-600],[7,9,9],100,0,-4,.35,'cap'),
]
node_records=[]
for id,master,c,s,yaw,pitch,roll,gain,role in nodes:
 A=rotation(yaw,pitch,roll)@np.diag(s);inv=np.linalg.inv(A)
 # Gaussian matches additional box-footprint variance beyond the existing native voxel average.
 sigma=np.sqrt(np.maximum(np.sum((inv*PITCH[None,:])**2,axis=1)/(12*2**2)-1/12,0))
 node_records.append({'id':id,'ingredient':master,'center_world_m':c,'scale_world_per_local_m':s,'euler_yaw_pitch_roll_deg':[yaw,pitch,roll],'world_from_local_centered_matrix':A.tolist(),'inverse':inv.tolist(),'gain':gain,'role':role,'prefilter_sigma_native_xyz':sigma.tolist()})
rng=np.random.Generator(np.random.PCG64(79241));noise=rng.standard_normal((96,96,96)).astype(np.float32);ndimage.gaussian_filter(noise,[1.4,1.3,9.],mode='wrap',output=noise);noise=(noise-noise.min())/(noise.max()-noise.min());noise.astype('<f2').tofile(R/'fibres-96.r16f')
# Axes[Z,Y,X]: long coherent X channels, 175-200m transverse standard deviation.
def fringe(values,worldx,worldy,worldz):
 coords=np.array([(worldz-BMIN[2])/EXT[2]*96-.5,(worldy-BMIN[1])/EXT[1]*96-.5,(worldx-BMIN[0])/EXT[0]*96-.5],dtype=np.float32)
 a=ndimage.map_coordinates(noise,coords,order=1,mode='nearest',prefilter=False);channel=smooth(.32,.66,a)
 return values*(1-smooth(1500,4400,worldx)*(1-channel))

def compose():
 dst=R/'cb-anatomy-384.density.r16f';q=np.memmap(dst,dtype='<f2',mode='w+',shape=(N,N,N));q[:]=0
 diagnostics=[]
 for no,node in enumerate(node_records):
  master=node['ingredient'];meta=json.loads((R/'inputs'/f'cloud-{master}-256.json').read_text());input=R/'inputs'/f'cloud-{master}-256.density.r16f';assert sha(input)==meta['output']['sha256']
  original=np.memmap(input,dtype='<f2',mode='r',shape=(256,256,256));v=original.astype(np.float32);before=float(np.sum(v,dtype=np.float64));peak=float(v.max());before_columns=[v.sum(axis=i,dtype=np.float64) for i in range(3)]
  ndimage.gaussian_filter(v,node['prefilter_sigma_native_xyz'][::-1],mode='constant',cval=0,truncate=3,output=v)
  maxerr=0.;sumerr=0.
  for z in range(0,256,8):diff=np.abs(v[z:z+8]-original[z:z+8].astype(np.float32));maxerr=max(maxerr,float(diff.max()));sumerr+=float(np.sum(diff,dtype=np.float64))
  filtering={'max_abs_density_change':maxerr,'mean_abs_density_change_full_input_cube':sumerr/256**3,'peak_retention':float(v.max())/peak,'relative_integral_change':float(np.sum(v,dtype=np.float64))/before-1,'ingredient_axis_column_relative_L1':[float(np.sum(np.abs(v.sum(axis=i,dtype=np.float64)-before_columns[i]),dtype=np.float64))/before for i in range(3)],'scope':'Native ingredient prefilter diagnostics; not full-formation arbitrary-ray alias error or beauty qualification'}
  A=np.array(node['world_from_local_centered_matrix']);inv=np.array(node['inverse']);center=np.array(node['center_world_m']);sigma=np.array(node['prefilter_sigma_native_xyz']);lo=np.array(meta['quality']['support_bbox_min_zyx'][::-1]) if 'quality' in meta else None
  # Exact metadata field is output_quality; select by support presence without assuming a descriptive key.
  quality=next(d for d in meta.values() if isinstance(d,dict) and 'support_bbox_min_zyx' in d)
  low=np.array(quality['support_bbox_min_zyx'][::-1],float)-np.ceil(3*sigma)-2;high=np.array(quality['support_bbox_max_zyx'][::-1],float)+np.ceil(3*sigma)+2
  corners=np.array([[x,y,z] for x in [low[0],high[0]+1] for y in [low[1],high[1]+1] for z in [low[2],high[2]+1]])*2-256
  wc=corners@A.T+center;imin=np.maximum(1,np.floor((wc.min(0)-BMIN)/PITCH-.5).astype(int));imax=np.minimum(N-2,np.ceil((wc.max(0)-BMIN)/PITCH-.5).astype(int));xx=BMIN[0]+(np.arange(imin[0],imax[0]+1)+.5)*PITCH[0];zz=BMIN[2]+(np.arange(imin[2],imax[2]+1)+.5)*PITCH[2]
  X,Z=np.meshgrid(xx,zz,indexing='xy');sampled=0
  for y0 in range(imin[1],imax[1]+1,4):
   yy=BMIN[1]+(np.arange(y0,min(y0+4,imax[1]+1))+.5)*PITCH[1];shape=(len(zz),len(yy),len(xx));wx=np.broadcast_to(X[:,None,:],shape);wz=np.broadcast_to(Z[:,None,:],shape);wy=np.broadcast_to(yy[None,:,None],shape)
   coords=np.empty((3,)+shape,np.float32)
   for k in range(3):coords[2-k]=(inv[k,0]*(wx-center[0])+inv[k,1]*(wy-center[1])+inv[k,2]*(wz-center[2])+256)/2-.5
   values=ndimage.map_coordinates(v,coords,order=1,mode='constant',cval=0,prefilter=False)*node['gain']
   if node['role']=='cap':values=fringe(values,wx,wy,wz)
   target=q[imin[2]:imax[2]+1,y0:y0+len(yy),imin[0]:imax[0]+1];np.maximum(target,values,out=values);target[:]=values.astype('<f2');sampled+=values.size
  diagnostics.append({'id':node['id'],'sampling_box_min_xyz':imin.tolist(),'sampling_box_max_xyz':imax.tolist(),'sampled_voxels':sampled,'filtering':filtering});q.flush();del v,original;gc.collect();print('Placed',node['id'],'samples',sampled,'rssMiB',resource.getrusage(resource.RUSAGE_SELF).ru_maxrss/1024**2,flush=True)
 q.flush();return diagnostics

def coarse():
 fine=np.memmap(R/'cb-anatomy-384.density.r16f',dtype='<f2',mode='r',shape=(N,N,N));dst=np.memmap(R/'cb-anatomy-256.density.r16f',dtype='<f2',mode='w+',shape=(256,256,256));i=np.arange(256);a=np.floor(i*1.5).astype(int);w=np.where(i%2==0,2/3,1/3).astype(np.float32)
 for z in range(256):
  total=np.zeros((256,256),np.float32)
  for j in range(2):
   plane=fine[a[z]+j].astype(np.float32);plane=plane[a,:]*w[:,None]+plane[a+1,:]*(1-w[:,None]);plane=plane[:,a]*w[None,:]+plane[:,a+1]*(1-w[None,:]);total+=plane*(w[z] if j==0 else 1-w[z])
  dst[z]=total.astype('<f2')
 dst.flush()

def components(q,threshold):
 # 6-connectivity by4-connected XZ slices plus coincident-pixel Y links; bounded slice memory.
 parent=[0];size=[0];previous=np.zeros((N,N),np.int32)
 def root(i):
  while parent[i]!=i:parent[i]=parent[parent[i]];i=parent[i]
  return i
 for y in range(N):
  labels,count=ndimage.label(q[:,y,:]>threshold);counts=np.bincount(labels.ravel());base=len(parent)-1
  parent.extend(range(base+1,base+count+1));size.extend(counts[1:].tolist());current=np.where(labels>0,labels+base,0)
  mask=(current>0)&(previous>0)
  if np.any(mask):
   pairs=np.unique(np.stack([current[mask],previous[mask]],axis=1),axis=0)
   for a,b in pairs:
    a,b=root(int(a)),root(int(b))
    if a!=b:
     if size[a]<size[b]:a,b=b,a
     parent[b]=a;size[a]+=size[b]
  previous=current
 sizes=sorted((size[i] for i in range(1,len(parent)) if parent[i]==i),reverse=True);total=sum(sizes)
 return {'components':len(sizes),'substantial_components_ge_512_voxels':sum(v>=512 for v in sizes),'positive_voxels':total,'largest_fraction':sizes[0]/max(total,1) if sizes else 0,'largest_component_voxels':sizes[:8]}

def diagnose(diag):
 q=np.memmap(R/'cb-anatomy-384.density.r16f',dtype='<f2',mode='r',shape=(N,N,N));peak=float(q.max());sumq=0.;invalid=0;nonzero=0;low=np.array([N,N,N]);high=np.zeros(3,int);centroid=np.zeros(3);projectionZ=np.zeros((N,N),np.float64);projectionY=np.zeros((N,N),np.float64);projectionX=np.zeros((N,N),np.float64);maxZ=np.zeros((N,N),np.float32);maxX=np.zeros((N,N),np.float32);maxY=np.zeros((N,N),np.float32);neck=[]
 for z in range(N):
  plane=q[z].astype(np.float32);invalid+=int(np.count_nonzero(~np.isfinite(plane)|(plane<0)|(plane>1)));sumq+=float(np.sum(plane,dtype=np.float64));nonzero+=int(np.count_nonzero(plane));coords=np.nonzero(plane)
  if coords[0].size:low=np.minimum(low,[coords[1].min(),coords[0].min(),z]);high=np.maximum(high,[coords[1].max(),coords[0].max(),z])
  sy=plane.sum(1,dtype=np.float64);sx=plane.sum(0,dtype=np.float64);centroid+=np.array([np.dot(sx,BMIN[0]+(np.arange(N)+.5)*PITCH[0]),np.dot(sy,BMIN[1]+(np.arange(N)+.5)*PITCH[1]),plane.sum(dtype=np.float64)*(BMIN[2]+(z+.5)*PITCH[2])]);projectionZ+=plane;projectionY[z]=sx;projectionX[z]=sy;maxZ=np.maximum(maxZ,plane);maxY[z]=plane.max(0);maxX[z]=plane.max(1)
 for y in range(N):
  mask=q[:,y,:]>.05*peak;neck.append({'height_world_m':float(BMIN[1]+(y+.5)*PITCH[1]),'area_above_05peak_m2':int(mask.sum())*PITCH[0]*PITCH[2]})
 connected={str(f):components(q,peak*f) for f in [0,.01,.05,.1,.3]}
 # Scientific diagnostic panels; raw max and Beer opacity only, no lighting baked.
 panels=[('Front max q / global peak',maxZ),('Side max q / global peak',maxX.T),('Top max q / global peak',maxY),('Front opacity kappa=.0045',1-np.exp(-projectionZ*PITCH[2]*.0045)),('Side opacity kappa=.0045',(1-np.exp(-projectionX*PITCH[0]*.0045)).T),('Top opacity kappa=.0045',1-np.exp(-projectionY*PITCH[1]*.0045))]
 image=Image.new('RGB',(3*N,2*(N+30)),(20,25,32));draw=ImageDraw.Draw(image)
 for i,(label,a) in enumerate(panels):
  if i<3:a=a/peak
  rgb=np.repeat(np.round(np.clip(a,0,1)*255).astype('uint8')[:,:,None],3,axis=2);pic=Image.fromarray(np.flipud(rgb));x=(i%3)*N;y=(i//3)*(N+30);image.paste(pic,(x,y+30));draw.text((x+5,y+6),label,fill=(240,240,240))
 image.save(R/'projections.png')
 metadata={'id':'thrum.cloud-cb-anatomy.frozen.v1','classification':'One authored frozen Cb anatomy challenger, not accepted source/lifecycle/art','output':{'file':'cb-anatomy-384.density.r16f','sha256':sha(R/'cb-anatomy-384.density.r16f'),'bytes':N**3*2,'dimensions_xyz':[N,N,N],'storage':'little-endianR16F Xfastest numpy[Z,Y,X]','bounds_min_corner_xyz_m':BMIN.tolist(),'bounds_max_corner_xyz_m':(BMIN+EXT).tolist(),'voxel_pitch_xyz_m':PITCH.tolist()},'world_target_m':[0,6500,0],'density_weighted_centroid_world_m':(centroid/sumq).tolist(),'stats':{'peak':peak,'sum':sumq,'nonzero':nonzero,'invalid':invalid,'support_bbox_min_xyz':low.tolist(),'support_bbox_max_xyz':high.tolist(),'minimum_zero_collar_voxels_xyz':np.minimum(low,N-1-high).tolist(),'connected_6_neighbor_relative_peak':connected,'horizontal_neck_sections':neck},'anatomy':{'composition':'Maximum of transformed linear density ingredients; never sum duplicated density; no aloft procedural sphere stamps','node_count':len(node_records),'nodes':node_records,'wind_direction_world':[1,0,0],'fringe':{'role':'Only cap ingredients, downwind smoothstep1500..4400m X; density carved by fixed stretched channels','seed':79241,'noise_dimensions_zyx':[96,96,96],'noise_sigma_zyx_cells':[1.4,1.3,9.],'noise_file':'fibres-96.r16f','noise_sha256':sha(R/'fibres-96.r16f'),'density_multiplier':'1-smoothstep(1500,4400,x)*(1-smoothstep(.32,.66,N))','scope':'Frozen authored directional edge content, not simulated ice phase, wind transport or dynamic fibre correspondence'}},'ingredient_sampling':diag,'recipe':{'file':'cook.py','sha256':sha(Path(__file__)),'python':platform.python_version(),'numpy':np.__version__,'scipy':scipy.__version__,'peak_process_rss_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss},'scope':['No complete lifecycle or four variants','No physical or phase-partition claim: density is scalar liquid for shared optical test','Gaussian footprint prefilter is approximate; native input resolution and max-union seams remain art limitations','Camera/source/kernel cost not production/fulls sky admission']}
 (R/'cb-anatomy-384.json').write_text(json.dumps(metadata,indent=2)+'\n');coarsemeta=dict(metadata);coarsemeta['output']={**metadata['output'],'file':'cb-anatomy-256.density.r16f','sha256':sha(R/'cb-anatomy-256.density.r16f'),'bytes':256**3*2,'dimensions_xyz':[256]*3,'voxel_pitch_xyz_m':(EXT/256).tolist()};coarsemeta['quality_policy']='Exact separable overlap-average of384piecewise-constant voxels thenhalf;384metadata native topology remains reference';(R/'cb-anatomy-256.json').write_text(json.dumps(coarsemeta,indent=2)+'\n');print('Final',metadata['output']['sha256'],'peak',peak,'connected',connected,'rssMiB',resource.getrusage(resource.RUSAGE_SELF).ru_maxrss/1024**2,flush=True)
if __name__=='__main__':
 diag=compose();coarse();diagnose(diag)
