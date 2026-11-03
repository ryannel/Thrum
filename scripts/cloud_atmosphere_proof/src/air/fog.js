// Pure binary64 CPU reference. No clock, renderer, world ownership or advection.
const U = Number.EPSILON / 2;
const validatedProfiles = new WeakSet();
const bits = new DataView(new ArrayBuffer(8));
export const FOG_ENVELOPE = Object.freeze({
  minRadius: 25, maxRadius: 8000, maxBetaT: .01,
  minG: -.95, maxG: .95, maxAbsCoordinate: 1e9,
  unitTolerance: 1e-12, rotationTolerance: 1e-12, maxTauError: 1e-5,
  arithmetic: 'binary64 basic operations; sqrt/exp assumed within four ulps',
});
function adjacent(x, up) {
  if (Number.isNaN(x) || x === (up ? Infinity : -Infinity)) return x;
  if (x === 0) return up ? Number.MIN_VALUE : -Number.MIN_VALUE;
  bits.setFloat64(0, x);
  let b = bits.getBigUint64(0);
  b += (x > 0) === up ? 1n : -1n;
  bits.setBigUint64(0, b);
  return bits.getFloat64(0);
}
function widen(x, up, count = 1) {
  for (let i = 0; i < count; ++i) x = adjacent(x, up);
  return x;
}
const exact = x => [x, x];
const add = (x,y) => [adjacent(x[0]+y[0],false),adjacent(x[1]+y[1],true)];
const neg = x => [-x[1],-x[0]];
const sub = (x,y) => add(x,neg(y));
function mul(x,y) {
  const v = [x[0]*y[0],x[0]*y[1],x[1]*y[0],x[1]*y[1]];
  return [adjacent(Math.min(...v),false),adjacent(Math.max(...v),true)];
}
function div(x,y) {
  if (y[0] <= 0 && y[1] >= 0) return [-Infinity,Infinity];
  return mul(x,[adjacent(1/y[1],false),adjacent(1/y[0],true)]);
}
function square(x) {
  return [x[0] <= 0 && x[1] >= 0 ? 0 : adjacent(Math.min(x[0]*x[0],x[1]*x[1]),false),
    adjacent(Math.max(x[0]*x[0],x[1]*x[1]),true)];
}
const dot = (x,y) => x[0]*y[0]+x[1]*y[1]+x[2]*y[2];
const dotI = (x,y) => add(add(mul(x[0],y[0]),mul(x[1],y[1])),mul(x[2],y[2]));
const normI = x => add(add(square(x[0]),square(x[1])),square(x[2]));
const sqrtI = x => [Math.max(0,widen(Math.sqrt(Math.max(0,x[0])),false,4)),
  widen(Math.sqrt(Math.max(0,x[1])),true,4)];
