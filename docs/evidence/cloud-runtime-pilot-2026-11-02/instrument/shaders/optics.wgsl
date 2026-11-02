struct Params  {
  camera:vec4f, focus:vec4f, sun:vec4f, bmin:vec4f, bmax:vec4f, weather:vec4f, control:vec4f, front:vec4f, image:vec4f, domain:vec4f
}
;
@group(0) @binding(0) var<uniform> p:Params;
@group(0) @binding(1) var densityTex:texture_3d<f32>;
@group(0) @binding(2) var basis:texture_3d<f32>;
@group(0) @binding(3) var linearClamp:sampler;
@group(0) @binding(4) var restTex:texture_3d<f32>;
@group(0) @binding(5) var voxelLight:texture_3d<f32>;
@group(0) @binding(7) var airTrans:texture_2d<f32>;
@group(0) @binding(10) var previousQ:texture_3d<f32>;
@group(0) @binding(11) var previousRest:texture_3d<f32>;
@group(0) @binding(13) var receiverMap:texture_2d<f32>;
@group(0) @binding(14) var linearRepeat:sampler;
const PI:f32=3.141592653589793;
fn rayBox(o:vec3f,d:vec3f,lo:vec3f,hi:vec3f)->vec2f {
  let safe=select(vec3f(1e-7),d,abs(d)>vec3f(1e-7));
  let a=(lo-o)/safe;
  let b=(hi-o)/safe;
  return vec2f(max(max(min(a.x,b.x),min(a.y,b.y)),min(a.z,b.z)),min(min(max(a.x,b.x),max(a.y,b.y)),max(a.z,b.z)));
}
fn uvw(x:vec3f)->vec3f {
  return(x-p.bmin.xyz)/(p.bmax.xyz-p.bmin.xyz);
}
fn rawQ(x:vec3f)->vec4f {
  let uv=uvw(x);
  if(any(uv<vec3f(0))||any(uv>vec3f(1))) {
    return vec4f(0);
  }
  return mix(textureSampleLevel(previousQ,linearClamp,uv,0.),textureSampleLevel(densityTex,linearClamp,uv,0.),p.domain.x);
}
fn coefficients(x:vec3f,detail:f32)->vec2f {
  if(p.domain.w>0.5) {
    return vec2f(0);
  }
  let q=rawQ(x).xy;
  if(q.x+q.y<=0.) {
    return vec2f(0);
  }
  let uv=uvw(x);
  let coord=mix(textureSampleLevel(previousRest,linearClamp,clamp(uv,vec3f(0),vec3f(1)),0.).xyz,textureSampleLevel(restTex,linearClamp,clamp(uv,vec3f(0),vec3f(1)),0.).xyz,p.domain.x);
  let low=textureSampleLevel(basis,linearRepeat,fract(coord),0.).x;
  let n=textureSampleLevel(basis,linearRepeat,fract(coord*4.3),0.);
  let bodyDensity=clamp((q-vec2f((1.-low)*.65))/.35,vec2f(0),vec2f(1));
  let edge=1.-smoothstep(vec2f(.4),vec2f(.9),bodyDensity);
  var shaped=max(bodyDensity-vec2f((1.-n.y)*.20*detail)*edge,vec2f(0));
  if(u32(p.control.w)==4u) {
    shaped=max(q-vec2f((1.-n.y)*.06*detail),vec2f(0));
  }
  return shaped*vec2f(p.bmin.w,p.bmax.w);
}
fn tauRay(x:vec3f,d:vec3f,count:u32,detail:f32)->f32 {
  let interval=rayBox(x,d,p.bmin.xyz,p.bmax.xyz);
  let start=max(0.,interval.x);
  let end=interval.y;
  if(end<=start) {
    return 0.;
  }
  let ds=(end-start)/f32(count);
  var tau=0.;
  for(var j=0u;j<count;j++) {
    let c=coefficients(x+d*(start+(f32(j)+.5)*ds),detail);
    tau+=(c.x+c.y)*ds;
  }
  return tau;
}
fn directT(x:vec3f)->vec2f {
  let tau=tauRay(x,p.sun.xyz,32u,1.);
  return exp(-vec2f(tau,tau*p.domain.y));
}
fn cacheT(x:vec3f)->vec2f {
  let uv=uvw(x);
  if(any(uv<vec3f(0))||any(uv>vec3f(1))) {
    return directT(x);
  }
  return textureSampleLevel(voxelLight,linearClamp,uv,0.).xy;
}
fn cloudSun(x:vec3f)->vec2f {
  var tau=0.;
  let ds=100.;
  for(var i=0u;i<2u;i++) {
    let c=coefficients(x+p.sun.xyz*((f32(i)+.5)*ds),1.);
    tau+=(c.x+c.y)*ds;
  }
  return exp(-vec2f(tau,p.domain.y*tau))*cacheT(x+p.sun.xyz*200.);
}
fn ambientA(x:vec3f)->f32 {
  let uv=uvw(x);
  if(any(uv<vec3f(0))||any(uv>vec3f(1))) {
    return 1.;
  }
  return textureSampleLevel(voxelLight,linearClamp,uv,0.).z;
}
fn hg(g:f32,mu:f32)->f32 {
  return(1.-g*g)/(4.*PI*pow(max(1.+g*g-2.*g*mu,.001),1.5));
}
fn phase(mu:f32,ice:f32,factor:f32)->f32 {
  let a=mix(.8,.85,ice)*factor;
  let b=mix(-.2,-.15,ice)*factor;
  let w=mix(.8,.85,ice);
  return w*hg(a,mu)+(1.-w)*hg(b,mu);
}
fn airAt(x:vec3f)->mat3x3f {
  let h=max(x.y,0.);
  let ray=exp(-h/8000.)*vec3f(5.802e-6,13.558e-6,33.1e-6);
  let mie=vec3f(4e-6*exp(-h/1200.));
  let ozone=vec3f(.65e-6,1.881e-6,.085e-6)*max(0.,1.-abs(h-25000.)/15000.);
  return mat3x3f(ray+mie+ozone,ray,mie*.9);
}
fn atmosphereExit(x:vec3f,d:vec3f)->f32 {
  let o=x+vec3f(0,6371000.,0);
  let b=dot(o,d);
  let c=dot(o,o)-6471000.*6471000.;
  return max(0.,-b+sqrt(max(b*b-c,0.)));
}
fn airRayT(x:vec3f,dir:vec3f)->vec3f {
  let o=x+vec3f(0,6371000.,0);
  let dotSun=dot(o,dir);
  if(dotSun<0.) {
    if(dotSun*dotSun-dot(o,o)+6371000.*6371000.>0.) {
      return vec3f(0);
    }
  }
  let total=atmosphereExit(x,dir);
  var tau=vec3f(0);
  for(var i=0u;i<48u;i++) {
    let t0=pow(f32(i)/48.,2.)*total;
    let t1=pow(f32(i+1u)/48.,2.)*total;
    let point=x+dir*((t0+t1)*.5);
    let h=length(point+vec3f(0,6371000.,0))-6371000.;
    tau+=airAt(vec3f(0,h,0))[0]*(t1-t0);
  }
  return exp(-tau);
}
fn airSun(x:vec3f)->vec3f {
  let o=x+vec3f(0,6371000.,0);
  let h=max(length(o)-6371000.,0.);
  let mu=dot(normalize(o),p.sun.xyz);
  if(mu<0.) {
    let b=dot(o,p.sun.xyz);
    if(b*b-dot(o,o)+6371000.*6371000.>0.) {
      return vec3f(0);
    }
  }
  let uv=vec2f(mu*.5+.5,sqrt(clamp(h/100000.,0.,1.)));
  return textureSampleLevel(airTrans,linearClamp,uv,0.).rgb;
}
fn solar()->vec3f {
  return vec3f(6.8,6.55,6.3);
}
fn skyMean()->vec3f {
  return vec3f(.42,.49,.58)*(.2+.8*smoothstep(-.08,.4,p.sun.y));
}
