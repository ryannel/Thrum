struct Params  {
  camera:vec4f, focus:vec4f, sun:vec4f, bmin:vec4f, bmax:vec4f, weather:vec4f, control:vec4f, front:vec4f, image:vec4f, domain:vec4f, lifecycle:vec4f
}
;
@group(0) @binding(0) var<uniform> p:Params;
@group(0) @binding(1) var oldQ:texture_3d<f32>;
@group(0) @binding(2) var forwardQ:texture_3d<f32>;
@group(0) @binding(3) var reverseQ:texture_3d<f32>;
@group(0) @binding(4) var outputQ:texture_storage_3d<rgba16float,write>;
@group(0) @binding(5) var basis:texture_3d<f32>;
@group(0) @binding(6) var linearClamp:sampler;
@group(0) @binding(7) var oldRest:texture_3d<f32>;
struct Emitter  {
  location:vec4f,timing:vec4f
}
;
@group(0) @binding(8) var<storage,read> emitters:array<Emitter,32>;
@group(0) @binding(9) var curlField:texture_3d<f32>;
@group(0) @binding(10) var linearRepeat:sampler;
@group(0) @binding(11) var<storage,read> activeBricks:array<u32>;
@group(0) @binding(12) var<storage,read> velocityField:array<vec4f>;
struct ThermalHead {position:vec4f,shape:vec4f,identity:vec4f,motion:vec4f};
@group(0) @binding(13) var<storage,read> thermalHeads:array<ThermalHead,48>;
@group(0) @binding(14) var<storage,read> thermalBands:array<u32>;
fn finiteEvaporation(q:vec2f,E:vec2f,dt:f32)->vec2f {return max(q-E*dt,vec2f(0));}
fn condensingSource(q:f32,vy:f32,moist:f32,lift:f32,height:f32)->f32 {return .0008*max(vy,0.)*moist*min(lift,1.)*max(q,0.)*smoothstep(.01,.08,q)*height;}
fn rollingVelocity(d:vec3f,R:f32,H:f32,A:f32)->vec3f {
  let r2=dot(d.xz,d.xz)/(R*R);let E=exp(-r2-d.y*d.y/(H*H));
  return vec3f(2.*A*d.y*d.x/(H*H),2.*A*(1.-r2),2.*A*d.y*d.z/(H*H))*E;
}
fn materialCoreTarget(d:vec3f,m:vec4f)->vec3f {return vec3f(m.y*d.x,m.x+m.z*d.y,m.y*d.z);}
fn sharedWind(y:f32,kind:u32)->vec3f {
  let layer=weatherLayer();let shear=clamp((y-layer.x)/8000.,0.,1.);
  let jet=select(0.,smoothstep(5500.,9500.,y),kind==1u&&p.front.z>.5);
  return vec3f(p.weather.z*(select(1.,3.,kind==4u)+.6*shear)+18.*jet,0,1.1+3.*shear-5.*jet);
}
fn stemTarget(x:vec3f,h:ThermalHead)->vec4f {
  let root=vec2f(p.weather.w,p.weather.x*1.1);let L=max(h.position.y-1320.,80.);
  let s=clamp((x.y-1320.)/L,0.,1.);let g=s*s*(3.-2.*s);let slope=6.*s*(1.-s);
  let delta=h.position.xz-root;let center=root+g*delta;
  let r0=clamp(h.position.w-.08*max(h.position.y-1400.,0.),1100.,1600.);
  let radial=1.-smoothstep(.6,1.4,length(x.xz-center)/(.32*r0));
  let vertical=smoothstep(1100.,1350.,x.y)*(1.-smoothstep(h.position.y-.5*h.shape.y,h.position.y+.25*h.shape.y,x.y));
  let weight=radial*vertical*h.motion.w;if(weight<=0.){return vec4f(0);}
  let cap=1.-smoothstep(9000.,10300.,x.y);let up=h.motion.x*cap;
  let rootDot=vec2f(p.weather.z,1.1);let headDot=sharedWind(h.position.y,1u).xz;
  let full=rootDot+g*(headDot-rootDot)+slope*(up-s*h.motion.x)*delta/L;
  let internal=full-sharedWind(x.y,1u).xz;
  return vec4f(internal.x,up,internal.y,weight);
}
struct ThermalFlow {roll:vec3f,spread:vec2f,core:vec3f,weight:f32,stem:vec3f,stemWeight:f32};
fn thermalVelocity(x:vec3f)->ThermalFlow {
  let band=u32(clamp(floor(x.y/500.),0.,31.));let offset=band*49u;let count=thermalBands[offset];var v=vec3f(0);var spread=vec2f(0);var core=vec3f(0);var weight=0.;var stem=vec3f(0);var stemWeight=0.;
  for(var j=0u;j<count;j++){
    let h=thermalHeads[thermalBands[offset+1u+j]];let d=x-h.position.xyz;
    if(p.front.z>2.5){let feed=stemTarget(x,h);stem+=feed.xyz*feed.w;stemWeight+=feed.w;}
    let c=h.identity.y;let s=h.identity.z;let a=h.identity.w;
    let local=vec3f((c*d.x+s*d.z)/a,d.y,(-s*d.x+c*d.z)*a);
    let r2=dot(local.xz,local.xz)/(h.position.w*h.position.w);let y2=d.y*d.y/(h.shape.y*h.shape.y);
    if(r2>9.||y2>9.){continue;}
    if(p.front.z>1.5){let w=(1.-smoothstep(.65,1.25,sqrt(r2+y2)))*h.motion.w;core+=w*materialCoreTarget(d,h.motion);weight+=w;}
    var roll=rollingVelocity(local,h.position.w,h.shape.y,h.shape.x);
    roll+=rollingVelocity(local-vec3f(.65*h.position.w,.15*h.shape.y,0),.55*h.position.w,.75*h.shape.y,.45*h.shape.x);
    let horizontal=vec2f(c*a*roll.x-s*roll.z/a,s*a*roll.x+c*roll.z/a);
    v+=vec3f(horizontal.x,roll.y,horizontal.y);
    if(x.y>9000.&&h.position.y>7000.){
    let capSupport=1.-smoothstep(2.,3.,sqrt(r2));
    let arrival=smoothstep(7000.,9000.,h.position.y)*select(h.shape.z,h.motion.w,p.front.z>1.5);
    let radial=local.xz/sqrt(dot(local.xz,local.xz)+160000.);
    let pushed=vec2f(c*a*radial.x-s*radial.y/a,s*a*radial.x+c*radial.y/a);
    spread+=16.*arrival*exp(-r2/9.)*capSupport*pushed;
    }
  }
  return ThermalFlow(v,spread,core,weight,stem,stemWeight);
}
fn hash3(x:vec3f)->f32 {
  return fract(sin(dot(x,vec3f(127.1,311.7,74.7)))*43758.5453);
}
fn valueNoise(x:vec3f)->f32 {
  let i=floor(x);
  let f=fract(x);
  let u=f*f*(3.-2.*f);
  return mix(mix(mix(hash3(i),hash3(i+vec3f(1,0,0)),u.x),mix(hash3(i+vec3f(0,1,0)),hash3(i+vec3f(1,1,0)),u.x),u.y),mix(mix(hash3(i+vec3f(0,0,1)),hash3(i+vec3f(1,0,1)),u.x),mix(hash3(i+vec3f(0,1,1)),hash3(i+vec3f(1,1,1)),u.x),u.y),u.z);
}
fn pos(uv:vec3f)->vec3f {
  return mix(p.bmin.xyz,p.bmax.xyz,uv);
}
fn coords(x:vec3f)->vec3f {
  return (x-p.bmin.xyz)/(p.bmax.xyz-p.bmin.xyz);
}
fn qAt(t:texture_3d<f32>,x:vec3f)->vec4f {
  let uv=coords(x);
  if(any(uv<vec3f(0))||any(uv>vec3f(1))) {
    return vec4f(0);
  }
  return textureSampleLevel(t,linearClamp,uv,0.);
}
fn weatherLayer()->vec3f {
  let kind=u32(p.control.w);
  if(kind==1u) {
    return vec3f(1100,12500,9600);
  }
  if(kind==2u) {
    return vec3f(1100,2600,2600);
  }
  if(kind==3u) {
    return vec3f(1000,1900,1900);
  }
  if(kind==4u) {
    return vec3f(8500,11200,11200);
  }
  return vec3f(1100,3900,3900);
}
fn flow(x:vec3f)->vec4f {
  let layer=weatherLayer();
  let kind=u32(p.control.w);
  let center=select(vec2f(0),vec2f(p.weather.w,p.weather.x*1.1),kind==1u);
  let radial=length(x.xz-center);
  let roll=valueNoise(vec3f(x.x*.001,0,x.z*.001));
  var up=(7.+14.*roll)*p.control.z*exp(-radial*radial/4200000.);
  up*=smoothstep(layer.x-300.,layer.x+150.,x.y)*(1.-smoothstep(layer.z-150.,layer.z+550.,x.y));
  if(kind==2u||kind==3u) {
    up*=.13;
  }
  if(kind==4u) {
    up=-1.5;
  }
  if(kind==1u) {
    up=select(36.+18.*roll,4.+2.*roll,p.front.z>.5)*p.control.z*exp(-radial*radial/5800000.);
    up*=smoothstep(layer.x-300.,layer.x+150.,x.y)*(1.-smoothstep(layer.z-600.,layer.z+700.,x.y));
  }
  let outflow=select(0.,40.*smoothstep(layer.z-600.,layer.z+700.,x.y)*exp(-radial*radial/36000000.)*p.front.y,kind==1u);
  let delta=x.xz-center;
  let radialDir=delta/sqrt(dot(delta,delta)+160000.);
  let shear=clamp((x.y-layer.x)/8000.,0.,1.);
  let c=textureSampleLevel(curlField,linearRepeat,fract((x-vec3f(p.weather.w,0.,p.weather.x*1.1))/vec3f(2400.)),0.).xyz;
  let eddy=c*vec3f(18.,12.,18.)*smoothstep(layer.x-150.,layer.x+200.,x.y);
  var convective=0.;
  var internal=eddy+vec3f(radialDir.x*outflow,up,radialDir.y*outflow);
  if(kind==4u){
    let axis=normalize(vec2f(p.weather.z*(3.+.6*shear),1.1+3.*shear));
    let along=axis*dot(eddy.xz,axis);
    let horizontal=along*.35+(eddy.xz-along)*.06;
    internal=vec3f(horizontal.x,-1.5+eddy.y*.06,horizontal.y);
  }
  if(kind==1u&&p.front.z>.5){
    let head=thermalVelocity(x);let cap=1.-smoothstep(layer.z-600.,layer.z+700.,x.y);
    convective=max(head.roll.y,0.)*cap;
    let spread=head.spread*smoothstep(layer.z-600.,layer.z+700.,x.y)*p.front.y;
    internal=eddy*.5+vec3f(spread.x,up,spread.y)+head.roll*vec3f(1,cap,1);
    if(p.front.z>1.5){
      let residual=max(0.,1.-head.weight);let Z=max(1.,head.weight);
      var environment=eddy*.5+vec3f(0,up,0)+head.roll*vec3f(1,cap,1);
      var environmentAscent=max(head.roll.y,0.)*cap;
      if(p.front.z>2.5){let residualStem=max(0.,1.-head.stemWeight);let stemZ=max(1.,head.stemWeight);
        environment=(head.stem+residualStem*environment)/stemZ;
        environmentAscent=(head.stem.y+residualStem*environmentAscent)/stemZ;
      }
      internal=(head.core*vec3f(1,cap,1)+residual*environment)/Z+vec3f(spread.x,0,spread.y);
      convective=(max(head.core.y,0.)*cap+residual*environmentAscent)/Z;
      if(p.front.z>2.5){
        // Activity gates the normalized result once: overlapping weights cannot cancel release.
        let activity=clamp(sqrt(max(p.control.z,0.)),0.,1.);
        let freeEnvironment=eddy*.5+vec3f(0,up,0)+head.roll*vec3f(1,cap,1);
        internal=mix(freeEnvironment,internal,activity);
        convective=mix(max(head.roll.y,0.)*cap,convective,activity);
      }
    }
  }
  if(kind==1u){let scale=min(1.,100./max(length(internal),1e-6));internal*=scale;convective*=scale;}
  return vec4f(internal+sharedWind(x.y,kind),convective);
}
fn velocity(x:vec3f)->vec3f {return flow(x).xyz;
}
fn reaction(q:vec2f,s:vec2f,d:vec2f,dt:f32)->vec2f {
  let k=s+d;
  let e=exp(-k*dt);
  return q*e+(s/max(k,vec2f(1e-8)))*(1.-e);
}
// These isolated layer proofs author a finite bank, not a sheet cut by the simulation box.
fn bankRadius(x:vec3f)->f32 {
  // Authored world footprint; resizing a storage domain must not resize the bank.
  return length(x.xz/vec2f(4000.,3200.));
}
fn bankSupport(x:vec3f)->f32 {return 1.-smoothstep(.65,1.,bankRadius(x));}
// Seed finite fall-streaks; ongoing supply is restricted to their ice heads.
fn cirrusPattern(x:vec3f)->vec2f {
  var seed=0.;var supply=0.;
  for(var i=0u;i<8u;i++){
    let e=emitters[i];if(e.location.w<=0.){continue;}
    let d=x.xz-e.location.xy;let L=e.timing.x;let c=e.timing.y;let s=e.timing.z;
    let u=c*d.x+s*d.y;let v=-s*d.x+c*d.y;let t=clamp(u/L,0.,1.);
    if(u< -700.||u>L+300.){continue;}
    let bend=(select(-1.,1.,i%2u==0u))*.09*L*t*t;
    let width=e.location.z*(1.-.5*t);
    let height=e.timing.w-(460.+390.*f32(i%3u)/2.)*t;
    let H=130.+25.*f32(i%3u);
    let ends=smoothstep(-.10*L,.03*L,u)*(1.-smoothstep(.72*L,L,u));
    let irregular=.35+.65*valueNoise(vec3f(u/700.,v/310.,f32(i)*7.1));
    let tail=exp(-pow((v-bend)/width,2.)-pow((x.y-height)/H,2.))*ends*irregular;
    let head=exp(-pow(u/340.,2.)-pow(v/e.location.z,2.)-pow((x.y-e.timing.w)/H,2.));
    seed+=e.location.w*(tail+.65*head);supply+=e.location.w*head;
  }
  return clamp(vec2f(seed,supply),vec2f(0),vec2f(1));
}
fn localSource(x:vec3f)->vec3f {
  let kind=u32(p.control.w);
  let layer=weatherLayer();
  if(kind==4u){let ci=cirrusPattern(x);let front=smoothstep(p.front.x-400.,p.front.x+400.,x.x);return vec3f(ci.y*.006*p.control.x*min(p.control.z,1.),ci.x,front*.045);}
  var s=0.;
  var spatial=0.;
  for(var i=0u;i<32u;i++) {
    let e=emitters[i];
    if(e.location.w<.5) {
      continue;
    }
    let age=p.weather.x-e.timing.x;
    let packet=kind==1u&&p.front.z>.5;
    if(packet&&p.front.z<2.5&&e.location.w<1.5){continue;}
    let rise=select(e.timing.y,4.,packet);
    let hold=select(e.timing.z,8.,packet);
    let fall=select(e.timing.w,10.,packet);
    let window=smoothstep(0.,rise,age)*(1.-smoothstep(rise+hold,rise+hold+fall,age));
    if(packet&&window<=0.){continue;}
    let center=e.location.xy*select(1.,2.,packet)+select(vec2f(0),vec2f(p.weather.w,p.weather.x*1.1),kind==1u);
    let radius=e.location.z*select(1.,2.,kind==1u);
    let r=length((x.xz-center)/vec2f(radius));
    let mask=1.-smoothstep(.25,1.25,r);
    spatial=max(spatial,mask);
    s+=mask*window;
  }
  let edgeNoise=textureSampleLevel(basis,linearRepeat,fract(x/vec3f(2200.,1700.,2200.)+vec3f(.3,.17,.29)),0.).x;
  let baseBand=exp(-pow((x.y-(layer.x+120.))/190.,2.));
  let pulseSupply=s;
  s*=baseBand*smoothstep(.3,.65,edgeNoise)*1.8;
  if(kind==1u){
    // A dense lower feed must survive its short residence in this fast updraft.
    let band=exp(-pow((x.y-(layer.x+220.))/280.,2.))*smoothstep(layer.x-50.,layer.x+100.,x.y);
    s=pulseSupply;
    s*=band*(.45+.55*smoothstep(.25,.65,edgeNoise))*2.*select(1.,.2,p.front.z>2.5);
  }
  if(kind==2u||kind==3u) {
    let cells=valueNoise(vec3f(x.x*.00065,0,x.z*.00065));
    let coverage=select(smoothstep(.27,.67,cells),.8+.2*cells,kind==3u);
    s=coverage*exp(-pow((x.y-(layer.x+220.))/330.,2.))*(.55+.45*valueNoise(x*.003+vec3f(p.weather.x*.007,0,0)));
    let bank=bankSupport(x);
    s*=bank;
    spatial=bank;
  }
  let dryFront=smoothstep(p.front.x-400.,p.front.x+400.,x.x);
  return vec3f(s*.065*p.control.x*min(p.control.z,1.),spatial,dryFront*.045);
}
fn backtracedRest(x:vec3f,dt:f32)->vec3f {
  let back=x-velocity(x)*dt;let uv=coords(back);
  if(any(uv<vec3f(0))||any(uv>vec3f(1))){return x/vec3f(4800.,3200.,4200.)+vec3f(.41,.17,.29);}
  return textureSampleLevel(oldRest,linearClamp,uv,0.).xyz;
}
@compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u) {
  let dims=textureDimensions(outputQ);
  if(any(id>=dims)) {
    return;
  }
  let uv=(vec3f(id)+.5)/vec3f(dims);
  let x=pos(uv);
  let stage=i32(p.domain.w);
  let kind=u32(p.control.w);
  let layer=weatherLayer();
  if(stage==0||stage==-1) {
    var q=0.;
    if(stage==0) {
      let radial=exp(-dot(x.xz,x.xz)/6000000.);
      let height=smoothstep(layer.x-50.,layer.x+190.,x.y)*(1.-smoothstep(layer.y-1100.,layer.y,x.y));
      let n=textureSampleLevel(basis,linearRepeat,fract(x/vec3f(4800.,3200.,4200.)+vec3f(.41,.17,.29)),0.);
      q=clamp((n.x-.28)/.30,0.,1.)*height*radial*.86;
      if(kind==1u){
        let lowBody=smoothstep(layer.x-50.,layer.x+180.,x.y)*(1.-smoothstep(layer.x+900.,layer.x+1700.,x.y));
        q=clamp((n.x-.23)/.25,0.,1.)*lowBody*radial*.95*(1.-smoothstep(3000.,4200.,length(x.xz)));
      }
      if(kind==2u||kind==3u) {
        q=height*select(smoothstep(.3,.7,n.x),.65+.35*n.x,kind==3u)*bankSupport(x);
      }
      if(kind==4u) {
        q=cirrusPattern(x).x*.28*p.control.x;
      }
    }
    let ice=select(0.,q,kind==4u);
    textureStore(outputQ,id,vec4f(q-ice,ice,0.,0.));
    return;
  }
  if(stage==4||stage==5) {
    var rest=x/vec3f(4800.,3200.,4200.)+vec3f(.41,.17,.29);
    var metric=sqrt(3.);
    if(stage==4) {
      let back=x-velocity(x)*p.weather.y;
      let b=coords(back);
      if(all(b>=vec3f(0))&&all(b<=vec3f(1))) {
        rest=textureSampleLevel(oldRest,linearClamp,b,0.).xyz;
        if(kind==1u){
          let step=(p.bmax.xyz-p.bmin.xyz)/vec3f(textureDimensions(oldRest));
          let scale=vec3f(4800.,3200.,4200.);
          let dx=(backtracedRest(x+vec3f(step.x,0,0),p.weather.y)-backtracedRest(x-vec3f(step.x,0,0),p.weather.y))*scale/(2.*step.x);
          let dy=(backtracedRest(x+vec3f(0,step.y,0),p.weather.y)-backtracedRest(x-vec3f(0,step.y,0),p.weather.y))*scale/(2.*step.y);
          let dz=(backtracedRest(x+vec3f(0,0,step.z),p.weather.y)-backtracedRest(x-vec3f(0,0,step.z),p.weather.y))*scale/(2.*step.z);
          metric=clamp(sqrt(3.)*max(max(length(dx),length(dy)),length(dz)),1.,128.);
        }
      }
    }
    textureStore(outputQ,id,vec4f(rest,metric));
    return;
  }
  if(p.domain.z>.5) {
    let bricks=(dims+7u)/8u;let b=id/8u;
    if(activeBricks[(b.z*bricks.y+b.y)*bricks.x+b.x]==0u){textureStore(outputQ,id,vec4f(0));return;}
  }
  let dt=p.weather.y;
  var field=vec4f(0);
  if(p.camera.w>.5){field=velocityField[(id.z*dims.y+id.y)*dims.x+id.x];}else{field=flow(x);}
  let vel=field.xyz;
  if(stage==1) {
    textureStore(outputQ,id,qAt(oldQ,x-vel*dt));
    return;
  }
  if(stage==2) {
    textureStore(outputQ,id,qAt(forwardQ,x+vel*dt));
    return;
  }
  let old=textureLoad(oldQ,vec3i(id),0);
  let fwd=textureLoad(forwardQ,vec3i(id),0);
  let rev=textureLoad(reverseQ,vec3i(id),0);
  var adv=fwd.xy+.5*(old.xy-rev.xy);
  let donor=coords(x-vel*dt)*vec3f(dims)-.5;
  let base=vec3i(floor(donor));
  var lo=vec2f(1e4);
  var hi=vec2f(-1e4);
  var donorValid=true;
  for(var k=0;k<8;k++) {
    let off=vec3i(k&1,(k>>1)&1,(k>>2)&1);
    let cell=base+off;
    var value=vec2f(0);
    if(all(cell>=vec3i(0))&&all(cell<vec3i(dims))) {
      value=textureLoad(oldQ,cell,0).xy;
    }
    else {
      donorValid=false;
    }
    lo=min(lo,value);
    hi=max(hi,value);
  }
  adv=select(fwd.xy,clamp(adv,lo,hi),donorValid);
  let source=localSource(x);
  let flank=1.-source.y;
  var decay=.0015+p.control.y*.018+source.z+flank*.0025;
  decay+=(1.-smoothstep(layer.x-100.,layer.x+45.,x.y))*.06;
  decay+=smoothstep(layer.y,layer.y+600.,x.y)*.008;
  if(kind==2u||kind==3u){decay+=.08*smoothstep(.75,1.,bankRadius(x));}
  var add=vec2f(source.x,0.);
  var phaseDecay=vec2f(decay);
  if(kind==1u){
    phaseDecay=vec2f(.0015+p.control.y*.008+source.z+flank*.0015,.0004+p.control.y*.002+source.z*.5+flank*.0005);
    let below=(1.-smoothstep(layer.x-100.,layer.x+45.,x.y))*.2;
    let above=smoothstep(layer.y,layer.y+600.,x.y)*.008;
    phaseDecay+=vec2f(below+above);
  }
  if(kind==4u) {
    add=vec2f(0.,source.x);
    phaseDecay*=.7;
  }
  var evaporation=vec2f(0);
  if(kind==1u&&p.front.z>.5){
    let height=smoothstep(layer.x+100.,layer.x+400.,x.y)*(1.-smoothstep(layer.z-1500.,layer.z-500.,x.y));
    add.x+=condensingSource(adv.x,field.w,p.control.x,p.control.z,height);
    let shear=clamp((x.y-layer.x)/8000.,0.,1.);
    let jet=smoothstep(5500.,9500.,x.y);
    let flow=vec3f(p.weather.w*(1.+.6*shear)+18.*jet*p.weather.x,0,p.weather.x*(1.1+3.*shear-5.*jet));
    let pocket=valueNoise((x-flow)/vec3f(1100.,800.,1100.)+vec3f(.13,p.weather.x*.001,.37));
    let deficit=clamp(p.control.y+.6*(1.-p.control.x),0.,1.);
    let E=(.0008+.008*p.control.y*p.control.y)*deficit*(.2+1.0*smoothstep(.25,.75,pocket));
    evaporation=vec2f(E,E*.08);
  }
  var q=finiteEvaporation(reaction(max(adv,vec2f(0)),add,phaseDecay,dt),evaporation,dt);
  if(kind==1u) {
    let band=smoothstep(layer.z-550.,layer.z-200.,x.y)*(1.-smoothstep(layer.z+150.,layer.z+550.,x.y));
    let rate=.022*band*p.front.y*(.35+.65*clamp(vel.y/40.,0.,1.));
    let transfer=min(min(q.x,1.-q.y),q.x*(1.-exp(-rate*dt)));
    q+=vec2f(-transfer,transfer);
  }
  let previousAge=fwd.z;
  let total=q.x+q.y;
  let survived=finiteEvaporation(max(adv,vec2f(0))*exp(-phaseDecay*dt),evaporation,dt);
  let amountAdded=max(total-(survived.x+survived.y),0.);
  let age=select(0.,(previousAge*max(total-amountAdded,0.)+dt*total)/max(total,1e-6),total>.002);
  textureStore(outputQ,id,vec4f(clamp(q,vec2f(0),vec2f(1)),min(age,6000.),amountAdded));
}