// Snapshot indexed own components once. Array iteration can skip holes or be
// overridden; neither is allowed to establish a validated numeric vector.
function numeric3(x) {
  try {
    if (!Array.isArray(x) || x.length !== 3) return null;
    const values = [];
    for (let i=0;i<3;++i) {
      if (!Object.hasOwn(x,i)) return null;
      const value=x[i];
      if (!Number.isFinite(value)) return null;
      values.push(value);
    }
    return values;
  } catch { return null; }
}
function rotation3(x) {
  try {
    if (!Array.isArray(x) || x.length !== 3) return null;
    const columns=[];
    for (let i=0;i<3;++i) {
      if (!Object.hasOwn(x,i)) return null;
      const column=numeric3(x[i]);
      if (!column) return null;
      columns.push(column);
    }
    return columns;
  } catch { return null; }
}
const vector = x => Object.freeze([x[0],x[1],x[2]]);
function rgb(value, name, max) {
  const v = numeric3(typeof value === 'number' ? [value,value,value] : value);
  if (!v || v.some(x => x < 0 || x > max)) throw new RangeError(`Invalid ${name}`);
  return vector(v);
}
export function createFogProfile(input = {}) {
  const allowed = ['centre','rotationColumns','radii','betaT','albedo','g'];
  if (!input || typeof input !== 'object' || Object.getPrototypeOf(input)!==Object.prototype || Object.keys(input).some(k => !allowed.includes(k))) {
    throw new TypeError('Unknown fog profile field');
  }
  const centre = numeric3(input.centre ?? [-1400,120,900]);
  const radii = numeric3(input.radii ?? [1900,500,1800]);
  const rotation = rotation3(input.rotationColumns ?? [[1,0,0],[0,1,0],[0,0,1]]);
  const g = input.g ?? .3;
  if (!centre || centre.some(x => Math.abs(x)>FOG_ENVELOPE.maxAbsCoordinate)) throw new RangeError('Invalid centre');
  if (!radii || radii.some(x => x<25 || x>8000)) throw new RangeError('Invalid radii');
  if (!rotation) throw new RangeError('Invalid rotation');
  for (let i=0;i<3;++i) for (let j=0;j<3;++j) {
    if (Math.abs(dot(rotation[i],rotation[j])-(i===j?1:0))>FOG_ENVELOPE.rotationTolerance) throw new RangeError('Non-orthonormal rotation');
  }
  const det = dot(rotation[0],[rotation[1][1]*rotation[2][2]-rotation[1][2]*rotation[2][1],
    rotation[1][2]*rotation[2][0]-rotation[1][0]*rotation[2][2],rotation[1][0]*rotation[2][1]-rotation[1][1]*rotation[2][0]]);
  if (Math.abs(det-1)>FOG_ENVELOPE.rotationTolerance) throw new RangeError('Rotation must be right handed');
  if (!Number.isFinite(g) || g<-.95 || g>.95) throw new RangeError('Invalid g');
  const betaT = rgb(input.betaT ?? .0007,'betaT',.01);
  const albedo = rgb(input.albedo ?? .9,'albedo',1);
  const p = Object.freeze({centre:vector(centre),radii:vector(radii),
    rotationColumns:Object.freeze(rotation.map(vector)), betaT,albedo,
    betaS:vector(betaT.map((x,i)=>x*albedo[i])),g});
  validatedProfiles.add(p);
  return p;
}
export const DEFAULT_FOG_PROFILE = createFogProfile();
function invalid(reason) {
  return {valid:false,reason,interval:null,I_metres:null,errorI:null,
    tauRGB:null,T_RGB:null,averageSigmaT:null,averageSigmaS:null};
}
function position(x) {
  const owned=numeric3(x);
  return owned && owned.every(v=>Math.abs(v)<=FOG_ENVELOPE.maxAbsCoordinate) ? owned : null;
}
// Encoded binary64 coordinates are inputs, not exact real-world positions.
function local(p, x, direction=false) {
  const q = direction ? x : x.map((v,i)=>v-p.centre[i]);
  const qi = direction ? x.map(exact) : x.map((v,i)=>sub(exact(v),exact(p.centre[i])));
  return {value:p.rotationColumns.map((col,i)=>dot(col,q)/p.radii[i]),
    bound:p.rotationColumns.map((col,i)=>div(dotI(col.map(exact),qi),exact(p.radii[i])))};
}
export function fogPoint(p,x) {
  if (!validatedProfiles.has(p)) return {valid:false,reason:'unvalidated-profile'};
  x=position(x);
  if (!x) return {valid:false,reason:'position-out-of-envelope'};
  const u=local(p,x); const raw=1-dot(u.value,u.value);
  const fi=sub(exact(1),normI(u.bound));
  const f=Math.max(0,raw), lo=Math.max(0,fi[0]), hi=Math.min(1,Math.max(0,fi[1]));
  return {valid:true,reason:'ok',f,errorF:Math.max(Math.abs(f-lo),Math.abs(hi-f)),
    sigma_tRGB:p.betaT.map(v=>v*f),sigma_sRGB:p.betaS.map(v=>v*f),g:p.g};
}
function result(p,I,errorI,cellLength,interval,reason) {
  if (![I,errorI].every(Number.isFinite) || I<0 || errorI<0) return invalid('nonfinite-integral');
  if (Math.max(...p.betaT)*errorI>FOG_ENVELOPE.maxTauError) return invalid('integral-error-exceeds-optical-envelope');
  const tauRGB=p.betaT.map(v=>v*I);
  const errorTauRGB=p.betaT.map((v,i)=>v===0?0:adjacent(v*errorI+8*U*Math.abs(tauRGB[i]),true));
  const T_RGB=tauRGB.map(v=>Math.exp(-v));
  const errorT_RGB=tauRGB.map((v,i)=>p.betaT[i]===0 || (I===0 && errorI===0)?0:Math.min(1,errorTauRGB[i]+Math.abs(widen(T_RGB[i],true,4)-T_RGB[i])));
  return {valid:true,reason,interval,I_metres:I,errorI,tauRGB,T_RGB,errorTauRGB,errorT_RGB,
    averageSigmaT:p.betaT.map(v=>cellLength===0?0:v*(I/cellLength)),
    averageSigmaS:p.betaS.map(v=>cellLength===0?0:v*(I/cellLength))};
}
export function fogSegment(p,o,d,tMin,tMax) {
  if (!validatedProfiles.has(p)) return invalid('unvalidated-profile');
  o=position(o);
  if (!o) return invalid('position-out-of-envelope');
  d=numeric3(d);
  if (!d || Math.abs(dot(d,d)-1)>FOG_ENVELOPE.unitTolerance) return invalid('non-unit-direction');
  if (!Number.isFinite(tMin) || !Number.isFinite(tMax) || tMin<0 || tMax<tMin || tMax>1e9) return invalid('invalid-cell-range');
  const cellLength=tMax-tMin;
  if (cellLength===0) return result(p,0,0,0,null,'zero-length');
  const u=local(p,o),v=local(p,d,true);
  const a=dot(v.value,v.value),ai=normI(v.bound);
  if (!(ai[0]>0) || !Number.isFinite(ai[1])) return invalid('uncertain-ray-metric');
  const h=-dot(u.value,v.value)/a;
  const hi=neg(div(dotI(u.bound,v.bound),ai));
  const closest=u.value.map((x,i)=>x+h*v.value[i]);
  const closestI=u.bound.map((x,i)=>add(x,mul(hi,v.bound[i])));
  const delta=1-dot(closest,closest),di=sub(exact(1),normI(closestI));
  if (!Number.isFinite(di[0]) || !Number.isFinite(di[1])) return invalid('nonfinite-closest-approach');
  if (di[1]<=0) return result(p,0,0,cellLength,null,'empty');
  // If tangency is unresolved, bound the entire possibly missed chord.
  const deltaUpper=exact(Math.max(0,di[1]));
  const fullUpper=div(mul(div(exact(4),exact(3)),mul(deltaUpper,sqrtI(deltaUpper))),sqrtI(ai))[1];
  if (di[0]<=0) {
    const tangentError=fullUpper;
    if (Math.max(...p.betaT)*tangentError>FOG_ENVELOPE.maxTauError) return invalid('uncertain-tangent');
    return result(p,0,tangentError,cellLength,null,'tangent-bounded-missed-chord');
  }
  if (!(delta>0)) return invalid('inconsistent-intersection');
  const r=Math.sqrt(delta/a),ri=sqrtI(div(di,ai));
  const starts=sub(hi,ri),ends=add(hi,ri);
  if (tMax<=starts[0] || tMin>=ends[1]) return result(p,0,0,cellLength,null,'empty');
  const A=Math.max(tMin,h-r),B=Math.min(tMax,h+r);
  const Ai=[Math.max(tMin,starts[0]),Math.max(tMin,starts[1])];
  const Bi=[Math.min(tMax,ends[0]),Math.min(tMax,ends[1])];
  if (!(B>A)) {
    const possibleLength=Math.max(0,Bi[1]-Ai[0]);
    const missed=Math.min(fullUpper,adjacent(possibleLength*Math.min(1,di[1]),true));
    return result(p,0,missed,cellLength,null,'clip-boundary-bounded-missed-chord');
  }
  const L=B-A,Li=[Math.max(0,adjacent(Bi[0]-Ai[1],false)),Math.max(0,adjacent(Bi[1]-Ai[0],true))];
  const m=A+L/2,mi=mul(add(Ai,Bi),exact(.5));
  function density(s,si) {
    const z=s-h, zi=sub(si,hi);
    const raw=a*(r-z)*(r+z);
    const bound=mul(ai,mul(sub(ri,zi),add(ri,zi)));
    if (bound[1]<0 || !Number.isFinite(raw)) return null;
    return {value:Math.max(0,raw),bound:[Math.max(0,bound[0]),Math.min(1,Math.max(0,bound[1]))]};
  }
  const fa=density(A,Ai),fm=density(m,mi),fb=density(B,Bi);
  if (!fa || !fm || !fb) return invalid('negative-density-beyond-error');
  const I=L*(fa.value+4*fm.value+fb.value)/6;
  const Ii=div(mul(Li,add(add(fa.bound,mul(exact(4),fm.bound)),fb.bound)),exact(6));
  const lower=Math.max(0,Ii[0]),upper=Math.min(fullUpper,cellLength,Math.max(0,Ii[1]));
  if (lower>upper) return invalid('inconsistent-integral-bound');
  const errorI=adjacent(Math.max(Math.abs(I-lower),Math.abs(upper-I)),true);
  return result(p,I,errorI,cellLength,[A,B],'ok');
}
