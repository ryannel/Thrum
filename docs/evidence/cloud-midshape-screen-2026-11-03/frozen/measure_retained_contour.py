"""Recover already-painted .041 pixel contours from the sole completed contact; no native field work."""
from pathlib import Path
import json,hashlib,math
from PIL import Image
import numpy as np
R=Path(__file__).resolve().parent;p=R/'isolated-front.png';W,H=512,288
with Image.open(p)as im:
 edges=[]
 for j in [1,2]:
  a=np.array(im.crop((j*W*2,40,(j+1)*W*2,40+H*2)))[::2,::2,:3]
  edges.append(np.all(a==[255,150,40],axis=-1))
A,B=[np.column_stack(np.nonzero(x)).astype(float)for x in edges]
def closest(X,Y):
 out=[]
 for i in range(0,len(X),32):
  v=X[i:i+32,None]-Y[None];out.extend(np.sqrt(np.sum(v*v,axis=-1).min(axis=1)).tolist())
 return np.array(out)
def stats(a):return {'count':len(a),'mean_pixels':float(a.mean()),'rms_pixels':float(np.sqrt(np.mean(a*a))),'max_pixels':float(a.max()),'p95_pixels':float(np.quantile(a,.95))}
a,b=closest(A,B),closest(B,A);pitch=2*1200*math.tan(math.radians(24))/H
out={'scope':'Sole completed isolated-front .041 projected-peak contour, recovered exactly from orangeRGB255/150/40 on saved contact; undo2xnearest by sampling every2pixels. No new source/ray/native run. Pixel-centre nearest distances; not3Dsurface distance or composite visibility.','source_png_sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'method':'old/new panels1/2 crop y40..616,512x288, fixed1200m/48deg; bounded32point nearest-distance blocks, no EDT','old_to_new':stats(a),'new_to_old':stats(b),'hausdorff_pixels':float(max(a.max(),b.max())),'world_m_per_pixel_at_target_plane':pitch,'hausdorff_world_m_focal_plane':float(max(a.max(),b.max())*pitch),'unavailable':['isolated-side','isolated-oblique','composite220-front','composite220-side','composite220-oblique'],'unavailable_reason':'sole native run stopped at early RSS guard before side rows0'}
(R/'retained-visible-contour.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out))
