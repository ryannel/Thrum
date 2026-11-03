// Positive phase/moment mixing and native-field normal motion. Common helpers are prepended.
struct Params {camera:vec4f,focus:vec4f,sun:vec4f,bmin:vec4f,bmax:vec4f,weather:vec4f,control:vec4f,front:vec4f,image:vec4f,domain:vec4f,lifecycle:vec4f};
@group(0) @binding(0) var<uniform> p:Params;
@group(0) @binding(1) var source:texture_3d<f32>;
@group(0) @binding(2) var result:texture_storage_3d<rgba16float,write>;
@group(0) @binding(3) var basisNoise:texture_3d<f32>;
@group(0) @binding(4) var repeatSampler:sampler;
@group(0) @binding(5) var<storage,read> thermalHeads:array<ThermalHead,48>;
@group(0) @binding(6) var<storage,read> thermalBands:array<u32>;
@compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u){
 let n=textureDimensions(result);if(any(id>=n)){return;}
 let pitch=(p.bmax.xyz-p.bmin.xyz)/vec3f(n);let i=vec3i(id);let hi=vec3i(n)-1;
 let c=textureLoad(source,i,0);
 let neighbours=array<vec4f,6>(textureLoad(source,clamp(i+vec3i(1,0,0),vec3i(0),hi),0),textureLoad(source,clamp(i-vec3i(1,0,0),vec3i(0),hi),0),textureLoad(source,clamp(i+vec3i(0,1,0),vec3i(0),hi),0),textureLoad(source,clamp(i-vec3i(0,1,0),vec3i(0),hi),0),textureLoad(source,clamp(i+vec3i(0,0,1),vec3i(0),hi),0),textureLoad(source,clamp(i-vec3i(0,0,1),vec3i(0),hi),0));
 var occupied=c.x+c.y;for(var j=0;j<6;j++){occupied+=neighbours[j].x+neighbours[j].y;}
 if(occupied<=0.){textureStore(result,id,vec4f(0));return;}
 var growth=0.;var erosion=0.;
 if(p.front.z>3.5){
  let x=mix(p.bmin.xyz,p.bmax.xyz,(vec3f(id)+.5)/vec3f(n));
  let fronts=thermalMorphology(x,basisNoise,repeatSampler);
  growth=18.*sqrt(p.control.z)*p.control.x*fronts.x;
  let deficit=clamp(p.control.y+.6*(1.-p.control.x),0.,1.);
  let dryNoise=.3+.7*textureSampleLevel(basisNoise,repeatSampler,(x-vec3f(p.weather.w,0,p.weather.x*1.1))/vec3f(2200,1700,2200)+vec3f(.13,.37,.19),1.).x;
  erosion=((.5+12.*p.control.y*p.control.y)*deficit+8.*(1.-fronts.y))*dryNoise*(1.-fronts.z);
 }
 let kappa=select(0.,1000.,p.front.w>.5);
 textureStore(result,id,applyFrontMotion(c,neighbours,pitch,p.weather.y,kappa,growth,erosion));
}
