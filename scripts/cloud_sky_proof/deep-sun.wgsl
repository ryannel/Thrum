// Bounded coupled-depth candidate. Sections are extracted by deep-sun.js.
// BEGIN_DEEP_COMMON
const DEEP_N:u32=64u;
const DEEP_PHYSICAL:u32=66u;
const DEEP_K:u32=48u;
const DEEP_STEPS:u32=512u;
const DEEP_ALLOCATION:f32=.005;
const DEEP_EPSILON:f32=.004998;
const DEEP_NUMERIC_RESERVE:f32=.000002;
const DEEP_FALLBACK_CAP:u32=4096u;
struct DeepCurveMetadata { admission:vec4f,work:vec4f }
fn deepFinite(x:f32)->bool {return x==x&&abs(x)<3.402823e38;}
// Coefficients are zero outside the first/last half-cell reconstruction collar.
// U/V span the source support plus two nominal curve pitches on EACH side:
// H=raw+2*(2H/N), hence H=raw/(1-4/N). W has no lateral padding.
fn deepRawHalf()->vec3f {
  let halfSupport=.5*scene.extent.xyz*(vec3f(1.)+1./vec3f(scene.grid.xyz));
  return vec3f(dot(abs(scene.U.xyz),halfSupport),dot(abs(scene.V.xyz),halfSupport),dot(abs(scene.W.xyz),halfSupport));
}
fn deepHalf()->vec3f {let r=deepRawHalf();return vec3f(r.xy/(1.-4./f32(DEEP_N)),r.z);}
fn deepFrameValid()->bool {
  if(any(scene.grid.xyz==vec3u(0))||any(scene.extent.xyz<=vec3f(0.))){return false;}
  for(var axis=0u;axis<3u;axis++) {
    if(!deepFinite(scene.lo[axis])||!deepFinite(scene.extent[axis])||!deepFinite(scene.U[axis])||!deepFinite(scene.V[axis])||!deepFinite(scene.W[axis])||!deepFinite(scene.sun[axis])){return false;}
  }
  return abs(dot(scene.U.xyz,scene.U.xyz)-1.)<.00001&&abs(dot(scene.V.xyz,scene.V.xyz)-1.)<.00001&&abs(dot(scene.W.xyz,scene.W.xyz)-1.)<.00001&&abs(dot(scene.U.xyz,scene.V.xyz))<.00001&&abs(dot(scene.U.xyz,scene.W.xyz))<.00001&&abs(dot(scene.V.xyz,scene.W.xyz))<.00001&&length(scene.W.xyz+scene.sun.xyz)<.00001;
}
// END_DEEP_COMMON

