from pathlib import Path
import argparse,json,hashlib,gc,platform
import numpy as np
import scipy
from scipy import ndimage
ROOT=Path(__file__).resolve().parent
INPUT=ROOT.parent/'cloud-authored-master'

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def smooth(x):
 t=np.clip(x,0,1);return t*t*(3-2*t)

def validate_operator():
 # Axis-aligned half-space: exact EDT to opposite sample centres. Interface lies halfway between them.
 b=np.zeros((16,16,16),bool);b[:,:,:8]=True
 d=ndimage.distance_transform_edt(b)-ndimage.distance_transform_edt(~b)
 assert np.array_equal(d[8,8,:],np.r_[np.arange(8,0,-1),-np.arange(1,9)])
 return {'half_space_center_distance_exact':True,'definition':'signed distance to nearest opposite-class voxel centre; not exact continuous rho-isosurface SDF','interface_error_scope':'Binary classifier and centre EDT have up to voxel-scale surface/normal error; two-grid actual output required'}

def cook(which,n,iso):
 src=INPUT/f'cloud-{which}-{n}.density.r16f';meta=json.loads((INPUT/f'cloud-{which}-{n}.json').read_text())
 assert sha(src)==meta['output']['sha256']
 rho=np.fromfile(src,dtype='<f2').reshape((n,n,n)).astype(np.float32)
 pitch=512/n;world_scale=4.;w_world=32.;w=w_world/world_scale
 mask=rho>=iso
 inside=ndimage.distance_transform_edt(mask,sampling=pitch).astype(np.float32)
 outside=ndimage.distance_transform_edt(~mask,sampling=pitch).astype(np.float32)
 inside-=outside;del outside;gc.collect()
 raw=inside.astype('<f2');dst=ROOT/f'cloud-{which}-{n}-iso{str(iso).replace(".","p")}.distance.r16f';raw.tofile(dst)
 decoded=raw.astype(np.float32);ref=rho*smooth((decoded+w)/(2*w))
 sumrho=float(np.sum(rho,dtype=np.float64));sumref=float(np.sum(ref,dtype=np.float64))
 # Density-tail and distance-ramp losses are reported separately; neither is zero by definition.
 tail=float(np.sum(rho[~mask],dtype=np.float64))/sumrho
 maxd=float(decoded.max())*world_scale
 emax=(maxd+w_world+8.)/.3
 # Native lattice endpoints are authoritative for this numerical check, not a continuous-filter guarantee.
 zero=rho*smooth((decoded-(emax*.3/world_scale)+w)/(2*w))
 assert np.count_nonzero(zero)==0
 record={'source_file':str(src.relative_to(ROOT.parent)),'source_sha256':sha(src),'output':{'file':dst.name,'sha256':sha(dst),'bytes':dst.stat().st_size,'dimensions_xyz':[n,n,n],'storage':'little-endian binary16, Xfastest, numpy[Z,Y,X]','distance_units':'authored local metres; multiply by world scale4 for the current fixture'},'iso_density':iso,'classification':'rho>=iso','distance_definition':'EDT(mask)-EDT(~mask), distances to nearest opposite-class voxel centres, spacing512/n','scipy_reference':'https://docs.scipy.org/doc/scipy/reference/generated/scipy.ndimage.distance_transform_edt.html','pitch_local_m':pitch,'world_scale':world_scale,'reference_taper_halfwidth_world_m':w_world,'source_integral':sumrho,'tapered_reference_integral':sumref,'relative_reference_integral_loss':1-sumref/sumrho,'relative_sub_iso_density_integral':tail,'distance_max_inside_world_m':maxd,'distance_min_world_m':float(decoded.min())*world_scale,'half_distance_max_abs_rounding_local_m':float(np.max(np.abs(inside-decoded))),'minimum_noise_multiplier':.3,'full_extinction_exposure_world_m':emax,'endpoint_nonzero_voxels':int(np.count_nonzero(zero)),'endpoint_definition':'E=e*M with M>=.3, K>=0; emax=(max decoded d_world+w_world+8m)/.3','scope':'Distance to sampled binary boundary; no claim exact original density isosurface normals, constant noisy-front speed, lifecycle appearance or target budget','tool':{'python':platform.python_version(),'numpy':np.__version__,'scipy':scipy.__version__,'recipe_sha256':sha(Path(__file__))},'operator_reference':validate_operator()}
 out=dst.with_suffix('.json');out.write_text(json.dumps(record,indent=2)+'\n')
 print(which,n,iso,'maxd',maxd,'referenceLoss',record['relative_reference_integral_loss'],'tail',tail,'emax',emax,flush=True)
 del rho,mask,inside,raw,decoded,ref,zero;gc.collect()
 return record

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--master',default='03');p.add_argument('--sizes',nargs='+',type=int,default=[128,256]);a=p.parse_args()
 records=[cook(a.master,n,iso) for n in a.sizes for iso in [.001,.005]]
 (ROOT/f'cloud-{a.master}-distance-index.json').write_text(json.dumps(records,indent=2)+'\n')
