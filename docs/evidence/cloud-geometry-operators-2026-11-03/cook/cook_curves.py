from pathlib import Path
import json,hashlib,gc,platform
import numpy as np
import scipy
from scipy import ndimage
R=Path(__file__).resolve().parent;S=R.parent/'cloud-authored-master'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def smooth(x):
 t=np.clip(x,0,1);return t*t*(3-2*t)

def noise():
 p=R/'channels-32.r16f';rng=np.random.Generator(np.random.PCG64(731));a=ndimage.gaussian_filter(rng.standard_normal((32,32,32)).astype(np.float32),1.5,mode='wrap');a=(a-a.min())/(a.max()-a.min());a.astype('<f2').tofile(p)
 (R/'channels-32.json').write_text(json.dumps({'file':p.name,'sha256':sha(p),'dimensions_xyz':[32]*3,'storage':'little-endian binary16 Xfastest numpy[Z,Y,X]','seed':731,'generator':'numpy.PCG64 standard_normal -> scipy.gaussian_filter sigma1.5 modewrap -> minmax[0,1] -> binary16','sampling':'textureSampleLevel with clamp-to-edge linear; coordinate=uv*32-.5; M=.3+.7*N, no time offset','tool':{'numpy':np.__version__,'scipy':scipy.__version__}},indent=2)+'\n')
 return np.fromfile(p,dtype='<f2').reshape(32,32,32).astype(np.float32)

N=noise();structure=ndimage.generate_binary_structure(3,1)
def topology(values,positions,n,pitch):
 q=np.zeros(n**3,np.float32);q[positions]=values;q=q.reshape(n,n,n);peak=float(values.max(initial=0));out={}
 minvox=max(8,int(np.ceil((64/pitch)**3)))
 for fraction in [0,.1,.3,.5]:
  threshold=peak*fraction;mask=q>threshold if fraction==0 else q>=threshold
  if peak<=0:mask.fill(False)
  labels,count=ndimage.label(mask,structure=structure);sizes=np.bincount(labels.ravel())[1:];sub=sizes[sizes>=minvox]
  out[str(fraction)]={'threshold':threshold,'positive_voxels':int(mask.sum()),'components':int(count),'substantial_components':int(len(sub)),'minimum_substantial_voxels':minvox,'minimum_substantial_volume_m3':minvox*pitch**3,'largest_volume_m3':(np.sort(sub)[-8:][::-1]*pitch**3).tolist()}
  del labels,mask,sizes,sub
 del q;gc.collect();return out

