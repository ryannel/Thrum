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
// Low four bits name the family; bit4 selects the shared Cb material profile.
fn opticalFamily()->u32 {return u32(p.control.w)&15u;}
fn cbMaterialAmount(total:f32,low:f32,fine:f32,detail:f32)->f32 {
  let q=clamp(total,0.,1.);let threshold=.25+.30*(1.-q);
  let structure=.12+.88*smoothstep(threshold-.16,threshold+.16,low);
  return q*structure*(1.-.35*(1.-fine)*detail);
}
fn shapeMatter(q:vec2f,low:f32,fine:f32,detail:f32)->vec2f {
  let total=q.x+q.y;
  if(total<=0.){return vec2f(0);}
  let bodyDensity=clamp((total-(1.-low)*.65)/.35,0.,1.);
  let edge=1.-smoothstep(.4,.9,bodyDensity);
  var shaped=vec2f(max(bodyDensity-(1.-fine)*.20*detail*edge,0.))*q/total;
  if(opticalFamily()==1u){
    var amount=clamp(total,0.,1.)*(.65+.35*low)*(1.-.12*(1.-fine)*detail);
    if((u32(p.control.w)&16u)!=0u){amount=cbMaterialAmount(total,low,fine,detail);}
    shaped=vec2f(amount)*q/total;
  }
  if(opticalFamily()==4u){shaped=max(q-vec2f((1.-fine)*.06*detail),vec2f(0));}
  return shaped;
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
  let carrier=mix(textureSampleLevel(previousRest,linearClamp,clamp(uv,vec3f(0),vec3f(1)),0.),textureSampleLevel(restTex,linearClamp,clamp(uv,vec3f(0),vec3f(1)),0.),p.domain.x);
  var coord=carrier.xyz;
  let world=f32(u32(p.weather.y)%2u)==0.;
  if(world){coord=(x-vec3f(p.weather.w,0,p.weather.x*1.1))/vec3f(4800.,3200.,4200.)+vec3f(.41,.17,.29);}
  let filtered=p.weather.y>=2.&&opticalFamily()==1u;
  let footprint=64.*select(max(carrier.w,1.),1.,world);
  let lod=select(0.,clamp(log2(max(footprint*64./3200.,1.)),0.,6.),filtered);
  let fineLod=select(0.,clamp(lod+log2(4.3),0.,6.),filtered);
  let low=textureSampleLevel(basis,linearRepeat,fract(coord),lod).x;
  let n=textureSampleLevel(basis,linearRepeat,fract(coord*4.3),fineLod);
  // One material envelope: freezing partitions matter, never thresholds it twice.
  return shapeMatter(q,low,n.y,detail)*vec2f(p.bmin.w,p.bmax.w);
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
// Independent thin-plane attenuation provider. There is no density field in this profile.
fn analyticShadowT(x:vec3f)->f32 {
  if(x.y>=2200.){return 1.;}
  let caster=x+p.sun.xyz*((2200.-x.y)/max(p.sun.y,.03));
  let wave=.55+.35*sin((caster.x-p.weather.w)*.0012)*cos(caster.z*.001);
  let front=1.-.85*smoothstep(p.front.x-400.,p.front.x+400.,caster.x);
  let amount=max(0.,wave)*p.control.x*(1.-p.control.y)*front;
  return exp(-amount*1.8);
}
fn directT(x:vec3f)->vec2f {
  if(p.domain.w>1.5){return vec2f(1);}
  if(p.domain.w>.5){let t=analyticShadowT(x);return vec2f(t,pow(t,p.domain.y));}
  let tau=tauRay(x,p.sun.xyz,32u,1.);
  return exp(-vec2f(tau,tau*p.domain.y));
}
fn cacheT(x:vec3f)->vec2f {
  if(p.domain.w>.5){return directT(x);}
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
