import unittest,json
from pathlib import Path
import numpy as np
import preflight as P
class TestPreflight(unittest.TestCase):
 def test_forward_inverse_stored_frame(self):
  cap=json.loads((P.T/'cloud-system-proof/captures/system-t220000-mode0-c3e0ebee9680337c-eea93588-4.json').read_text())
  points=np.array([[225,265,197],[221,295,207],[200,318,214],[232,296,224]],float)
  for p in cap['source']['parts']:
   np.testing.assert_allclose(P.material(p,P.world(p,points)),points,atol=3e-12,rtol=0)
 def test_capsule_exact_envelope(self):
  a=np.array([225,265,197]);b=np.array([221,295,207]);r0,r1=12,24
  for x in np.array([[225,265,197],[230,271,203],[200,318,214],[232,296,224],[200,240,180]],float):
   tt=np.linspace(0,1,10001);ref=np.max(r0+(r1-r0)*tt-np.linalg.norm(x-a-tt[:,None]*(b-a),axis=1));got=float(P.capsule(x,a,b,r0,r1))
   self.assertGreaterEqual(got+1e-10,ref);self.assertLess(got-ref,1e-4)
 def test_smoothmax_and_sampling(self):
  a=np.array([-9.,0,2,9]);b=np.array([9.,0,3,-9]);q=P.smax(a,b);self.assertTrue(np.all(q>=np.maximum(a,b)));self.assertTrue(np.all(q<=np.maximum(a,b)+.75));self.assertEqual(q[0],9)
  x=np.ones((3,3,3),'<f2')*.125;pts=np.array([[0,0,0],[3,3,3],[1,1,1],[-.01,1,1],[3.01,1,1]])
  np.testing.assert_allclose(P.sample(x,pts,np.zeros(3),np.ones(3)*3),[.125,.125,.125,0,0],atol=0,rtol=0)
if __name__=='__main__':unittest.main()
