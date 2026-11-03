from pathlib import Path
import json,hashlib,time,resource,math,sys
import numpy as np
from scipy.ndimage import label
R=Path(__file__).resolve().parent;T=R.parent;START=time.perf_counter();O=np.array([112.,224.,144.]);SH=(72,64,96);ROOT=np.array([225.,265.,197.]);sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
rho=np.memmap(R/'density.r16f',dtype='<f2',mode='r',shape=SH);d=np.memmap(R/'distance.r16f',dtype='<f2',mode='r',shape=SH);a=rho.astype(np.float32);inside=a>=np.float32(.001);cert=json.loads((R/'certificates.json').read_text());field=json.loads((R/'field.json').read_text());assert sha(R/'distance.r16f')==field['distance_sha256']
assert all(np.all(x==0)for x in[rho[0],rho[-1],rho[:,0],rho[:,-1],rho[:,:,0],rho[:,:,-1]])
def smooth(v):
 u=np.clip((v+8)/16,0,1);return u*u*(3-2*u)
def point(x):
 t=(x-O)/2-.5;b=np.floor(t).astype(int);f=t-b;rr=dd=0.
 for z in(0,1):
  for y in(0,1):
   for xx in(0,1):
    ij=np.clip(b+np.array([xx,y,z]),0,np.array(SH[::-1])-1);w=(f[0]if xx else 1-f[0])*(f[1]if y else 1-f[1])*(f[2]if z else 1-f[2]);rr+=float(rho[tuple(ij[::-1])])*w;dd+=float(d[tuple(ij[::-1])])*w
 return rr*float(smooth(dd))
# Independent SciPy6-connectivity instead of producer's explicit queue.
safe=np.memmap(R/'safe-radius.f32',dtype='<f4',mode='r',shape=SH);good=np.asarray(safe>=6);rootidx=tuple(np.rint((ROOT-O-1)/2).astype(int)[::-1]);good[rootidx]=True;labels,n=label(good);reachable=labels==labels[rootidx];assert np.array_equal(reachable,np.memmap(R/'reachable.u8',dtype='u1',mode='r',shape=SH).astype(bool))
S=np.memmap(T/'cloud-natural-crown-v1/dS.f64',dtype='<f8',mode='r',shape=SH)==0;assert S.sum()==19208
master=np.memmap(T/'cloud-authored-master/cloud-03-256.density.r16f',dtype='<f2',mode='r',shape=(256,256,256));raw=master[72:144,112:176,56:152];iz,iy,ix=np.nonzero(S);coords=O+1+2*np.column_stack([ix,iy,iz]);weights=raw[iz,iy,ix].astype(float);total=math.fsum(weights);centroid=np.array([math.fsum(weights*coords[:,k])/total for k in range(3)]);assert np.max(np.abs(centroid-cert['original_S_raw_rho_weighted_centroid']))<1e-10
eligible=reachable[iz,iy,ix];ec=coords[eligible];dd=((ec-centroid)**2).sum(axis=1);order=np.lexsort((ec[:,2],ec[:,1],ec[:,0],dd));assert np.array_equal(ec[order[0]],cert['headWitnesses'][0]['centerLocal'])
# Recompute actual closed-cell bounds directly from eight decoded field corners.
sphereCount=0;surfaceCount=0;minimumSurface=1.;minimumLower=1.;minimumOverlap=1e9
for h in cert['headWitnesses']:
 balls=h['rootChain'];assert len(balls)<=16 and balls[0]['centerLocal']==ROOT.tolist()and balls[-1]['centerLocal']==h['centerLocal']
 for bi,ball in enumerate(balls):
  c=np.array(ball['centerLocal']);r=ball['radiusLocal'];assert r>=6
  low=np.maximum(0,np.floor((c-r-O-3)/2).astype(int));high=np.minimum(np.array(SH[::-1])-2,np.ceil((c+r-O-1)/2).astype(int));xyz=np.stack(np.meshgrid(np.arange(low[0],high[0]+1),np.arange(low[1],high[1]+1),np.arange(low[2],high[2]+1),indexing='ij'),axis=-1).reshape(-1,3);p=O+1+2*xyz;delta=np.maximum(np.maximum(p-c,c-p-2),0);xyz=xyz[(delta*delta).sum(axis=1)<=r*r];mnR=np.full(len(xyz),np.inf);mnD=mnR.copy()
  for z in(0,1):
   for y in(0,1):
    for x in(0,1):
     k=xyz+np.array([x,y,z]);idx=tuple(k[:,::-1].T);mnR=np.minimum(mnR,rho[idx].astype(float));mnD=np.minimum(mnD,d[idx].astype(float))
  lower=mnR*smooth(mnD)-2e-12;assert lower.min()>=.041+1e-6;minimumLower=min(minimumLower,float(lower.min()));sphereCount+=1
  for j in range(96):
   z=1-2*(j+.5)/96;theta=j*(math.pi*(3-math.sqrt(5)));v=np.array([math.sqrt(1-z*z)*math.cos(theta),math.sqrt(1-z*z)*math.sin(theta),z]);q=point(c+r*v);minimumSurface=min(minimumSurface,q);surfaceCount+=1;assert q>=.041
  if bi:
   prev=balls[bi-1];overlap=prev['radiusLocal']+r-float(np.linalg.norm(c-prev['centerLocal']));assert overlap>0;minimumOverlap=min(minimumOverlap,overlap)
