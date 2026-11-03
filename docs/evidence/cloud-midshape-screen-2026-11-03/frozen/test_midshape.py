import unittest,math,hashlib,struct,json
import numpy as np
import midshape as M
from pathlib import Path
PATH=json.loads((Path(__file__).resolve().parent.parent/'cloud-component-content-v2/crown-b.json').read_text())['path_certificate']['vertices'];M.set_path([p['point_global_xyz_local_m']for p in PATH])

def scalar_hash(seed,i):
 digest=hashlib.sha256(('thrum-cloud-midshape-v1\n%d\n%d,%d,%d\n'%(seed,*i)).encode()).digest();return 2*struct.unpack('<I',digest[:4])[0]/4294967295-1

def scalar_noise(x,size,seed):
 q=[v/size for v in x];i=[math.floor(v)for v in q];f=[v-j for v,j in zip(q,i)];w=[t*t*t*(t*(6*t-15)+10)for t in f];ans=0
 for a in [0,1]:
  for b in [0,1]:
   for c in [0,1]:ans+=scalar_hash(seed,[i[0]+a,i[1]+b,i[2]+c])*(w[0]if a else 1-w[0])*(w[1]if b else 1-w[1])*(w[2]if c else 1-w[2])
 return ans

class MidshapeTest(unittest.TestCase):
 def test_hash_tensor(self):
  for seed,size in [(18431,16),(93277,8)]:
   N=M.ValueNoise(size,seed,bounds=(np.array([-64]*3),np.array([64]*3)))
   for i in [(0,0,0),(-3,2,-1),(2,1,4)]:self.assertEqual(M.lattice(seed,i),scalar_hash(seed,i));self.assertEqual(float(N(np.array(i,dtype=float)*size)),M.lattice(seed,i))
   points=np.array([[.125,-3.5,19.75],[-31.125,-17.75,9.125],[18.25,31.75,-8.5]])
   np.testing.assert_allclose(N(points),[scalar_noise(p,size,seed)for p in points],rtol=0,atol=3e-14)
 def test_active_scalar_envelope_and_projection(self):
  points=np.array([[225,265,197],[221,295,207],[260,285,190],[190,240,180],[200,318,214]],float)
  tt=np.linspace(0,M.L,100001);centres=M.ROOT+tt[:,None]*M.E
  for x in points:
   expected=np.max(33+M.K*tt-np.linalg.norm(x-centres,axis=1));self.assertLess(abs(float(M.d0(x))-expected),2e-5)
  x=np.array([[250,280,198],[190,290,210],[230,322,220]],float);d,p0,chi,s,axis=M.details(x)
  np.testing.assert_allclose(M.d0(p0),0,atol=5e-13,rtol=0)
 def test_axis_protection(self):
  x=M.ROOT+np.linspace(0,M.L,19)[:,None]*M.E;o=M.operator(x)
  self.assertTrue(np.all(o['chi']==0));self.assertTrue(np.all(np.isfinite(o['p0'])));self.assertTrue(np.all(o['offset']==0));self.assertGreater(np.count_nonzero(o['axis']),0)
 def test_seams_and_radial_bound(self):
  U=np.cross(M.E,[0,0,1]);U/=np.linalg.norm(U);V=np.cross(M.E,U)
  for angle in np.arange(12)*2*math.pi/12:
   t=math.cos(angle)*U+math.sin(angle)*V;n=-M.K*M.E+M.H*t
   for c,r in [(M.ROOT,33),(M.HEAD,40)]:
    seam=c+r*n;xs=np.array([seam-1e-7*M.E,seam,seam+1e-7*M.E]);out=M.operator(xs)
    self.assertLess(float(np.ptp(out['Dnew'])),1e-6);self.assertLess(float(np.max(np.linalg.norm(np.diff(out['p0'],axis=0),axis=1))),1e-6)
   for ss,nn in [(0,-M.E),(M.L,M.E),(M.L*.2,n),(M.L*.7,n)]:
    c=M.ROOT+ss*M.E;r=33+M.K*ss;vv=np.linspace(r-12,r+14,161);xs=c+vv[:,None]*nn;out=M.operator(xs)
    np.testing.assert_allclose(out['s'],ss,atol=2e-12,rtol=0);self.assertLess(float(np.max(np.linalg.norm(out['p0']-out['p0'][0],axis=1))),1e-10)
    self.assertGreaterEqual(float(np.min(-np.diff(out['Dnew'])/np.diff(vv))),.25-1e-10)
if __name__=='__main__':unittest.main()
