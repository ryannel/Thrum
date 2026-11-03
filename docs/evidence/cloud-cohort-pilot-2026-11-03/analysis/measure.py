from pathlib import Path
import numpy as np,json,hashlib,resource,platform,gc
from scipy import ndimage
from PIL import Image,ImageDraw
R=Path(__file__).resolve().parent;P=R.parent/'cloud-cohort-proof';data=json.loads((R/'snapshots.json').read_text());anchor=np.array(data['anchor']);H=8.;thresholds=[1e-7,.001,.005,.01,.04]
rho=np.fromfile(P/'assets/cloud-03-256.density.r16f',dtype='<f2').reshape(256,256,256).astype(np.float32);dist=np.fromfile(P/'assets/cloud-03-256-iso0p001.distance.r16f',dtype='<f2').reshape(256,256,256).astype(np.float32);noise=np.fromfile(P/'assets/channels-32.r16f',dtype='<f2').reshape(32,32,32).astype(np.float32)
meta=json.loads((P/'assets/cloud-03-256.json').read_text());quality=next(v for v in meta.values() if isinstance(v,dict) and 'support_bbox_min_zyx' in v);imin=np.array(quality['support_bbox_min_zyx'])[::-1];imax=np.array(quality['support_bbox_max_zyx'])[::-1];localLo=(imin+.5)*2-2;localHi=(imax+.5)*2+2
smooth=lambda v:np.clip(v,0,1)**2*(3-2*np.clip(v,0,1))
def matrix(c):
 b=4*c['s']*c['g'];co=np.cos(c['yaw']);si=np.sin(c['yaw']);return np.array([[b*co,c['K'][0],-b*si],[0,b,0],[b*si,c['K'][1],b*co]])
def mapped(c,xx,yy,zz):
 A=matrix(c);dx=xx-anchor[0];dy=yy-anchor[1];dz=zz-anchor[2];return[c['C'][k]+A[k,0]*dx+A[k,1]*dy+A[k,2]*dz for k in range(3)]
def query(c,world,potential=False):
 b=4*c['s']*c['g'];co=np.cos(c['yaw']);si=np.sin(c['yaw']);dy=(world[1]-c['C'][1])/b;ux=world[0]-c['C'][0]-c['K'][0]*dy;uz=world[2]-c['C'][2]-c['K'][1]*dy;shape=np.broadcast_shapes(np.shape(ux),np.shape(uz),np.shape(dy));coords=np.empty((3,)+shape,np.float32);coords[0]=((-si*ux+co*uz)/b+anchor[2])/2-.5;coords[1]=(dy+anchor[1])/2-.5;coords[2]=((co*ux+si*uz)/b+anchor[0])/2-.5
 valid=np.all((coords>=-.5)&(coords<=255.5),axis=0);rr=ndimage.map_coordinates(rho,coords,order=1,mode='nearest',prefilter=False);dd=ndimage.map_coordinates(dist,coords,order=1,mode='nearest',prefilter=False)
 if potential:e=0.
 else:
  e=c['ageE']
  if e>0:e=e*(.3+.7*ndimage.map_coordinates(noise,(coords+.5)/8-.5,order=1,mode='nearest',prefilter=False))
 q=c['A']*rr*smooth((dd-e+8)/16);q[~valid]=0;return q
# Common weather-registered8mworld lattice. Translation alone is removed; actual vertical rise/shear/yaw stay.
mins=[];maxs=[]
for snap in data['snapshots']:
 for c in snap['items']:
  corners=np.array([[x,y,z]for x in[localLo[0],localHi[0]]for y in[localLo[1],localHi[1]]for z in[localLo[2],localHi[2]]]);v=(corners-anchor)@matrix(c).T+np.array(c['C'])-np.array([snap['region'][0],0,snap['region'][2]]);mins.append(v.min(0));maxs.append(v.max(0))