// BEGIN_DEEP_QUERY
// Requires caller's Scene uniform `scene`, errors[0..3] atomics and
// tauDirect(p, normalizedSun, 512u). No recursive light/source evaluation.
@group(0) @binding(11) var<storage,read> deepNodes:array<vec2f>;
@group(0) @binding(12) var<storage,read> deepMetadata:array<DeepCurveMetadata>;
fn deepSunFallback(p:vec3f)->vec2f {
  let request=atomicAdd(&errors[1],1u);
  if(request>=DEEP_FALLBACK_CAP){atomicAdd(&errors[3],1u);return vec2f(-1.);}
  let tau=tauDirect(p,scene.sun.xyz,DEEP_STEPS);
  if(!deepFinite(tau)||tau<0.) {
    // Explicit unsupported-input failure: negative terms require visible graph failure.
    // Caller must reject any frame whose unsupported counter is nonzero.
    atomicAdd(&errors[3],1u);return vec2f(-1.);
  }
  return exp(-vec2f(1.,.25)*tau);
}
fn deepCurveTau(column:u32,depth:f32,count:u32)->f32 {
  let base=column*DEEP_K;
  if(depth<=deepNodes[base].x){return deepNodes[base].y;}
  if(depth>=deepNodes[base+count-1u].x){return deepNodes[base+count-1u].y;}
  var lo=0u;var hi=count-1u;
  loop {if(hi-lo<=1u){break;}let mid=lo+(hi-lo)/2u;if(deepNodes[base+mid].x<=depth){lo=mid;}else{hi=mid;}}
  let a=deepNodes[base+lo];let b=deepNodes[base+hi];
  return mix(a.y,b.y,(depth-a.x)/(b.x-a.x));
}
fn deepSunTerms(p:vec3f)->vec2f {
  if(!deepFrameValid()||!deepFinite(p.x)||!deepFinite(p.y)||!deepFinite(p.z)) {
    atomicAdd(&errors[3],1u);return vec2f(-1.);
  }
  // Domain/layout failure is uniform across the builder, unlike compression
  // overflow. Check before even a support-proven clear return. Never invoke the
  // caller's packed direct sampler with a mismatched Scene/texture layout.
  let domainFlags=deepMetadata[0].work.w;
  if(!deepFinite(domainFlags)||domainFlags<0.||domainFlags!=floor(domainFlags)){
    atomicAdd(&errors[3],1u);return vec2f(-1.);
  }
  if((u32(domainFlags)&1u)!=0u){atomicAdd(&errors[3],1u);return vec2f(-1.);}
  let q=p-(scene.lo.xyz+scene.extent.xyz*.5);
  let projection=vec3f(dot(q,scene.U.xyz),dot(q,scene.V.xyz),dot(q,scene.W.xyz));
  let raw=deepRawHalf();let halfSpan=deepHalf();
  // The defined operator is four-tap filtered light, not point tau. Even a
  // point just outside raw support can weight an interior column. Keep the
  // entire padded projection; only beyond H are BOTH neighbour columns beyond
  // raw support (H-raw is two pitches, nearest tap at H-.5 pitch).
  // Before the shared Sun-facing plane every neighbouring prefix is zero.
  let guard=vec3f(.00001)*max(raw,vec3f(1.));
  if(any(abs(projection.xy)>halfSpan.xy+guard.xy)||projection.z< -raw.z-guard.z){return vec2f(1.);}
  let uv=projection.xy/(2.*halfSpan.xy)+.5;
  // Physical column i is at normalized (i-.5)/64; 0 and65 are guard centres.
  let continuous=uv*f32(DEEP_N)+.5;let cell=vec2i(floor(continuous));let f=fract(continuous);
  if(any(cell<vec2i(0))||any(cell+vec2i(1)>=vec2i(i32(DEEP_PHYSICAL)))) {
    atomicAdd(&errors[3],1u);return deepSunFallback(p);
  }
  let depth=projection.z+halfSpan.z;
  var values:array<vec2f,4>;
  for(var y=0u;y<2u;y++){for(var x=0u;x<2u;x++) {
    let c=cell+vec2i(i32(x),i32(y));let index=u32(c.y)*DEEP_PHYSICAL+u32(c.x);let record=deepMetadata[index].admission;
    let flags=deepMetadata[index].work.w;
    if(!deepFinite(flags)||flags<0.||flags!=floor(flags)||(u32(flags)&1u)!=0u){atomicAdd(&errors[3],1u);return vec2f(-1.);}
    if(record.y!=1.||!deepFinite(record.x)||record.x<2.||record.x>f32(DEEP_K)||record.x!=floor(record.x)||!deepFinite(record.z)||!deepFinite(record.w)||record.z<0.||record.w<0.||max(record.z,record.w)>DEEP_ALLOCATION) {
      atomicAdd(&errors[2],1u);return deepSunFallback(p);
    }
    values[y*2u+x]=exp(-vec2f(1.,.25)*deepCurveTau(index,depth,u32(record.x)));
  }}
  // Average each physical term, NEVER exp(-lerp(tau)) or -log(mean primary).
  return mix(mix(values[0],values[1],f.x),mix(values[2],values[3],f.x),f.y);
}
// END_DEEP_QUERY

