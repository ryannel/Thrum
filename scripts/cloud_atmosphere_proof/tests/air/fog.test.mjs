import test from 'node:test';
import assert from 'node:assert/strict';
import {createFogProfile,DEFAULT_FOG_PROFILE,FOG_ENVELOPE,fogPoint,fogSegment} from '../../src/air/fog.js';

function close(x,y,tolerance=1e-9) { assert.ok(Math.abs(x-y)<=tolerance,`${x} != ${y}, tolerance ${tolerance}`); }
function identity(r) {
  assert.equal(r.valid,true,r.reason); assert.equal(r.I_metres,0);
  assert.deepEqual(r.tauRGB,[0,0,0]); assert.deepEqual(r.T_RGB,[1,1,1]);
  assert.deepEqual(r.averageSigmaT,[0,0,0]);
}
function hasIntegral(r,value,tol=1e-9) {
  assert.equal(r.valid,true,r.reason);close(r.I_metres,value,tol);
  assert.ok(Math.abs(r.I_metres-value)<=r.errorI+tol);
}
const base=()=>createFogProfile({centre:[0,0,0],radii:[100,200,300],betaT:[.001,.002,.003],albedo:[.5,.7,.9]});

// Independent original field: explicit matrix transpose and ellipsoid formula,
// never calling fogPoint or the segment's closest-approach/Simpson operator.
function originalDensity(profile,world) {
  const q=world.map((v,i)=>v-profile.centre[i]);
  let norm2=0;
  for(let j=0;j<3;++j) {
    let projection=0;
    for(let k=0;k<3;++k) projection+=profile.rotationColumns[j][k]*q[k];
    norm2+=(projection/profile.radii[j])**2;
  }
  return Math.max(0,1-norm2);
}
function quadrature(profile,o,d,A,B,N=16384) {
  const step=(B-A)/N;
  let sum=0;
  for(let i=0;i<=N;++i) {
    const t=A+i*step;
    const f=originalDensity(profile,o.map((v,k)=>v+d[k]*t));
    sum+=(i===0||i===N?1:i%2?4:2)*f;
  }
  return sum*step/3;
}
const toWorld=(R,x,C=[0,0,0])=>C.map((v,k)=>v+R[0][k]*x[0]+R[1][k]*x[1]+R[2][k]*x[2]);

test('immutable validated profiles own all arrays and the selected envelope',()=>{
  const centre=[0,0,0],R=[[1,0,0],[0,1,0],[0,0,1]];
  const p=createFogProfile({centre,rotationColumns:R});centre[0]=700;R[0][0]=9;
  assert.deepEqual(p.centre,[0,0,0]);assert.equal(p.rotationColumns[0][0],1);
  assert.ok(Object.isFrozen(p)&&Object.isFrozen(p.rotationColumns[0])&&Object.isFrozen(p.betaT));
  assert.throws(()=>{p.radii[0]=42;},TypeError);
  assert.equal(DEFAULT_FOG_PROFILE.g,.3);assert.equal(FOG_ENVELOPE.maxTauError,1e-5);
});

test('axis full, half and offset chords match separate closed forms',()=>{
  const p=base();
  hasIntegral(fogSegment(p,[-200,0,0],[1,0,0],0,400),4*100/3);
  hasIntegral(fogSegment(p,[-200,0,0],[1,0,0],200,400),2*100/3);
  const delta=1-.5**2;
  hasIntegral(fogSegment(p,[-200,100,0],[1,0,0],0,400),4*100*delta**1.5/3);
  hasIntegral(fogSegment(p,[0,-400,0],[0,1,0],0,800),4*200/3);
});

test('clipped cell averages divide by the requested cell, not fog chord',()=>{
  const p=base(),r=fogSegment(p,[-200,0,0],[1,0,0],0,400);
  hasIntegral(r,400/3);assert.deepEqual(r.interval,[100,300]);
  for(let c=0;c<3;++c) {
    close(r.tauRGB[c],p.betaT[c]*400/3);
    close(r.T_RGB[c],Math.exp(-r.tauRGB[c]));
    close(r.averageSigmaT[c],p.betaT[c]/3);
    close(r.averageSigmaS[c],p.betaS[c]/3);
  }
  // Primitive integral on t=[150,225], x=[-50,25].
  const primitive=x=>x-x**3/(3*100**2);
  hasIntegral(fogSegment(p,[-200,0,0],[1,0,0],150,225),primitive(25)-primitive(-50));
});