lo=np.floor(np.min(mins,0)/H)*H-H;hi=np.ceil(np.max(maxs,0)/H)*H+H;dims=np.ceil((hi-lo)/H).astype(int);axes=[lo[i]+(np.arange(dims[i])+.5)*H for i in range(3)];shape=tuple(dims[::-1]);print('ReferencegridXYZ',dims,'points',int(np.prod(dims)),flush=True)
def volumes(q):return{str(t):{'voxels':int(np.count_nonzero(q>t)),'volume_m3':int(np.count_nonzero(q>t))*H**3}for t in thresholds}
def metrics(child,parent,older):
 out={'density_mass_proxy':float(child.sum(dtype=np.float64))*H**3,'increase_over_actual_parent_mass_proxy':None,'thresholds':{}}
 for t in thresholds:
  cm=child>t;pm=parent>t;strict=parent<=1e-7;old=older>t;count=int(cm.sum());mass=float(child.sum(dtype=np.float64));nov=cm&~pm;strictnov=cm&strict;novold=cm&~old
  out['thresholds'][str(t)]={'child_volume_m3':count*H**3,'outside_parent_uneroded_volume_m3':int(nov.sum())*H**3,'outside_parent_uneroded_fraction':int(nov.sum())/max(count,1),'outside_all_older_uneroded_fraction':int(novold.sum())/max(count,1),'strict_outside_parent_support_fraction_of_child_threshold_voxels':int(strictnov.sum())/max(count,1),'child_total_mass_fraction_strictly_outside_parent_support':float(child[strict].sum(dtype=np.float64))/max(mass,1e-30)}
 out['projection_novelty']={}
 for axis,name in [(0,'front_Z'),(2,'side_X'),(1,'top_Y')]:
  cp=child.max(axis);pp=parent.max(axis);proj={}
  for t in [.001,.005,.01,.04]:
   cm=cp>t;pm=pp>t;nov=cm&~pm;count=int(nov.sum());distance=ndimage.distance_transform_edt(~pm)*H;proj[str(t)]={'child_pixels':int(cm.sum()),'novel_pixels':count,'novel_fraction_of_child_projection':count/max(int(cm.sum()),1),'novel_area_m2':count*H**2,'max_distance_from_parent_projection_m':float(distance[nov].max(initial=0)),'novel_area_ge_32m_from_parent_m2':int(np.count_nonzero(nov&(distance>=32)))*H**2}
  out['projection_novelty'][name]=proj
 return out
results=[];images=[];spots=[]
for snap in data['snapshots']:
 reg=np.array([snap['region'][0],0,snap['region'][2]]);fields={};pots={}
 for c in snap['items']:
  q=np.zeros(shape,np.float32);pot=np.zeros(shape,np.float32)
  for z in range(0,shape[0],4):
   w=[axes[0][None,None,:]+reg[0],axes[1][None,:,None],axes[2][z:z+4,None,None]+reg[2]];q[z:z+4]=query(c,w);pot[z:z+4]=query(c,w,True)
  fields[c['id']]=q;pots[c['id']]=pot
 actual=np.maximum.reduce(list(fields.values()));allpot=np.maximum.reduce(list(pots.values()));record={'time':snap['time'],'cohorts':[],'actual_joint':volumes(actual),'all_current_uneroded_potential':volumes(allpot),'actual_density_sum_proxy':float(actual.sum(dtype=np.float64))*H**3}
 for c in snap['items']:
  state=next(s for s in snap['cohortStates']if s['id']==c['id']);item={**c,'world_linear_map':matrix(c).tolist(),'native_mapped_voxel_edge_lengths_m':(2*np.linalg.norm(matrix(c),axis=0)).tolist(),'template_extent_world_m_estimate':(np.abs(matrix(c))@(localHi-localLo)).tolist(),'parent_site':state['parentSite'],'actual_min_resolved_neck_during_healthy_establishment_m':state['minResolved'],'actual_min_seed_burial_margin_m':state['minSeedMargin']}
  if c['parentId']:
   parent=next(p for p in snap['items']if p['id']==c['parentId']);site=np.array(state['parentSite']['local_xyz']);expected=np.array(mapped(parent,*site));item['distance_from_actual_parent_socket_m']=float(np.linalg.norm(np.array(c['C'])-expected));item['socket_offset_from_parent_anchor_world_m']=float(np.linalg.norm(expected-np.array(parent['C'])));item['linear_scale_ratio_to_parent']=4*c['s']*c['g']/(4*parent['s']*parent['g']);item['exact_uneroded_support_volume_ratio_to_parent']=item['linear_scale_ratio_to_parent']**3
   older=np.maximum.reduce([pots[p['id']]for p in snap['items']if p['born']<c['born']]);measured=metrics(fields[c['id']],pots[parent['id']],older);measured['increase_over_actual_parent_mass_proxy']=float(np.maximum(fields[c['id']]-fields[parent['id']],0).sum(dtype=np.float64))*H**3;measured['increase_over_uneroded_parent_mass_proxy']=float(np.maximum(fields[c['id']]-pots[parent['id']],0).sum(dtype=np.float64))*H**3;item['world8m']=measured
  record['cohorts'].append(item)
 for c in snap['items']:
  for delta in [[0,0,0],[12,0,0],[-12,0,0],[0,10,0],[0,0,20]]:
   a=anchor+delta;w=np.array(mapped(c,*a));value=float(max(query(k,[np.array([w[0]]),np.array([w[1]]),np.array([w[2]])])[0]for k in snap['items']));spots.append({'time':snap['time'],'world':w.tolist(),'q':value})
 images.append({'time':snap['time'],'actual_front':1-np.exp(-.019*actual.sum(0,dtype=np.float64)*H),'root_potential_front':1-np.exp(-.019*pots[1].sum(0,dtype=np.float64)*H),'actual_side':1-np.exp(-.019*actual.sum(2,dtype=np.float64)*H),'root_potential_side':1-np.exp(-.019*pots[1].sum(2,dtype=np.float64)*H)})
 results.append(record);print('Worldfieldtime',snap['time'],'done; RSSMiB',resource.getrusage(resource.RUSAGE_SELF).ru_maxrss/1024**2,flush=True);del fields,pots,actual,allpot;gc.collect()