def run(n,iso):
 src=S/f'cloud-03-{n}.density.r16f';dpath=R/f'cloud-03-{n}-iso{str(iso).replace(".","p")}.distance.r16f';dm=json.loads(dpath.with_suffix('.json').read_text())
 raw=np.fromfile(src,dtype='<f2');positions=np.flatnonzero(raw>0);rho=raw[positions].astype(np.float32);del raw
 distance=np.fromfile(dpath,dtype='<f2');d=distance[positions].astype(np.float32)*4;del distance
 z=positions//(n*n);y=positions//n%n;x=positions%n
 coords=np.vstack([(z+.5)*32/n-.5,(y+.5)*32/n-.5,(x+.5)*32/n-.5]);m=.3+.7*ndimage.map_coordinates(N,coords,order=1,mode='nearest',prefilter=False);m=m.astype(np.float32);del coords,z,y,x
 width=np.float32(32);ref=rho*smooth((d+width)/(2*width));total=float(np.sum(ref,dtype=np.float64));pitch=2048/n
 result={'density_source_sha256':sha(src),'distance_file':dpath.name,'distance_sha256':sha(dpath),'noise_file':'channels-32.r16f','noise_sha256':sha(R/'channels-32.r16f'),'reference_integral':total,'reference_taper_halfwidth_world_m':32,'world_scale':4,'quality':n,'iso_density':iso,'sampling':'CPU native texel centres. Trilinear32noise clamp-to-edge. Float32 local math +float64 integral; GPU query and arbitrary projected/ray matching remain unqualified.','reference_scope':'rho_ref=rho*smoothstep(-32m,32m,d_world), differs from original input; basis for allthreeoperators','channels':{},'selected_topology':{},'tool':{'python':platform.python_version(),'numpy':np.__version__,'scipy':scipy.__version__,'recipe_sha256':sha(Path(__file__))}}
 # Same requested exposure grids and exact extinction endpoints; max2m interpolation intervals.
 probes=np.argsort(d)[-5:];result['interior_probe_positions_flat']=positions[probes].tolist();result['interior_probe_initial_density']=ref[probes].tolist()
 for channel,M in [('uniform',np.ones_like(m)),('noisy',m)]:
  emin=.3 if channel=='noisy' else 1.;emax=(float(d.max())+32+8)/emin
  exposures=np.unique(np.r_[np.arange(0,emax,2),[0,10,20,30,40,50,60,80,100,130,180,emax]])
  exposures=exposures[exposures<=emax]
  order=np.argsort(ref/M);ratios=(ref/M)[order].astype(np.float64);sorted_ref=ref[order].astype(np.float64);sorted_m=M[order].astype(np.float64)
  suffix_ref=np.cumsum(sorted_ref[::-1],dtype=np.float64)[::-1];suffix_m=np.cumsum(sorted_m[::-1],dtype=np.float64)[::-1]
  mass_at=np.maximum(0,suffix_ref-ratios*suffix_m)
  e_values=[];thresholds=[];alphas=[];integrals=[];observed=[]
  for e in exposures:
   q=rho*smooth((d-np.float32(e)*M+width)/(2*width));mass=float(np.sum(q,dtype=np.float64))
   threshold=float(np.interp(mass,mass_at[::-1],ratios[::-1])) if mass>0 else float(ratios[-1])+1e-7
   subtract=np.maximum(ref-np.float32(threshold)*M,0);sub_mass=float(np.sum(subtract,dtype=np.float64))
   alpha=mass/total
   e_values.append(float(e));thresholds.append(threshold);alphas.append(alpha);integrals.append(mass)
   if e in [0,10,20,40,60,100] or e==emax:
    checks={'exposure_world_m':float(e),'sdf_integral':mass,'matched_subtraction_integral':sub_mass,'matched_alpha':alpha,'relative_subtraction_integral_error':abs(sub_mass-mass)/max(total,1e-20),'sdf_peak':float(q.max(initial=0)),'sdf_probe_values':q[probes].tolist(),'alpha_probe_values':(ref[probes]*alpha).tolist()}
    # Keep the costly topology observer on128 only;256 remains actual-render/GPU validation work.
    if n==128:
     checks['sdf_topology']=topology(q,positions,n,pitch);checks['alpha_topology']=topology(ref*np.float32(alpha),positions,n,pitch);checks['subtraction_topology']=topology(subtract,positions,n,pitch)
    observed.append(checks)
  assert integrals[-1]==0,(channel,integrals[-1]);assert max(abs(v['relative_subtraction_integral_error']) for v in observed)<1e-5
  result['channels'][channel]={'exposure_world_m':e_values,'rho_threshold':thresholds,'alpha':alphas,'target_integral':integrals,'full_extinction_exposure_world_m':emax,'minimum_multiplier':emin,'interpolation':'piecewise linear in exposure, max2m spacing; exact endpoints retained; native-integral matching approximate betweenknots'}
  result['selected_topology'][channel]=observed
 out=R/f'cloud-03-{n}-iso{str(iso).replace(".","p")}.curves.json';out.write_text(json.dumps(result,indent=2)+'\n');print(out.name,'samples',len(e_values),'bytes',out.stat().st_size,flush=True)
 del rho,d,m,ref,positions;gc.collect()

for n in [128,256]:
 for iso in [.001,.005]:run(n,iso)
