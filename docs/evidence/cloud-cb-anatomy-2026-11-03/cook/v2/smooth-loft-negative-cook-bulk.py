from pathlib import Path
import numpy as np,json,hashlib,gc,resource,platform
import scipy
from scipy import ndimage
from PIL import Image,ImageDraw
R=Path(__file__).resolve().parent;BASE=R.parent;N=384;BMIN=np.array([-6000.,0.,-6000.]);EXT=np.array([12000.,14000.,12000.]);PITCH=EXT/N
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
base_meta=json.loads((BASE/'negative-f30d/cb-anatomy-384.json').read_text());node_records=base_meta['anatomy']['nodes']
def smooth(a,b,v):
 t=np.clip((v-a)/(b-a),0,1);return t*t*(3-2*t)
# One explicit broad interior loft. These controls author geometry, not wind/time or opacity.
LOFT={'height_m':[900,1600,2500,3800,5300,6700,7800,8900,9800],
 'radius_x_m':[300,1800,2600,2800,2600,2250,1850,1300,350],
 'radius_z_m':[300,1650,2350,2450,2250,2000,1750,1300,350],
 'center_x_m':[-700,-650,-650,-600,-450,-200,0,150,350],
 'center_z_m':[350,300,250,150,50,-50,-50,0,0],
 'angular_boundary':'1+.08*cos(3theta+.00019*y)+.05*sin(5theta-.00031*y)',
 'radial_window':[.78,1.04],'bottom_window_m':[1200,1900],'top_window_m':[8800,9800],
 'interior_density_floor':.075,'source_detail_gain':.45,'detail_master':'03/256',
 'detail_local_from_world':'local=(world-[-300,4800,100])/[14,24,16]+256; trilinear filtered density; density variation only, not stretchedCu outer geometry',
 'composition':'max(q_f30d,window*(.075+.45*sample03)); q_floor adds actual bulk density, kappa unchanged'}
def profile(y):
 # Monotone cubic interpolants avoid radius overshoot between explicit height sections.
 from scipy.interpolate import PchipInterpolator
 return [PchipInterpolator(LOFT['height_m'],LOFT[k],extrapolate=False)(y) for k in ['radius_x_m','radius_z_m','center_x_m','center_z_m']]
