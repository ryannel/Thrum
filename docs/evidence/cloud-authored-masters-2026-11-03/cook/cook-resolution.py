"""Cook two selected static CC0 masters; run with Blender's bundled Python, not Blender UI.
Writes Y-up RH XYZ scalar R16F, X fastest; no lifecycle frames or dynamic authority.
"""
import argparse,gc,hashlib,json,pathlib,struct,sys,zlib
import numpy as np
import openvdb
parser=argparse.ArgumentParser();parser.add_argument('--input-root',type=pathlib.Path,required=True);parser.add_argument('--output-root',type=pathlib.Path,required=True);parser.add_argument('--side',type=int,choices=[128,256],default=256);args=parser.parse_args();INPUT=args.input_root.resolve();ROOT=args.output_root.resolve();ROOT.mkdir(parents=True,exist_ok=True)
SIDE=args.side;EXTENT=512.;FILL=.90;PREVIEW_SIGMA=.02

def sha(p):
 h=hashlib.sha256()
 with pathlib.Path(p).open('rb') as f:
  for b in iter(lambda:f.read(1048576),b''):h.update(b)
 return h.hexdigest()

def png(path,image):
 a=np.ascontiguousarray(image,np.uint8);height,width=a.shape[:2]
 def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
 raw=b''.join(b'\0'+row.tobytes() for row in a)
 path.write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>2I5B',width,height,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(raw,6))+chunk(b'IEND',b''))

