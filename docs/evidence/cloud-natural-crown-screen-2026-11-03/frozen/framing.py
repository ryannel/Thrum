#!/usr/bin/env python3
"""One analytic common isolated-camera revision, no trial rendering."""
from pathlib import Path
import hashlib,itertools,json,math,resource,time
import numpy as np
R=Path(__file__).resolve().parent;T=R.parent;ROOT=Path(__file__).resolve().parents[2];t0=time.perf_counter()
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1<<20),b''):h.update(b)
 return h.hexdigest()
native=json.loads((R/'native.json').read_text());candidate=R/native['candidate']['file'];assert sha(candidate)==native['candidate']['sha256']
oldmeta=json.loads((T/'cloud-component-content-v2/crown-b.json').read_text());oldpath=T/'cloud-component-content-v2'/oldmeta['density']['file'];assert sha(oldpath)==oldmeta['density']['sha256']
old=np.memmap(oldpath,dtype='<f2',mode='r',shape=(96,96,96));ii=np.argwhere(old>0);assert len(ii)>0
oldlo=np.array(oldmeta['domain_origin_global_xyz_local_m'],float)+1+2*ii.min(axis=0)[::-1]
oldhi=np.array(oldmeta['domain_origin_global_xyz_local_m'],float)+1+2*ii.max(axis=0)[::-1]
newlo=np.array(native['native']['bbox_native_centres_xyz_m'][0],float);newhi=np.array(native['native']['bbox_native_centres_xyz_m'][1],float)
halo=2*math.sqrt(3);boxes={'old':(oldlo-halo,oldhi+halo),'new':(newlo-halo,newhi+halo)}
caps={};capdir=ROOT/'docs/evidence/cloud-crown-screen-2026-11-03/inputs'
for v,n in [('front',4),('side',5),('oblique',6)]:
 p=capdir/f'system-t220000-mode0-c3e0ebee9680337c-eea93588-{n}.json';caps[v]=(json.loads(p.read_text()),sha(p))
part=next(p for p in caps['front'][0]['source']['parts']if p['ID']==5)
for v,(c,_) in caps.items():assert next(p for p in c['source']['parts']if p['ID']==5)==part
def world(a):
 a=np.asarray(a,float);Q=np.array(part['Qcolumns'],float);rot=(a-np.array(part['root']))@Q
 return np.array(part['C'])+part['b']*rot+np.array(part['K'])*rot[...,1,None]
target=world((np.array(part['root'])+np.array([221,295,207]))/2)
aspect=16/9;tan=math.tan(math.radians(24));fraction=.85;records={};maximum=0.
for view,(cap,_) in caps.items():
 cam=cap['presentation']['camera'];yaw=cam['yaw'];pitch=cam['pitch']
 n=np.array([math.sin(yaw)*math.cos(pitch),math.sin(pitch),math.cos(yaw)*math.cos(pitch)])
 forward=-n;right=np.cross(forward,[0,1,0]);right/=np.linalg.norm(right);up=np.cross(right,forward)
 per={}
 for name,(lo,hi) in boxes.items():
  corners=np.array(list(itertools.product(*[(lo[i],hi[i])for i in range(3)])))
  q=world(corners)-target
  depthshift=q@n;horizontal=q@right;vertical=q@up
  need=np.maximum(depthshift+np.abs(vertical)/(fraction*tan),depthshift+np.abs(horizontal)/(fraction*aspect*tan))
  req=float(np.max(need));maximum=max(maximum,req)
  per[name]={'local_lo_xyz_m':lo.tolist(),'local_hi_xyz_m':hi.tolist(),'world_corner_min_xyz_m':world(corners).min(axis=0).tolist(),'world_corner_max_xyz_m':world(corners).max(axis=0).tolist(),'distance_required_m':req,'controlling_corner_local_xyz_m':corners[int(np.argmax(need))].tolist(),'max_abs_horizontal_m':float(np.max(np.abs(horizontal))),'max_abs_vertical_m':float(np.max(np.abs(vertical))),'max_near_depth_shift_m':float(np.max(depthshift))}
 records[view]={'yaw':yaw,'pitch':pitch,'unit_eye_direction':n.tolist(),'right':right.tolist(),'up':up.tolist(),'comparators':per}
distance=100*math.ceil(max(maximum,1200)/100)
width=32*math.ceil((512*max(1,distance/2000))/32);height=width*9//16
rows=max(1,min(4,16384//(8*width)))
assert rows*width*8<=16384 and width%32==0 and width*9%16==0
old_world_px=2*2000*tan/288;new_world_px=2*distance*tan/height
assert new_world_px<=old_world_px+1e-12
out={'scope':'one analytic common isolated framing revision; no image or art selection','old_failed_isolated_distance_m':1200,'old_failed_border_counts_raw_old_new':[307,0,28],'native_receipt_sha256':sha(R/'native.json'),'old_B_rho_sha256':sha(oldpath),'source_captures_sha256':{v:x[1]for v,x in caps.items()},'map':'v=Q(a-root) with outer-list Qcolumns; world=C+b*v+K*v.y; original source220 ID5','part_id':5,'part_pose':part,'target_world_xyz_m':target.tolist(),'frustum':{'vertical_fov_degrees':48,'aspect':aspect,'per_axis_extent_fraction':fraction},'local_support_boxes_include_2sqrt3_m_halo':True,'per_view':records,'raw_minimum_common_distance_m':maximum,'rounded_common_distance_m':distance,'image_pixels':[width,height],'rows_per_tile':rows,'depth_per_tile':8,'max_query_points_per_tile':rows*width*8,'old_2km512_world_pixel_m':old_world_px,'new_focus_world_pixel_m':new_world_px,'code_sha256':sha(Path(__file__)),'wall_s':time.perf_counter()-t0,'process_highwater_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss}
(R/'framing-rev2.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({'required_m':maximum,'rounded_m':distance,'pixels':[width,height],'rows':rows,'points_tile':rows*width*8,'old_px_m':old_world_px,'new_px_m':new_world_px,'peakMiB':out['process_highwater_bytes']/2**20}))
