"""ONE pinned offline mask. Global local metres, Float64 geometry/noise; no source runtime."""
import hashlib,math
import numpy as np
ROOT=np.array([225.,265.,197.]);HEAD=np.array([221.,295.,207.]);L=float(np.linalg.norm(HEAD-ROOT));E=(HEAD-ROOT)/L;K=7/L;H=math.sqrt(1-K*K)
P0_BOUNDS=(np.array([181.,225.,157.]),np.array([265.,335.,247.]))
PATH=None

def smooth(a,b,x):
 u=np.clip((x-a)/(b-a),0,1);return u*u*(3-2*u)

def lattice(seed,i):
 text=f'thrum-cloud-midshape-v1\n{seed}\n{i[0]},{i[1]},{i[2]}\n'
 n=int.from_bytes(hashlib.sha256(text.encode('ascii')).digest()[:4],'little')
 return 2*n/(2**32-1)-1

class ValueNoise:
 def __init__(self,size,seed,bounds=P0_BOUNDS):
  self.size=size;self.seed=seed;self.low=np.floor(bounds[0]/size).astype(int)-1;hi=np.floor(bounds[1]/size).astype(int)+2
  self.values=np.empty(tuple(hi-self.low+1),np.float64)
  for x in range(self.values.shape[0]):
   for y in range(self.values.shape[1]):
    for z in range(self.values.shape[2]):self.values[x,y,z]=lattice(seed,(int(x+self.low[0]),int(y+self.low[1]),int(z+self.low[2])))
 def __call__(self,x):
  q=x/self.size;i=np.floor(q).astype(np.int32);f=q-i;w=6*f**5-15*f**4+10*f**3;idx=i-self.low
  if np.any(idx<0)or np.any(idx+1>=np.array(self.values.shape)):raise ValueError('Noise lookup outside fixed lattice')
  out=np.zeros(x.shape[:-1],np.float64)
  for a in [0,1]:
   for b in [0,1]:
    for c in [0,1]:out+=self.values[idx[...,0]+a,idx[...,1]+b,idx[...,2]+c]*(w[...,0]if a else 1-w[...,0])*(w[...,1]if b else 1-w[...,1])*(w[...,2]if c else 1-w[...,2])
  if np.any(out< -1-1e-13)or np.any(out>1+1e-13):raise ValueError('Noise range')
  return out

NOISE=None

def set_path(path):
 global PATH
 PATH=np.asarray(path,np.float64)

def path_distance(x):
 if PATH is None:raise ValueError('Proven path not initialized')
 ds=np.full(x.shape[:-1],np.inf)
 for a,b in zip(PATH[:-1],PATH[1:]):
  d=b-a;v=x-a;t=np.clip(np.sum(v*d,axis=-1)/np.dot(d,d),0,1);vv=v-t[...,None]*d
  ds=np.minimum(ds,np.sum(vv*vv,axis=-1))
 return np.sqrt(ds)

def details(x):
 v=x-ROOT;u=np.sum(v*E,axis=-1);perp=v-u[...,None]*E;h=np.linalg.norm(perp,axis=-1);s=np.clip(u+K*h/H,0,L);c=ROOT+s[...,None]*E;r=33+K*s;rad=x-c;length=np.linalg.norm(rad,axis=-1);d=r-length
 chi=smooth(18,30,path_distance(x));axis=length<1e-12
 if np.any(chi[axis]!=0):raise ValueError('Singular axis not protected')
 p0=c+r[...,None]*rad/np.where(axis,1,length)[...,None]
 p0[axis]=ROOT+np.array([33,0,0])
 return d,p0,chi,s,axis

def d0(x):
 v=x-ROOT;u=np.sum(v*E,axis=-1);h=np.linalg.norm(v-u[...,None]*E,axis=-1);s=np.clip(u+K*h/H,0,L);return 33+K*s-np.linalg.norm(v-s[...,None]*E,axis=-1)

def operator(x):
 global NOISE
 if NOISE is None:NOISE=(ValueNoise(16,18431),ValueNoise(8,93277))
 d,p0,chi,s,axis=details(x);delta=4*NOISE[0](p0)+2*NOISE[1](p0);offset=chi*delta
 if not np.all(np.isfinite(offset))or np.any(np.abs(delta)>6+1e-12):raise ValueError('Invalid delta')
 return {'D0':d,'p0':p0,'chi':chi,'Delta':delta,'offset':offset,'Dnew':d+offset,'s':s,'axis':axis}

class Moments:
 def __init__(self):self.n=0;self.total=0.;self.square=0.;self.lo=float('inf');self.hi=-float('inf')
 def add(self,x):
  if not x.size:return
  self.n+=x.size;self.total+=float(x.sum(dtype=np.float64));self.square+=float(np.sum(x*x,dtype=np.float64));self.lo=min(self.lo,float(x.min()));self.hi=max(self.hi,float(x.max()))
 def json(self):return {'count':self.n,'min':self.lo if self.n else None,'mean':self.total/self.n if self.n else None,'rms':math.sqrt(self.square/self.n)if self.n else None,'max':self.hi if self.n else None}