test('outside, zero length and tangency return explicit identity with honest error',()=>{
  const p=base();
  identity(fogSegment(p,[0,300,0],[1,0,0],0,400));
  identity(fogSegment(p,[-200,0,0],[1,0,0],0,50));
  identity(fogSegment(p,[0,0,0],[1,0,0],2,2));
  const tangent=fogSegment(p,[-200,200,0],[1,0,0],0,400);
  identity(tangent);assert.ok(tangent.reason.includes('tangent'));
  assert.ok(tangent.errorI*.003<=1e-5);
  const barelyInside=fogSegment(p,[-200,200-1e-12,0],[1,0,0],0,400);
  assert.equal(barelyInside.valid,true,barelyInside.reason);
  assert.ok(barelyInside.errorI*.003<=1e-5);
  const missed=4*100*(1-((200-1e-12)/200)**2)**1.5/3;
  assert.ok(missed<=barelyInside.I_metres+barelyInside.errorI);
});

test('rotation and translation covary with an anisotropic ellipsoid',()=>{
  const a=.71,b=-.43,ca=Math.cos(a),sa=Math.sin(a),cb=Math.cos(b),sb=Math.sin(b);
  const R=[[ca*cb,sa*cb,-sb],[-sa,ca,0],[ca*sb,sa*sb,cb]],C=[-1400,120,900];
  const p=base(),rotated=createFogProfile({centre:C,radii:p.radii,rotationColumns:R,betaT:p.betaT,albedo:p.albedo});
  const o=[-200,60,50],d=[1,0,0],ref=fogSegment(p,o,d,0,400);
  const out=fogSegment(rotated,toWorld(R,o,C),toWorld(R,d),0,400);
  assert.equal(out.valid,true,out.reason);close(out.I_metres,ref.I_metres,2e-10);
  close(fogPoint(rotated,toWorld(R,[20,60,50],C)).f,fogPoint(p,[20,60,50]).f,1e-13);
});

test('tiny requested segments and far encoded origins stay within reported error',()=>{
  const p=base();
  const tiny=fogSegment(p,[0,0,0],[1,0,0],0,1e-8);
  hasIntegral(tiny,1e-8,1e-20);
  const far=fogSegment(p,[1e8,100,0],[-1,0,0],1e8-200,1e8+200);
  hasIntegral(far,4*100*.75**1.5/3,2e-8);
  assert.ok(Math.abs(far.I_metres-4*100*.75**1.5/3)<=far.errorI+1e-10);
});

test('original-field quadrature independently checks clipped and oblique rays',()=>{
  const p=base(),d=[.6,.8,0];
  const cases=[[[ -160,-180,30],d,0,500],[[0,0,0],d,0,500],
    [[-200,100,0],[1,0,0],30,350],[[10,-400,150],[0,1,0],10,790]];
  for(const [o,dir,A,B] of cases) {
    const r=fogSegment(p,o,dir,A,B);assert.equal(r.valid,true,r.reason);
    const expected=quadrature(p,o,dir,A,B);
    close(r.I_metres,expected,2e-6);
  }
});

test('point coefficients and zero density retain geometry without forged source',()=>{
  const p=base(),c=fogPoint(p,[0,0,0]);assert.equal(c.valid,true);assert.equal(c.f,1);
  assert.deepEqual(c.sigma_tRGB,[.001,.002,.003]);
  close(fogPoint(p,[50,0,0]).f,.75);
  assert.equal(fogPoint(p,[200,0,0]).f,0);
  const zero=createFogProfile({centre:[0,0,0],radii:[100,200,300],betaT:0});
  const r=fogSegment(zero,[-200,0,0],[1,0,0],0,400);
  hasIntegral(r,400/3);assert.deepEqual(r.T_RGB,[1,1,1]);
});

