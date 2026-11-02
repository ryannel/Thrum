// Bounded optical source fixture: vacuum + diffuse external terminal, no spherical-air claim.
struct Scene {
  lo:vec4f, extent:vec4f, camera:vec4f, lookAt:vec4f,
  sun:vec4f, U:vec4f, V:vec4f, W:vec4f,
  E:vec4f, sky:vec4f, grid:vec4u, switches:vec4u
}
@group(0) @binding(0) var density:texture_3d<f32>;
@group(0) @binding(1) var filtered:sampler;
@group(0) @binding(2) var prefix:texture_3d<f32>;
@group(0) @binding(3) var ambient:texture_3d<f32>;
@group(0) @binding(4) var<uniform> scene:Scene;
@group(0) @binding(5) var<storage,read_write> errors:array<atomic<u32>>;
@group(0) @binding(7) var bankA:texture_2d<f32>;
const PI:f32=3.141592653589793;
fn rayBox(o:vec3f,d:vec3f,lo:vec3f,hi:vec3f)->vec2f {
  var near=-1e20;var far=1e20;
  for(var k=0;k<3;k++) {
    if(abs(d[k])<1e-8) { if(o[k]<lo[k]||o[k]>hi[k]) {return vec2f(1.,-1.);} }
    else {let a=(lo[k]-o[k])/d[k];let b=(hi[k]-o[k])/d[k];near=max(near,min(a,b));far=min(far,max(a,b));}
  }
  return vec2f(near,far);
}
fn sigmaAt(p:vec3f)->f32 {
  let u=(p-scene.lo.xyz)/scene.extent.xyz*vec3f(scene.grid.xyz);
  return samplePacked(density,filtered,u,scene.grid.xyz);
}
fn sourceInterval(o:vec3f,d:vec3f)->vec2f {
  let halo=.5*scene.extent.xyz/vec3f(scene.grid.xyz);
  return rayBox(o,d,scene.lo.xyz-halo,scene.lo.xyz+scene.extent.xyz+halo);
}
fn rayStep(d:vec3f)->f32 {
  let pitch=scene.extent.xyz/vec3f(scene.grid.xyz);
  return .5*min(pitch.x/max(abs(d.x),1e-6),min(pitch.y/max(abs(d.y),1e-6),pitch.z/max(abs(d.z),1e-6)));
}
fn tauDirect(o:vec3f,d:vec3f,count:u32)->f32 {
  let span=sourceInterval(o,d);let start=max(0.,span.x);let end=max(start,span.y);
  let h=(end-start)/f32(count);var tau=0.;
  for(var i=0u;i<count;i++) {tau+=sigmaAt(o+d*(start+(f32(i)+.5)*h))*h;}
  return tau;
}
fn sunTau(p:vec3f)->f32 {
  let q=p-(scene.lo.xyz+scene.extent.xyz*.5);
  let uv=vec3f(dot(q,scene.U.xyz)/(2.*scene.U.w),dot(q,scene.V.xyz)/(2.*scene.V.w),dot(q,scene.W.xyz)/(2.*scene.W.w))+.5;
  if(any(uv.xy<vec2f(0.))||any(uv.xy>vec2f(1.))||uv.z<=0.) {return 0.;}
  return textureSampleLevel(prefix,filtered,vec3f((uv.xy*64.+1.)/66.,(min(uv.z,1.)*64.+1.5)/66.),0.).x;
}
fn externalAmbient(p:vec3f)->vec3f {
  let u=clamp((p-scene.lo.xyz)/scene.extent.xyz,vec3f(0.),vec3f(1.));
  return textureSampleLevel(ambient,filtered,u,0.).rgb;
}
fn hg(mu:f32,g:f32)->f32 {let q=1.+g*g-2.*g*mu;return(1.-g*g)/(4.*PI*q*sqrt(q));}
fn geometryVisibility(p:vec3f,d:vec3f)->f32 {
  let q=p-vec3f(0.,80.,0.);let b=dot(q,d);let discriminant=b*b-dot(q,q)+6400.;
  if(discriminant>=0.&&(-b+sqrt(discriminant))>.01) {return 0.;}
  return 1.;
}
fn bankCell(c:vec2i)->f32 {
  if(any(c<vec2i(0))||any(c>=vec2i(textureDimensions(bankA)))){return 0.;}
  return textureLoad(bankA,c,0).r;
}
fn bankAt(p:vec3f)->f32 {
  let u=(p.xz-scene.lo.xz)/scene.extent.xz*vec2f(scene.grid.xz)-.5;
  let c=vec2i(floor(u));let f=fract(u);
  return mix(mix(bankCell(c),bankCell(c+vec2i(1,0)),f.x),mix(bankCell(c+vec2i(0,1)),bankCell(c+vec2i(1,1)),f.x),f.y);
}
fn cloudSource(p:vec3f,ray:vec3f,sigma:f32)->vec3f {
  let tau=sunTau(p);let mu=dot(ray,scene.sun.xyz);
  var directional=hg(mu,.86)*exp(-tau);
  if(scene.grid.w==0u){directional=.7*hg(mu,.86)*exp(-tau)+.1*hg(mu,.43)*exp(-.5*tau)+.0175*hg(mu,.215)*exp(-.25*tau)+.00375*hg(mu,.1075)*exp(-.125*tau);}
  else if(scene.grid.w==2u&&scene.sun.w>0.){directional+=bankAt(p)/(4.*PI)*exp(-.25*tau);}
  return sigma*(scene.E.rgb*scene.sun.w*geometryVisibility(p,scene.sun.xyz)*directional+externalAmbient(p));
}
struct Pair {S:vec3f,T:f32}
fn integrate(o:vec3f,d:vec3f,limit:f32,visible:bool)->Pair {
  var pair:Pair;pair.S=vec3f(0.);pair.T=1.;
  if(!visible) {return pair;}
  let span=sourceInterval(o,d);var t=max(0.,span.x);let end=min(limit,span.y);
  let step=rayStep(d);
  var count=0u;
  loop {
    if(t>=end) {break;}
    if(count>=1024u) {atomicAdd(&errors[0],1u);pair.S=vec3f(1e8,0.,1e8);return pair;}
    let h=min(step,end-t);let p=o+d*(t+h*.5);let sigma=sigmaAt(p);let tau=sigma*h;
    if(sigma>0.) {pair.S+=pair.T*cloudSource(p,d,sigma)*h*beerFactor(tau);pair.T*=exp(-tau);}
    t+=h;count++;
    if(pair.T*2000000.<.25) {break;}
  }
  return pair;
}
struct Hit {t:f32,kind:u32,normal:vec3f}
fn hitScene(o:vec3f,d:vec3f,mirror:bool)->Hit {
  var hit:Hit;hit.t=80000.;hit.kind=0u;hit.normal=vec3f(0.,1.,0.);
  if(d.y< -1e-7) {
    let t=-o.y/d.y;
    if(t>0.) {hit.t=t;hit.kind=1u;let p=o+d*t;if(mirror&&((scene.switches.y==2u&&p.x> -1800.&&p.x<1800.&&p.z>400.&&p.z<1800.)||(scene.switches.y==1u&&abs(p.x)<10000.&&p.z>0.&&p.z<10000.))){hit.kind=3u;}}
  }
  let centre=vec3f(0.,80.,0.);let q=o-centre;let b=dot(q,d);let disc=b*b-dot(q,q)+6400.;
  if(disc>=0.) {let t=-b-sqrt(disc);if(t>0.&&t<hit.t){hit.t=t;hit.kind=2u;hit.normal=normalize(o+d*t-centre);}}
  return hit;
}
fn surface(p:vec3f,normal:vec3f,kind:u32)->vec3f {
  var albedo=vec3f(.38,.43,.34);
  if(kind==2u) {albedo=vec3f(.6,.18,.08);}else{let q=fract(p.xz/200.);if(q.x<.02||q.y<.02){albedo*=.65;}}
  // Constant diffuse receiver term is a fixture, not a qualified full-sky surface environment.
  return albedo/PI*(scene.E.rgb*scene.sun.w*geometryVisibility(p+normal*.02,scene.sun.xyz)*exp(-sunTau(p))*max(0.,dot(normal,scene.sun.xyz))+scene.sky.rgb*PI*.5);
}
fn terminal(o:vec3f,d:vec3f,hit:Hit)->vec3f {
  if(hit.kind==0u) {return scene.sky.rgb;}
  return surface(o+d*hit.t,hit.normal,hit.kind);
}
// Geometry-only inspection, deliberately independent of Sun/ambient/display exposure.
fn shapeDiagnostic(o:vec3f,d:vec3f)->vec3f {
  let span=sourceInterval(o,d);var t=max(0.,span.x);let end=span.y;let h=rayStep(d);
  let threshold=scene.sky.w*.5;
  var previous=t;var count=0u;
  loop {
    if(t>=end) {break;}
    if(count>=1024u){atomicAdd(&errors[0],1u);return vec3f(1.,0.,1.);}
    if(sigmaAt(o+d*t)>=threshold) {
      var lo=previous;var hi=t;
      for(var j=0u;j<5u;j++){let mid=.5*(lo+hi);if(sigmaAt(o+d*mid)>=threshold){hi=mid;}else{lo=mid;}}
      let p=o+d*.5*(lo+hi);let delta=scene.extent.xyz/vec3f(scene.grid.xyz)*.5;
      let gradient=vec3f((sigmaAt(p+vec3f(delta.x,0.,0.))-sigmaAt(p-vec3f(delta.x,0.,0.)))/delta.x,
                        (sigmaAt(p+vec3f(0.,delta.y,0.))-sigmaAt(p-vec3f(0.,delta.y,0.)))/delta.y,
                        (sigmaAt(p+vec3f(0.,0.,delta.z))-sigmaAt(p-vec3f(0.,0.,delta.z)))/delta.z);
      var normal=-d;if(length(gradient)>1e-8){normal=-normalize(gradient);}
      let lamp=normalize(vec3f(-.6,.8,.35));
      return vec3f(.12+.78*max(0.,dot(normal,lamp)));
    }
    previous=t;t=min(t+h,end);count++;
  }
  return vec3f(.18);
}
struct Vertex { @builtin(position) position:vec4f,@location(0) uv:vec2f }
@vertex fn triangle(@builtin(vertex_index) i:u32)->Vertex {
  var p=array<vec2f,3>(vec2f(-1.,-1.),vec2f(3.,-1.),vec2f(-1.,3.));
  var out:Vertex;out.position=vec4f(p[i],0.,1.);out.uv=p[i]*vec2f(.5,-.5)+.5;return out;
}
@fragment fn image(in:Vertex)->@location(0)vec4f {
  let forward=normalize(scene.lookAt.xyz-scene.camera.xyz);let right=normalize(cross(forward,vec3f(0.,1.,0.)));let up=cross(right,forward);
  let q=vec2f((in.uv.x*2.-1.)*scene.camera.w,1.-in.uv.y*2.);
  let ray=normalize(forward+scene.lookAt.w*(right*q.x+up*q.y));
  if(scene.switches.w==1u){return vec4f(pow(shapeDiagnostic(scene.camera.xyz,ray),vec3f(1./2.2)),1.);}
  let hit=hitScene(scene.camera.xyz,ray,scene.switches.y!=0u);
  var far=terminal(scene.camera.xyz,ray,hit);
  if(hit.kind==3u) {
    let p=scene.camera.xyz+ray*hit.t;let reflected=vec3f(ray.x,-ray.y,ray.z);let start=p+reflected*.02;
    let next=hitScene(start,reflected,false);let leg=integrate(start,reflected,next.t,true);
    far=leg.S+leg.T*terminal(start,reflected,next);
  }
  let near=integrate(scene.camera.xyz,ray,hit.t,scene.switches.x!=0u);
  var value=(near.S+near.T*far)*scene.E.w;
  if(scene.switches.z!=0u||scene.switches.w==2u) {return vec4f(pow(vec3f(1.-near.T),vec3f(1./2.2)),1.);}
  value=value/(vec3f(1.)+value);
  return vec4f(pow(max(value,vec3f(0.)),vec3f(1./2.2)),1.);
}