def panel(values):
 v=np.flipud(np.clip(values,0,1));image=np.rint(np.asarray([14,20,30])+v[...,None]*np.asarray([225,220,215])).astype(np.uint8)
 # Uniform nearest display resampling preserves the represented density/projection, not a beauty filter.
 h,w=v.shape;scale=min(384/w,384/h);hh=max(1,round(h*scale));ww=max(1,round(w*scale))
 ys=np.minimum((np.arange(hh)/scale).astype(int),h-1);xs=np.minimum((np.arange(ww)/scale).astype(int),w-1)
 out=np.full((384,384,3),[14,20,30],np.uint8);out[(384-hh)//2:(384-hh)//2+hh,(384-ww)//2:(384-ww)//2+ww]=image[ys[:,None],xs[None,:]]
 return out

def weights(centers,count,step):
 # Exact overlap with native unit voxel cells, including zero background outside bbox.
 lower=centers[:,None]-step/2;upper=centers[:,None]+step/2
 source=np.arange(count)[None,:]
 return (np.maximum(0,np.minimum(upper,source+.5)-np.maximum(lower,source-.5))/step).astype(np.float32)

def stats(a):
 positive=a[a>0]
 return {'shape':list(a.shape),'nonzero':int(positive.size),'positive_percentiles':dict(zip(['0','1','25','50','75','95','99','99.9','100'],[float(v) for v in np.percentile(positive,[0,1,25,50,75,95,99,99.9,100])])),'sum':float(np.sum(a,dtype=np.float64)),'mean_full_cube':float(np.mean(a,dtype=np.float64)),'mean_positive':float(np.mean(positive,dtype=np.float64))}

rows=[];outputs=[]
for number in (3,8):
 path=INPUT/f'CloudPack/CloudPackVDB/cloud_{number:02d}_variant_0000.vdb';grid=openvdb.read(str(path),'density')
 low,high=[np.asarray(p,np.int32) for p in grid.evalActiveVoxelBoundingBox()];dims=high-low+1
 origin=np.asarray(grid.transform.indexToWorld((0,0,0)),np.float64)
 basis=np.asarray([grid.transform.indexToWorld(tuple(v)) for v in np.eye(3)],np.float64)-origin
 if not np.allclose(basis,np.eye(3),atol=1e-12):raise ValueError('This bounded recipe requires the inspected unit positive XYZ transform')
 a=np.empty(tuple(int(n) for n in dims),np.float32);grid.copyToArray(a,ijk=tuple(int(v) for v in low))
 if not np.isfinite(a).all() or a.min()<0:raise ValueError('Nonfinite or negative source density')
 source_stats=stats(a);maximum=float(a.max());scale=FILL*EXTENT/float(max(dims));center=(low+high)*.5
 v=(np.arange(SIDE)+.5)*EXTENT/SIDE;step=EXTENT/SIDE/scale
 wx=weights((v-EXTENT/2)/scale+center[0]-low[0],int(dims[0]),step)
 wy=weights(center[1]-(v-EXTENT/2)/scale-low[1],int(dims[1]),step)
 wz=weights((v-EXTENT/2)/scale+center[2]-low[2],int(dims[2]),step)
 tmp=np.einsum('ix,xyz->iyz',wx,a,optimize=True)
 mid=np.einsum('jy,iyz->ijz',wy,tmp,optimize=True);del tmp
 q=np.einsum('kz,ijz->jki',wz,mid,optimize=True);del mid
 source_integral=source_stats['sum']*scale**3
 cooked_integral=float(q.sum(dtype=np.float64))*(EXTENT/SIDE)**3
 integral_relative_error=abs(cooked_integral-source_integral)/source_integral
 if integral_relative_error>1e-5:raise ValueError('Conservative scalar cook integral mismatch')
 half=q.astype('<f2');decoded=half.astype(np.float32)
 out=ROOT/f'cloud-{number:02d}-{SIDE}.density.r16f';out.write_bytes(half.tobytes(order='C'))
 faces=[decoded[0],decoded[-1],decoded[:,0],decoded[:,-1],decoded[:,:,0],decoded[:,:,-1]]
 if any(np.any(f) for f in faces):raise ValueError('Expected exact empty collar at all six faces')
 native_tau_front=PREVIEW_SIGMA*a.sum(axis=1,dtype=np.float64)*scale
 native_tau_side=PREVIEW_SIGMA*a.sum(axis=0,dtype=np.float64)*scale
 native_front=(1-np.exp(-native_tau_front)).T
 radiographic={}
 for axis,native_tau,left,coarse in [('front',native_tau_front,wx,decoded.sum(axis=0,dtype=np.float64)),('side',native_tau_side,wy,decoded.sum(axis=2,dtype=np.float64).T)]:
  tau_ref=(left.astype(np.float64)@native_tau@wz.astype(np.float64).T).T
  trans_ref=1-(left.astype(np.float64)@(1-np.exp(-native_tau))@wz.astype(np.float64).T).T
  tau=PREVIEW_SIGMA*coarse*(EXTENT/SIDE)
  radiographic[axis]={'max_abs_tau_error_vs_native_voxel_column_average':float(np.abs(tau-tau_ref).max()),'mean_abs_tau_error_vs_native_voxel_column_average':float(np.abs(tau-tau_ref).mean()),'max_abs_T_error_vs_native_footprint_average':float(np.abs(np.exp(-tau)-trans_ref).max()),'mean_abs_T_error_vs_native_footprint_average':float(np.abs(np.exp(-tau)-trans_ref).mean())}

 max_front=decoded.max(axis=0);front=1-np.exp(-PREVIEW_SIGMA*decoded.sum(axis=0)*(EXTENT/SIDE));side=(1-np.exp(-PREVIEW_SIGMA*decoded.sum(axis=2)*(EXTENT/SIDE))).T
 row=np.concatenate([panel(native_front),panel(max_front),panel(front),panel(side)],axis=1);rows.append(row)
 png(ROOT/f'cloud-{number:02d}-{SIDE}-projections.png',row)
 support=np.argwhere(decoded>0)
 info={'id':f'thrum.cloud-master.jangafx-{number:02d}.static-{SIDE}.v1','source_file':str(path.relative_to(INPUT)),'source_sha256':sha(path),'source_metadata':{k:str(grid[k]) for k in grid},'source_transform':{'index_origin_world_xyz':origin.tolist(),'index_to_world_basis_rows_xyz':basis.tolist()},'source_active_index_bbox_min_xyz':low.tolist(),'source_active_index_bbox_max_xyz':high.tolist(),'source_density_stats':source_stats,'source_grid_memory_bytes':int(grid.memUsage()),'peak_dense_source_bytes':int(a.nbytes),'orientation':{'source_up_interpretation':'Z-up, consistent with source domain proportions/official default export and preview; VDB has no semantic up-axis tag','reference_url':'https://docs.jangafx.com/embergen/pages/references/How-To%20Guides/exportVDB.html','inference_evidence':'Source original domain415x639x223, both selected densities share lowest occupiedZ=11, silhouette/proportions agree with official thumbnail; no embedded semantic up-axis tag','right_handed_target_mapping':'targetXYZ=(sourceX,sourceZ,-sourceY)','center_source_index_xyz':center.tolist()},'authored_geometry':{'extent_m_xyz':[EXTENT]*3,'uniform_metres_per_source_index':scale,'active_extent_target_xyz_m':(dims[[0,2,1]]*scale).tolist(),'center_target_xyz_m':[EXTENT/2]*3,'longest_active_extent_fraction':FILL,'physical_calibration':'Authored512m cube, not a claim about original atmospheric units'},'output':{'file':out.name,'sha256':sha(out),'bytes':out.stat().st_size,'dimensions_xyz':[SIDE]*3,'bounds_min_corner_xyz_m':[0.,0.,0.],'bounds_max_corner_xyz_m':[EXTENT]*3,'voxel_pitch_xyz_m':[EXTENT/SIDE]*3,'index0_center_xyz_m':[EXTENT/SIDE/2]*3,'index_center_to_local':f'localXYZ=(indexXYZ+.5)*{EXTENT/SIDE}m','storage':'little-endian IEEE754binary16 scalar; Xfastest thenYthenZ; numpy[Z,Y,X]','density_normalization_constant':1.,'decode':'q=stored_value=raw native density voxel-average; no clamp, max normalization or explicit threshold','density_stats':stats(decoded),'sampling':'Exact separable overlap-average of native piecewise-constant voxel cells; zero background; not a trilinear-point sample','max_abs_half_rounding_error':float(np.max(np.abs(decoded-q))),'mean_abs_half_rounding_error_full_cube':float(np.mean(np.abs(decoded-q),dtype=np.float64)),'relative_integral_error_before_half':integral_relative_error,'relative_integral_error_after_half':abs(float(decoded.sum(dtype=np.float64))*(EXTENT/SIDE)**3-source_integral)/source_integral,'zero_faces':True,'minimum_exact_zero_collar_voxels':int(min(support.min(),(SIDE-1-support.max(axis=0)).min())),'support_bbox_min_zyx':support.min(axis=0).tolist(),'support_bbox_max_zyx':support.max(axis=0).tolist()},'gpu_contract':'Expand q toRGBA16F as(liquid=q,ice=0,age=0,birth=0); choose and record opticalkappa independently; no wholecloudage envelope or lifecycle playback','preview':{'columns':['native front Beeropacity at mapped voxel size',f'{SIDE}front maximum q',f'{SIDE}front Beeropacity',f'{SIDE}side Beeropacity'],'sigma_per_m':PREVIEW_SIGMA,'radiographic_audit':radiographic,'notes':'Diagnostic projections only; no source shading baked, no diffuse cloud-lighting claim'},'license':'CC0-1.0; retainedLICENSE.txt','recipe':{'file':pathlib.Path(__file__).name,'sha256':sha(__file__),'python':sys.version,'executable':sys.executable,'numpy':np.__version__,'openvdb_module_sha256':sha(openvdb.__file__),'openvdb_module':openvdb.__file__},'limits':['Static Cu ingredient only, not completeCb or accepted dynamicweather lifecycle',f'Conservative voxel-average filtering removes unresolved detail; native-to{SIDE} arbitrary-ray optical/beauty quality remains unqualified','Texture must be sampled in a tight localvolume; depositing into the24kmCb128grid destroys its useful spatialdetail']}
 (ROOT/f'cloud-{number:02d}-{SIDE}.json').write_text(json.dumps(info,indent=2)+'\n');outputs.append(info)
 print('COOKED',number,'source',dims.tolist(),'source_max',maximum,'output',info['output']['density_stats'],'faceszero',True,flush=True)
 del a,grid,q,half,decoded;gc.collect()
png(ROOT/f'projection-contact-{SIDE}.png',np.concatenate(rows,axis=0))
(ROOT/f'cook-index-{SIDE}.json').write_text(json.dumps({'packages':[i['id'] for i in outputs],'projection_row_order':['cloud03','cloud08'],'projection_column_order':outputs[0]['preview']['columns'],'output_hashes':{i['output']['file']:i['output']['sha256'] for i in outputs}},indent=2)+'\n')