# Independent finite nearest-opposite-class oracle, streamed by native planes.
pts=[rootidx]+[tuple(np.rint((np.array(h['centerLocal'])-O-1)/2).astype(int)[::-1])for h in cert['headWitnesses']]+[((j*17)%72,(j*29)%64,(j*47)%96)for j in range(16)];i64=np.memmap(R/'inside.f64',dtype='<f8',mode='r',shape=SH);o64=np.memmap(R/'outside.f64',dtype='<f8',mode='r',shape=SH);oracles=[]
for z,y,x in pts:
 target=~inside if inside[z,y,x]else inside;best=np.inf
 for zz in range(72):
  yy,xx=np.nonzero(target[zz]);v=(xx-x)**2+(yy-y)**2+(zz-z)**2
  if len(v):best=min(best,int(v.min()))
 exact=math.sqrt(best)*2*(1 if inside[z,y,x]else-1);stored64=float(i64[z,y,x]-o64[z,y,x]);assert abs(stored64-exact)<1e-12;oracles.append({'indexZYX':[int(z),int(y),int(x)],'bruteSignedDistance':exact,'EDT64':stored64,'R16':float(d[z,y,x])})
F=np.array(cert['root_to_crown_frame']['root_to_crown_orthoframe_columns']).T;Y=np.array(cert['headWitnesses'][0]['centerLocal'])-ROOT;Y/=np.linalg.norm(Y);assert np.max(np.abs(F[:,1]-Y))<1e-12 and np.max(np.abs(F.T@F-np.eye(3)))<1e-12 and np.linalg.det(F)>1-1e-12
# Conservative support bounds: maxrho>0 ANDmaxd>-8 over every interpolationcell.
bounds=field['strict_positive_trilinear_bounds_rho_q'][1];potentialCount=0
for z in range(71):
 mr=np.zeros((63,95));md=np.full((63,95),-np.inf)
 for dz in(0,1):
  for dy in(0,1):
   for dx in(0,1):mr=np.maximum(mr,rho[z+dz,dy:dy+63,dx:dx+95]);md=np.maximum(md,d[z+dz,dy:dy+63,dx:dx+95])
 yy,xx=np.nonzero((mr>0)&(md>-8));potentialCount+=len(xx)
 if len(xx):
  lo=O+1+2*np.array([xx.min(),yy.min(),z]);hi=O+3+2*np.array([xx.max(),yy.max(),z]);assert np.all(lo>=bounds[0])and np.all(hi<=bounds[1])
assert float(d.max())+8<.3*field['endpoint_local_m'];assert not np.any(a*smooth(d.astype(float)-.3*field['endpoint_local_m'])>0)
out={'pass':True,'rho_sha256':sha(R/'density.r16f'),'distance_sha256':sha(R/'distance.r16f'),'certificate_sha256':sha(R/'certificates.json'),'verification_recipe_sha256':sha(__file__),'connectedCentreMaskMatchesIndependentSciPyLabel':True,'principalNearestReachableCentroidSelection':True,'centroidIndependentMaxErrorLocalM':float(np.max(np.abs(centroid-cert['original_S_raw_rho_weighted_centroid']))),'spheresIndependentlyRecomputed':sphereCount,'sphereSurfaceSamples':surfaceCount,'minimumSurfaceQ':minimumSurface,'minimumCellLower':minimumLower,'minimumChainOverlapLocalM':minimumOverlap,'nativeDistanceOracles':oracles,'strictPositivePotentialCellsChecked':potentialCount,'endpointContinuousBoundAndNativeZero':True,'frameRightHandedAndDataDerived':True,'elapsed_s':time.perf_counter()-START,'process_highwater_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,'scope':'Finite independent native/closed-cell/connectivity/frame checks; ideal half-trilinear field only, not portableGPUfilter/art/admission/lifecycle proof'};(R/'verification.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out)[:600])