# Full material-native lattice, including a half-cell shifted phase, catches details smallerthan8mwhileyoung.
for snap,record in zip(data['snapshots'],results):
 for child in [c for c in snap['items']if c['parentId']]:
  parent=next(c for c in snap['items']if c['id']==child['parentId']);older=[p for p in snap['items']if p['born']<child['born']];phases=[]
  for offset in [0,.5]:
   accum={str(t):{'n':0,'novel':0,'strict':0,'outsideOlder':0,'mass':0.,'strictMass':0.}for t in thresholds};qs=0.;outsideMass=0.
   for zi in range(max(0,imin[2]-1),min(256,imax[2]+2),4):
    xx=(np.arange(max(0,imin[0]-1),min(256,imax[0]+2))+.5+offset)*2;yy=(np.arange(max(0,imin[1]-1),min(256,imax[1]+2))+.5+offset)*2;zz=(np.arange(zi,min(zi+4,imax[2]+2))+.5+offset)*2;local=[xx[None,None,:],yy[None,:,None],zz[:,None,None]];world=mapped(child,*local);qc=query(child,world);qp=query(parent,world,True);qo=np.maximum.reduce([query(p,world,True)for p in older]);strict=qp<=1e-7;qs+=float(qc.sum(dtype=np.float64));outsideMass+=float(qc[strict].sum(dtype=np.float64))
    for t in thresholds:
     a=accum[str(t)];cm=qc>t;a['n']+=int(cm.sum());a['novel']+=int(np.count_nonzero(cm&(qp<=t)));a['strict']+=int(np.count_nonzero(cm&strict));a['outsideOlder']+=int(np.count_nonzero(cm&(qo<=t)))
   dv=(4*child['s']*child['g'])**3*8;phases.append({'material_lattice_phase_native_cells':offset,'voxel_volume_world_m3':dv,'native_mass_fraction_outside_parent_strict_support':outsideMass/max(qs,1e-30),'thresholds':{t:{'child_volume_m3':a['n']*dv,'outside_parent_uneroded_fraction':a['novel']/max(a['n'],1),'strict_outside_parent_support_fraction':a['strict']/max(a['n'],1),'outside_all_older_uneroded_fraction':a['outsideOlder']/max(a['n'],1)}for t,a in accum.items()}})
  item=next(c for c in record['cohorts']if c['id']==child['id']);item['material_native']=phases;print('Materialnative',snap['time'],child['id'],'done',flush=True)
# Fixedscale scientific opacity panels, weather-registered axes; no lighting/camera/alpha changes.
W=380;HH=190;canvas=Image.new('RGB',(5*W,4*HH),(15,20,27));draw=ImageDraw.Draw(canvas)
for col,img in enumerate(images):
 for row,key in enumerate(['actual_front','root_potential_front','actual_side','root_potential_side']):
  a=img[key];rgb=np.repeat(np.round(np.clip(a,0,1)*255).astype('uint8')[:,:,None],3,axis=2);pic=Image.fromarray(np.flipud(rgb));scale=min((W-12)/pic.width,(HH-30)/pic.height);pic=pic.resize((round(pic.width*scale),round(pic.height*scale)),Image.Resampling.NEAREST);canvas.paste(pic,(col*W+(W-pic.width)//2,row*HH+30));draw.text((col*W+5,row*HH+6),f"t{img['time']} {key} k=.019",fill='white')
canvas.save(R/'registered-opacity.png')
report={'build':data['build'],'method':{'world_grid_pitch_m':H,'world_grid_dimensions_xyz':dims.tolist(),'world_grid_registered_min_corner_xyz_m':lo.tolist(),'registration':'Subtract only common condensation-region XZ; retain actualCheight/internalmotion/shear/yaw','field':'Nativeρ,dseparate trilinear samples then exactkernelandpointmax; Eweather0verifiedbefore420; parentpotentialremovesage/weatherlossatSAMEactualmap','native_material':'Full boundednative256lattice plus .5cell shiftedphase, physicalvoxeldeterminantweights; no8mworldundersamplingblindspot','thresholds':thresholds,'limits':'Volume/contour quadrature, not continuousglobalinequality or renderedbeautyproof. Positive-tailnewvolume is not automatically outside projected silhouette.'},'sourcePins':data['sourcePins'],'snapshots':results,'peak_process_rss_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,'tools':{'python':platform.python_version(),'numpy':np.__version__},'recipe_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()};(R/'metrics.json').write_text(json.dumps(report,indent=2)+'\n');(R/'spots.json').write_text(json.dumps(spots,indent=2)+'\n');print('ALLDONE',report['peak_process_rss_bytes'],flush=True)