def write_bulk():
 old_path=BASE/'negative-f30d/instrument/assets/cb-anatomy-384.density.r16f';assert sha(old_path)==base_meta['output']['sha256'];old=np.memmap(old_path,dtype='<f2',mode='r',shape=(N,N,N));new=np.memmap(R/'cb-anatomy-384.density.r16f',dtype='<f2',mode='w+',shape=(N,N,N));raw=np.memmap(R/'inputs/cloud-03-256.density.r16f',dtype='<f2',mode='r',shape=(256,256,256));detail=raw.astype(np.float32)
 sigma=np.sqrt(np.maximum((PITCH/np.array([14,24,16])/2)**2/12-1/12,0));ndimage.gaussian_filter(detail,sigma[::-1],mode='constant',cval=0,output=detail)
 x=BMIN[0]+(np.arange(N)+.5)*PITCH[0];z=BMIN[2]+(np.arange(N)+.5)*PITCH[2];X,Z=np.meshgrid(x,z,indexing='xy');added=0.;changed=0;interior={label:{'voxels':0,'above_05peak':0,'above_10peak':0,'sum':0.} for label in ['f30d','bulk_v2']};fixed_roi={'x_m':[-1100,1100],'y_m':[3500,7800],'z_m':[-900,900]};peak=base_meta['stats']['peak']
 for y in range(N):
  height=(y+.5)*PITCH[1];a=old[:,y,:].astype(np.float32);b=a.copy()
  if 1200<height<9800:
   rx,rz,cx,cz=profile(height)
   if np.isfinite(rx) and rx>0 and rz>0:
    nx=(X-cx)/rx;nz=(Z-cz)/rz;theta=np.arctan2(nz,nx);irregular=1+.08*np.cos(3*theta+.00019*height)+.05*np.sin(5*theta-.00031*height);rho=np.sqrt(nx*nx+nz*nz)/irregular
    window=(1-smooth(.78,1.04,rho))*smooth(1200,1900,height)*(1-smooth(8800,9800,height));coords=np.array([((Z-100)/16+256)/2-.5,np.full_like(X,((height-4800)/24+256)/2-.5),((X+300)/14+256)/2-.5],dtype=np.float32);qdetail=ndimage.map_coordinates(detail,coords,order=1,mode='constant',cval=0,prefilter=False);np.maximum(a,window*(.075+.45*qdetail),out=b)
  new[:,y,:]=b.astype('<f2');actual=new[:,y,:].astype(np.float32);added+=float(np.sum(actual-a,dtype=np.float64));changed+=int(np.count_nonzero(actual!=a))
  if 3500<=height<=7800:
   roi=(np.abs(X)<=1100)&(np.abs(Z)<=900)
   for label,q in [('f30d',a),('bulk_v2',actual)]:
    vals=q[roi];o=interior[label];o['voxels']+=len(vals);o['above_05peak']+=int(np.count_nonzero(vals>.05*peak));o['above_10peak']+=int(np.count_nonzero(vals>.1*peak));o['sum']+=float(vals.sum(dtype=np.float64))
 new.flush();del detail,raw;gc.collect();cuts=[]
 def longest(mask):
  padded=np.r_[False,mask,False].astype(np.int8);d=np.diff(padded);starts=np.flatnonzero(d==1);ends=np.flatnonzero(d==-1);return int(np.max(ends-starts,initial=0))
 for height in [2500,3500,4500,6000,7500,8500]:
  iy=int(round(height/PITCH[1]-.5))
  for axis in ['front_Z','side_X']:
   for offset in [-1000,0,1000]:
    i=int(round((offset+6000)/31.25-.5));vectors=[old[:,iy,i],new[:,iy,i]] if axis=='front_Z' else [old[i,iy,:],new[i,iy,:]]
    record={'height_world_m':float((iy+.5)*PITCH[1]),'ray_axis':axis,'cross_axis_offset_m':offset,'kappa_per_m':.0045}
    for label,vec in zip(['f30d','bulk_v2'],vectors):
     v=vec.astype(np.float32);tau=float(v.sum(dtype=np.float64)*31.25*.0045);record[label]={'density_sum':float(v.sum(dtype=np.float64)),'optical_depth':tau,'T':float(np.exp(-tau)),'longest_occupied_chord_m_above_05peak':longest(v>.05*peak)*31.25,'occupied_chord_m_above_10peak':int(np.count_nonzero(v>.1*peak))*31.25,'q_samples_native':v.tolist()}
    cuts.append(record)
 for o in interior.values():o['fraction_above_05peak']=o['above_05peak']/o['voxels'];o['fraction_above_10peak']=o['above_10peak']/o['voxels'];o['mean_density']=o['sum']/o['voxels']
 report={'loft':LOFT,'detail_prefilter_sigma_native_xyz':sigma.tolist(),'source_base_sha256':base_meta['output']['sha256'],'source_detail_sha256':sha(R/'inputs/cloud-03-256.density.r16f'),'fixed_central_void_roi':fixed_roi,'fixed_roi_occupancy':interior,'changed_native_voxels':changed,'added_native_density_sum':added,'relative_added_density_sum':added/base_meta['stats']['sum'],'linecuts':cuts,'scope':'Native scalar occupancy and Beer columns at unchangedkappa. Fixed Cartesian ROI declared independently of loft window. This tests filled bulk, not accepted shape/light, constant source rates or lifecycle.'}
 (R/'bulk-diagnostics.json').write_text(json.dumps(report,indent=2)+'\n')
 # Native central slices: same absoluteqrange, before/after, no maxprojection hiding hollows.
 out=Image.new('RGB',(4*N,N+35),(20,25,32));draw=ImageDraw.Draw(out)
 for j,(label,plane) in enumerate([('f30d centreZ slice',old[N//2,:,:]),('bulk v2 centreZ slice',new[N//2,:,:]),('f30d centreX slice',old[:,:,N//2].T),('bulk v2 centreX slice',new[:,:,N//2].T)]):
  rgb=np.repeat(np.round(np.clip(plane.astype(np.float32)/.2,0,1)*255).astype('uint8')[:,:,None],3,axis=2);out.paste(Image.fromarray(np.flipud(rgb)),(j*N,35));draw.text((j*N+5,8),label+' q0..0.2',fill='white')
 out.save(R/'native-central-slices.png');print('Actual added mass fraction',report['relative_added_density_sum'],'fixed ROI',interior,'RSSMiB',resource.getrusage(resource.RUSAGE_SELF).ru_maxrss/1024**2,flush=True)
# Shared native diagnostic functions are copied byte-for-byte from the preservedf30d recipe.
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
 parent=[0];size=[0];flags=[0];yrange=[[N,-1]];seed_counts=[0,0,0];previous=np.zeros((N,N),np.int32)
 def root(i):
  while parent[i]!=i:parent[i]=parent[parent[i]];i=parent[i]
  return i
 for y in range(N):
  labels,count=ndimage.label(q[:,y,:]>threshold);counts=np.bincount(labels.ravel());base=len(parent)-1
  parent.extend(range(base+1,base+count+1));size.extend(counts[1:].tolist());flags.extend([0]*count);yrange.extend([[y,y] for _ in range(count)]);current=np.where(labels>0,labels+base,0)
  seeds=[([-2000,1600,-1500],[2000,3500,1500]),([-1800,8200,-1200],[1000,9600,1600]),([1000,9900,-1500],[4000,11200,1500])]
  for sid,(lo,hi) in enumerate(seeds):
   if lo[1]<=BMIN[1]+(y+.5)*PITCH[1]<=hi[1]:
    ix=np.where((BMIN[0]+(np.arange(N)+.5)*PITCH[0]>=lo[0])&(BMIN[0]+(np.arange(N)+.5)*PITCH[0]<=hi[0]))[0];iz=np.where((BMIN[2]+(np.arange(N)+.5)*PITCH[2]>=lo[2])&(BMIN[2]+(np.arange(N)+.5)*PITCH[2]<=hi[2]))[0];lab=current[np.ix_(iz,ix)];seed_counts[sid]+=int(np.count_nonzero(lab))
    for item in np.unique(lab):
     if item:flags[root(int(item))]|=1<<sid
  mask=(current>0)&(previous>0)
  if np.any(mask):
   pairs=np.unique(np.stack([current[mask],previous[mask]],axis=1),axis=0)
   for a,b in pairs:
    a,b=root(int(a)),root(int(b))
    if a!=b:
     if size[a]<size[b]:a,b=b,a
     parent[b]=a;size[a]+=size[b];flags[a]|=flags[b];yrange[a]=[min(yrange[a][0],yrange[b][0]),max(yrange[a][1],yrange[b][1])]
  previous=current
 sizes=sorted((size[i] for i in range(1,len(parent)) if parent[i]==i),reverse=True);total=sum(sizes)
 roots=[i for i in range(1,len(parent)) if parent[i]==i];return {'seed_nonzero_voxels_body_core_cap':seed_counts,'body_to_highest_core_connected':any(flags[i]&3==3 for i in roots),'core_to_cap_connected':any(flags[i]&6==6 for i in roots),'body_core_cap_connected':any(flags[i]&7==7 for i in roots),'largest_components_y_range_m':[[float((yrange[i][0]+.5)*PITCH[1]),float((yrange[i][1]+.5)*PITCH[1]),size[i]] for i in sorted(roots,key=lambda i:size[i],reverse=True)[:8]],'components':len(sizes),'substantial_components_ge_512_voxels':sum(v>=512 for v in sizes),'positive_voxels':total,'largest_fraction':sizes[0]/max(total,1) if sizes else 0,'largest_component_voxels':sizes[:8]}

def diagnose(diag):
 q=np.memmap(R/'cb-anatomy-384.density.r16f',dtype='<f2',mode='r',shape=(N,N,N));peak=float(q.max());sumq=0.;invalid=0;nonzero=0;low=np.array([N,N,N]);high=np.zeros(3,int);centroid=np.zeros(3);projectionZ=np.zeros((N,N),np.float64);projectionY=np.zeros((N,N),np.float64);projectionX=np.zeros((N,N),np.float64);maxZ=np.zeros((N,N),np.float32);maxX=np.zeros((N,N),np.float32);maxY=np.zeros((N,N),np.float32);neck=[]
 for z in range(N):
  plane=q[z].astype(np.float32);invalid+=int(np.count_nonzero(~np.isfinite(plane)|(plane<0)|(plane>1)));sumq+=float(np.sum(plane,dtype=np.float64));nonzero+=int(np.count_nonzero(plane));coords=np.nonzero(plane)
  if coords[0].size:low=np.minimum(low,[coords[1].min(),coords[0].min(),z]);high=np.maximum(high,[coords[1].max(),coords[0].max(),z])
  sy=plane.sum(1,dtype=np.float64);sx=plane.sum(0,dtype=np.float64);centroid+=np.array([np.dot(sx,BMIN[0]+(np.arange(N)+.5)*PITCH[0]),np.dot(sy,BMIN[1]+(np.arange(N)+.5)*PITCH[1]),plane.sum(dtype=np.float64)*(BMIN[2]+(z+.5)*PITCH[2])]);projectionZ+=plane;projectionY[z]=sx;projectionX[z]=sy;maxZ=np.maximum(maxZ,plane);maxY[z]=plane.max(0);maxX[z]=plane.max(1)
 for y in range(N):
  mask=q[:,y,:]>.05*peak;neck.append({'height_world_m':float(BMIN[1]+(y+.5)*PITCH[1]),'area_above_05peak_m2':int(mask.sum())*PITCH[0]*PITCH[2]})
 connected={str(f):components(q,peak*f) for f in [0,.01,.05,.1,.3]}
 low_attach=0.;high_attach=.3
 for _ in range(8):
  mid=(low_attach+high_attach)/2
  if components(q,peak*mid)['body_core_cap_connected']:low_attach=mid
  else:high_attach=mid
 attachment={'relative_peak_lower_bound':low_attach,'relative_peak_upper_bound':high_attach,'absolute_q_lower_bound':low_attach*peak,'definition':'Maximum6-neighbor threshold linking nonempty body/core/cap seed boxes, bisection bracket; a path metric, not bulk neck diameter proof'}
 # Scientific diagnostic panels; raw max and Beer opacity only, no lighting baked.
 panels=[('Front max q / global peak',maxZ),('Side max q / global peak',maxX.T),('Top max q / global peak',maxY),('Front opacity kappa=.0045',1-np.exp(-projectionZ*PITCH[2]*.0045)),('Side opacity kappa=.0045',(1-np.exp(-projectionX*PITCH[0]*.0045)).T),('Top opacity kappa=.0045',1-np.exp(-projectionY*PITCH[1]*.0045))]
 image=Image.new('RGB',(3*N,2*(N+30)),(20,25,32));draw=ImageDraw.Draw(image)
 for i,(label,a) in enumerate(panels):
  if i<3:a=a/peak
  rgb=np.repeat(np.round(np.clip(a,0,1)*255).astype('uint8')[:,:,None],3,axis=2);pic=Image.fromarray(np.flipud(rgb));x=(i%3)*N;y=(i//3)*(N+30);image.paste(pic,(x,y+30));draw.text((x+5,y+6),label,fill=(240,240,240))
 image.save(R/'projections.png')
 metadata={'id':'thrum.cloud-cb-anatomy.frozen.v1','classification':'One authored frozen Cb anatomy challenger, not accepted source/lifecycle/art','output':{'file':'cb-anatomy-384.density.r16f','sha256':sha(R/'cb-anatomy-384.density.r16f'),'bytes':N**3*2,'dimensions_xyz':[N,N,N],'storage':'little-endianR16F Xfastest numpy[Z,Y,X]','bounds_min_corner_xyz_m':BMIN.tolist(),'bounds_max_corner_xyz_m':(BMIN+EXT).tolist(),'voxel_pitch_xyz_m':PITCH.tolist()},'world_target_m':[0,6500,0],'density_weighted_centroid_world_m':(centroid/sumq).tolist(),'stats':{'peak':peak,'sum':sumq,'nonzero':nonzero,'invalid':invalid,'support_bbox_min_xyz':low.tolist(),'support_bbox_max_xyz':high.tolist(),'minimum_zero_collar_voxels_xyz':np.minimum(low,N-1-high).tolist(),'connected_6_neighbor_relative_peak':connected,'attachment_bottleneck':attachment,'horizontal_neck_sections':neck},'anatomy':{'composition':'Maximum of transformed linear density ingredients; never sum duplicated density; no aloft procedural sphere stamps','node_count':len(node_records),'nodes':node_records,'wind_direction_world':[1,0,0],'fringe':{'role':'Only cap ingredients, downwind smoothstep1500..4400m X; density carved by fixed stretched channels','seed':79241,'noise_dimensions_zyx':[96,96,96],'noise_sigma_zyx_cells':[1.4,1.3,9.],'noise_file':'fibres-96.r16f','noise_sha256':sha(R/'fibres-96.r16f'),'density_multiplier':'1-smoothstep(1500,4400,x)*(1-smoothstep(.32,.66,N))','scope':'Frozen authored directional edge content, not simulated ice phase, wind transport or dynamic fibre correspondence'}},'ingredient_sampling':diag,'recipe':{'file':'cook.py','sha256':sha(Path(__file__)),'python':platform.python_version(),'numpy':np.__version__,'scipy':scipy.__version__,'peak_process_rss_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss},'scope':['No complete lifecycle or four variants','No physical or phase-partition claim: density is scalar liquid for shared optical test','Gaussian footprint prefilter is approximate; native input resolution and max-union seams remain art limitations','Camera/source/kernel cost not production/fulls sky admission']}
 (R/'cb-anatomy-384.json').write_text(json.dumps(metadata,indent=2)+'\n');coarsemeta=dict(metadata);coarsemeta['output']={**metadata['output'],'file':'cb-anatomy-256.density.r16f','sha256':sha(R/'cb-anatomy-256.density.r16f'),'bytes':256**3*2,'dimensions_xyz':[256]*3,'voxel_pitch_xyz_m':(EXT/256).tolist()};coarsemeta['quality_policy']='Exact separable overlap-average of384piecewise-constant voxels thenhalf;384metadata native topology remains reference';(R/'cb-anatomy-256.json').write_text(json.dumps(coarsemeta,indent=2)+'\n');print('Final',metadata['output']['sha256'],'peak',peak,'connected',connected,'rssMiB',resource.getrusage(resource.RUSAGE_SELF).ru_maxrss/1024**2,flush=True)
if __name__=='__main__':
 write_bulk();coarse();diagnose(base_meta['ingredient_sampling']);m=json.loads((R/'cb-anatomy-384.json').read_text());m['id']='thrum.cloud-cb-anatomy.frozen.bulk-v2';m['anatomy']['composition']=LOFT['composition'];m['anatomy']['bulk_loft']=LOFT;m['base_negative']={'build':'f30d926dcc6695fc','source_sha256':base_meta['output']['sha256'],'recipe_sha256':base_meta['recipe']['sha256'],'rejection':'Skeletalarch/hollow cauliflower; path connectivity did not supply voluminous body'};m['bulk_diagnostics']={'file':'bulk-diagnostics.json','sha256':sha(R/'bulk-diagnostics.json')};m['recipe']['file']='cook-bulk.py';m['scope'].append('Bulk v2 is a substantive frozen geometry revision, not a kappa/noise/lighting change');(R/'cb-anatomy-384.json').write_text(json.dumps(m,indent=2)+'\n');c=json.loads((R/'cb-anatomy-256.json').read_text());c['id']=m['id'];c['anatomy']=m['anatomy'];c['base_negative']=m['base_negative'];c['bulk_diagnostics']=m['bulk_diagnostics'];c['recipe']['file']='cook-bulk.py';(R/'cb-anatomy-256.json').write_text(json.dumps(c,indent=2)+'\n')