// BEGIN_DEEP_BUILD
// Exact current bounded viewer ABI; no dependence on its pipeline/shader.
struct Scene {
  lo:vec4f,extent:vec4f,camera:vec4f,lookAt:vec4f,
  sun:vec4f,U:vec4f,V:vec4f,W:vec4f,
  E:vec4f,sky:vec4f,grid:vec4u,switches:vec4u
}
@group(0) @binding(0) var deepDensity:texture_3d<f32>;
@group(0) @binding(1) var deepFiltered:sampler;
@group(0) @binding(4) var<uniform> scene:Scene;
@group(1) @binding(4) var<storage,read_write> deepNodesOut:array<vec2f>;
@group(1) @binding(5) var<storage,read_write> deepMetadataOut:array<DeepCurveMetadata>;
// Canonical four-Z packing: XY linear filtering, explicit logical-Z blend.
// Explicit world support test prevents clamp-to-edge from extending the source.
fn deepSigma(p:vec3f)->f32 {
  let u=(p-scene.lo.xyz)/(scene.extent.xyz/vec3f(scene.grid.xyz));
  if(any(u<vec3f(-.5))||any(u>vec3f(scene.grid.xyz)+.5)){return 0.;}
  let q=u.z-.5;let z0=i32(floor(q));let physical=vec3f(textureDimensions(deepDensity));
  var v=vec2f(0.);
  for(var i=0u;i<2u;i++) {
    let z=z0+i32(i);
    if(z>=0&&z<i32(scene.grid.z)) {
      v[i]=textureSampleLevel(deepDensity,deepFiltered,vec3f((u.xy+1.)/physical.xy,(f32(z/4)+1.5)/physical.z),0.)[z%4];
    }
  }
  return mix(v.x,v.y,fract(q));
}
// Stable -log(1-r), with r>=epsilon. The eight-term small-r series has
// analytic tail <=r^9/(9*(1-r)); for r<.125 this is <9.5e-10. Remaining
// exp/log/slope/interpolation f32 error is NOT uniformly backend-certified.
fn deepNegativeLogOneMinus(r:f32)->f32 {
  if(r<.125) {
    var power=r;var result=0.;
    for(var order=1u;order<=8u;order++){result+=power/f32(order);power*=r;}
    return result;
  }
  return -log(1.-r);
}
// -1 means both branches opaque within epsilon; -2 is numerical failure.
// Evaluate exp(-a*tau) BEFORE division, never epsilon*exp(+a*tau).
fn deepTubeDelta(tau:f32)->f32 {
  var delta=3.402823e38;var constrained=false;
  for(var branch=0u;branch<2u;branch++) {
    let a=select(1.,.25,branch==1u);let term=exp(-a*tau);
    if(term>DEEP_EPSILON) {
      let ratio=DEEP_EPSILON/term;
      if(!(ratio>0.&&ratio<1.)){return -2.;}
      let width=deepNegativeLogOneMinus(ratio)/a;
      if(!deepFinite(width)||width<=0.){return -2.;}
      delta=min(delta,width);constrained=true;
    }
  }
  return select(-1.,delta,constrained);
}
// One invocation per column. Completed original endpoints are written once;
// private working state is O(1), with no K-array/minimum scan/removal history.
// Metadata work=(coefficient requests, endpoint visits, anchor builds, flags).
// flags:1 frame/domain,2 coefficient,4 depth/tau,8 K capacity,16 tube arithmetic.
@compute @workgroup_size(4,4) fn buildDeepSun(@builtin(global_invocation_id) id:vec3u) {
  if(any(id.xy>=vec2u(DEEP_PHYSICAL))){return;}
  let column=id.y*DEEP_PHYSICAL+id.x;let base=column*DEEP_K;
  if(!deepFrameValid()) {deepMetadataOut[column]=DeepCurveMetadata(vec4f(0.),vec4f(0.,0.,0.,1.));return;}
  let packedShape=vec3u(scene.grid.xy+vec2u(2u),scene.grid.z/4u+select(0u,1u,scene.grid.z%4u!=0u)+2u);
  if(any(textureDimensions(deepDensity)!=packedShape)){
    deepMetadataOut[column]=DeepCurveMetadata(vec4f(0.),vec4f(0.,0.,0.,1.));return;
  }
  let halfSpan=deepHalf();let span=2.*halfSpan.z;
  if(!deepFinite(span)||span<=0.||!deepFinite(halfSpan.x)||!deepFinite(halfSpan.y)){deepMetadataOut[column]=DeepCurveMetadata(vec4f(0.),vec4f(0.,0.,0.,1.));return;}
  let xy=((vec2f(id.xy)-.5)/f32(DEEP_N)-.5)*2.*halfSpan.xy;
  let origin=scene.lo.xyz+scene.extent.xyz*.5+scene.U.xyz*xy.x+scene.V.xyz*xy.y-scene.W.xyz*halfSpan.z;
  let h=span/f32(DEEP_STEPS);
  var anchor=vec2f(0.);var previous=anchor;var tau=0.;
  var count=1u;var requests=0u;var visits=0u;var anchorBuilds=1u;var flags=0u;
  var lower=0.;var upper=3.402823e38;var delta=deepTubeDelta(0.);
  deepNodesOut[base]=anchor;
  for(var step=0u;step<DEEP_STEPS;step++) {
    requests++;
    let sigma=deepSigma(origin+scene.W.xyz*((f32(step)+.5)*h));
    if(!deepFinite(sigma)||sigma<0.){flags=2u;break;}
    tau+=sigma*h;
    let point=vec2f(f32(step+1u)*h,tau);
    if(!deepFinite(tau)||!deepFinite(point.x)||point.x<=previous.x||tau<previous.y){flags=4u;break;}
    for(var attempt=0u;attempt<2u;attempt++) {
      visits++;
      let dx=point.x-anchor.x;let dt=point.y-anchor.y;let slope=dt/dx;
      if(!deepFinite(slope)||delta< -1.){flags=16u;break;}
      // Critical rule: test ACTUAL endpoint slope against OLD intersection.
      if(slope>=lower&&slope<=upper) {
        if(delta>=0.) {
          let nextLower=max(lower,(dt-delta)/dx);let nextUpper=min(upper,(dt+delta)/dx);
          if(!deepFinite(nextLower)||!deepFinite(nextUpper)||nextLower>nextUpper){flags=16u;break;}
          lower=nextLower;upper=nextUpper;
        }
        previous=point;break;
      }
      if(attempt!=0u||previous.x<=anchor.x){flags=16u;break;}
      if(count>=DEEP_K){flags=8u;break;}
      // Commit PREVIOUS accepted original endpoint, never the failing point.
      deepNodesOut[base+count]=previous;count++;
      anchor=previous;lower=0.;upper=3.402823e38;
      delta=deepTubeDelta(anchor.y);anchorBuilds++;
      // Reprocess same point; never requery source or advance its generation.
    }
    if(flags!=0u){break;}
  }
  if(flags==0u&&previous.x>anchor.x) {
    if(count>=DEEP_K){flags=8u;}else{deepNodesOut[base+count]=previous;count++;}
  }
  let admitted=flags==0u&&count>=2u;
  // .005 is the NOMINAL epsilon+once-only reserve, NOT a measured/certified
  // backend max error. Disjoint segments take a maximum, never a removal sum.
  deepMetadataOut[column]=DeepCurveMetadata(vec4f(f32(count),select(0.,1.,admitted),DEEP_ALLOCATION,DEEP_ALLOCATION),vec4f(f32(requests),f32(visits),f32(anchorBuilds),f32(flags)));
}
// END_DEEP_BUILD