test('invalid profile, direction, range and coordinate never manufacture clear fog',()=>{
  for(const input of [[],new Date(),{radii:[0,100,100]},{radii:[24,100,100]},{betaT:[.001,-1,0]},
    {albedo:1.1},{g:1},{rotationColumns:[[1,0,0],[1,0,0],[0,0,1]]},
    {rotationColumns:[[-1,0,0],[0,1,0],[0,0,1]]},{centre:[NaN,0,0]},{beta_t:.001}]) {
    assert.throws(()=>createFogProfile(input));
  }
  const p=base();
  for(const r of [fogSegment({},[0,0,0],[1,0,0],0,1),fogSegment(p,[0,0,0],[2,0,0],0,1),
    fogSegment(p,[0,0,0],[NaN,0,0],0,1),fogSegment(p,[0,0,0],[1,0,0],2,1),
    fogSegment(p,[0,0,0],[1,0,0],0,Infinity),fogSegment(p,[1e10,0,0],[1,0,0],0,1)]) {
    assert.equal(r.valid,false);assert.equal(r.T_RGB,null);assert.equal(r.I_metres,null);
  }
  assert.equal(fogPoint(p,[Infinity,0,0]).valid,false);
});

test('all profile vectors and rotation levels reject holes, including inherited indices',()=>{
  for(const [key,dense] of Object.entries({centre:[0,0,0],radii:[100,200,300],betaT:[.001,.002,.003],albedo:[.5,.7,.9]})) {
    assert.throws(()=>createFogProfile({[key]:Array(3)}),RangeError,key);
    for(let i=0;i<3;++i) {
      const sparse=[...dense];delete sparse[i];
      assert.throws(()=>createFogProfile({[key]:sparse}),RangeError,`${key}[${i}]`);
      const inherited=[...dense];delete inherited[i];
      const proto=Object.create(Array.prototype);proto[i]=dense[i];Object.setPrototypeOf(inherited,proto);
      assert.throws(()=>createFogProfile({[key]:inherited}),RangeError,`${key} inherited[${i}]`);
    }
  }
  assert.throws(()=>createFogProfile({rotationColumns:Array(3)}),RangeError);
  for(let i=0;i<3;++i) {
    const columns=[[1,0,0],[0,1,0],[0,0,1]];delete columns[i];
    assert.throws(()=>createFogProfile({rotationColumns:columns}),RangeError);
    for(let j=0;j<3;++j) {
      const inner=[[1,0,0],[0,1,0],[0,0,1]];delete inner[i][j];
      assert.throws(()=>createFogProfile({rotationColumns:inner}),RangeError);
    }
  }
});

test('sparse point, origin and direction queries return invalid without throwing',()=>{
  const p=base();
  const variants=[Array(3),...[0,1,2].map(i=>{const v=[0,0,0];delete v[i];return v;})];
  for(const sparse of variants) {
    const point=fogPoint(p,sparse);assert.equal(point.valid,false);
    for(const result of [fogSegment(p,sparse,[1,0,0],0,400),fogSegment(p,[0,0,0],sparse,0,400)]) {
      assert.equal(result.valid,false);assert.equal(result.T_RGB,null);assert.equal(result.I_metres,null);
    }
  }
});

test('numeric ownership ignores caller iterators/methods and snapshots getters once',()=>{
  const centre=[0,0,0],radii=[100,200,300],betaT=[.001,.002,.003];
  for(const a of [centre,radii,betaT]) {
    a[Symbol.iterator]=()=>{throw new Error('caller iterator must not run');};
    a.every=()=>true;a.some=()=>false;a.map=()=>{throw new Error('caller map must not run');};
  }
  const p=createFogProfile({centre,radii,betaT});
  assert.deepEqual(p.centre,[0,0,0]);assert.deepEqual(p.radii,[100,200,300]);
  assert.deepEqual(p.betaT,[.001,.002,.003]);
  const origin=[-200,0,0];let reads=0;
  Object.defineProperty(origin,0,{get(){reads++;return reads===1?-200:NaN;}});
  hasIntegral(fogSegment(p,origin,[1,0,0],0,400),400/3);assert.equal(reads,1);
  const point=[0,0,0];let pointReads=0;
  Object.defineProperty(point,1,{get(){pointReads++;return pointReads===1?0:NaN;}});
  assert.equal(fogPoint(p,point).f,1);assert.equal(pointReads,1);
  const throwing=[0,0,0];Object.defineProperty(throwing,0,{get(){throw new Error('bad component');}});
  assert.equal(fogPoint(p,throwing).valid,false);
  assert.equal(fogSegment(p,throwing,[1,0,0],0,1).valid,false);
});
