// Shared source/morphology geometry. Callers provide p and the immutable head/band tables.
struct ThermalHead {position:vec4f,shape:vec4f,identity:vec4f,motion:vec4f};
fn thermalLocal(x:vec3f,h:ThermalHead)->vec3f {
  let d=x-h.position.xyz;let c=h.identity.y;let s=h.identity.z;let a=h.identity.w;
  return vec3f((c*d.x+s*d.z)/a,d.y,(-s*d.x+c*d.z)*a);
}
fn thermalBaseRadius(h:ThermalHead)->f32 {
  return clamp(h.position.w-.08*max(h.position.y-1400.,0.),1100.,1600.);
}
fn thermalActivity()->f32 {return clamp(sqrt(max(p.control.z,0.)),0.,1.);}
fn thermalWind(y:f32)->vec3f {
  let shear=clamp((y-1100.)/8000.,0.,1.);let jet=smoothstep(5500.,9500.,y);
  return vec3f(p.weather.z*(1.+.6*shear)+18.*jet,0,1.1+3.*shear-5.*jet);
}
fn thermalStem(x:vec3f,h:ThermalHead)->vec4f {
  let root=vec2f(p.weather.w,p.weather.x*1.1);let L=max(h.position.y-1320.,80.);
  let s=clamp((x.y-1320.)/L,0.,1.);let g=s*s*(3.-2.*s);let slope=6.*s*(1.-s);
  let delta=h.position.xz-root;let center=root+g*delta;
  let radial=1.-smoothstep(.6,1.4,length(x.xz-center)/(.32*thermalBaseRadius(h)));
  let vertical=smoothstep(1100.,1350.,x.y)*(1.-smoothstep(h.position.y-.5*h.shape.y,h.position.y+.25*h.shape.y,x.y));
  let weight=radial*vertical*h.motion.w;if(weight<=0.){return vec4f(0);}
  let cap=1.-smoothstep(9000.,10300.,x.y);let up=h.motion.x*cap;
  let rootDot=vec2f(p.weather.z,1.1);let headDot=thermalWind(h.position.y).xz;
  let full=rootDot+g*(headDot-rootDot)+slope*(up-s*h.motion.x)*delta/L;
  let internal=full-thermalWind(x.y).xz;
  return vec4f(internal.x,up,internal.y,weight);
}
// Growth has prepared noise and upper-world fade; keep has shutdown activity, never a body union.
fn thermalFront(x:vec3f,h:ThermalHead,noise:texture_3d<f32>,repeat:sampler)->vec2f {
  let width=h.shape.w;let activity=thermalActivity();
  let frontCenter=h.position.y+.45*h.shape.y;
  if(width<=0.||h.motion.w<=1e-4||activity<=0.||frontCenter-.30*thermalBaseRadius(h)>=9100.){return vec2f(0);}
  let local=thermalLocal(x,h);let radius=length(local.xz)/h.position.w;
  let vertical=1.-smoothstep(.5,1.,abs(x.y-frontCenter)/width);
  if(vertical<=0.||radius>=1.25){return vec2f(0);}
  let front=vertical*(1.-smoothstep(.65,1.25,radius))*h.motion.w;
  let keep=vertical*(1.-smoothstep(.55,1.,radius))*h.motion.w*activity;
  let phase=h.identity.x*vec3f(.173,.317,.419);
  // Bounded Cb proof only: 24km x 14.5km x 24km / 128x80x128. Other grids must supply pitch.
  let nativePitch=vec3f(187.5,181.25,187.5);
  let shortHorizontal=h.position.w*min(h.identity.w,1./h.identity.w);
  let footprint=64.*.35*max(max(nativePitch.x,nativePitch.z)/shortHorizontal,nativePitch.y/h.shape.y);
  let lod=clamp(max(2.,log2(max(footprint,1.))),2.,6.);
  let coord=local/vec3f(h.position.w,h.shape.y,h.position.w)*.35+phase;
  let variation=.25+.75*preparedFieldNoise(noise,repeat,coord,lod);
  return vec2f(front*variation*(1.-smoothstep(8100.,9100.,x.y)),keep);
}
fn thermalMorphology(x:vec3f,noise:texture_3d<f32>,repeat:sampler)->vec3f {
  let band=u32(clamp(floor(x.y/500.),0.,31.));let offset=band*49u;let count=thermalBands[offset];
  var front=0.;var keep=0.;var stems=0.;
  for(var j=0u;j<count;j++){
    let h=thermalHeads[thermalBands[offset+1u+j]];
    let frontal=thermalFront(x,h,noise,repeat);
    front=max(front,frontal.x);keep=max(keep,frontal.y);stems+=thermalStem(x,h).w;
  }
  return vec3f(front,keep,min(1.,stems)*thermalActivity());
}
