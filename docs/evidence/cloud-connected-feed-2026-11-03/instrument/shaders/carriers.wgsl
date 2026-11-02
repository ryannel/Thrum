// Appended to source helpers, with the source entry renamed. One authority for u.
@group(0) @binding(15) var secondRest:texture_3d<f32>;
@group(0) @binding(16) var firstResult:texture_storage_3d<rgba16float,write>;
@group(0) @binding(17) var secondResult:texture_storage_3d<rgba16float,write>;
fn restSeed(x:vec3f)->vec3f{return (x-vec3f(p.weather.w,0,p.weather.x*1.1))/vec3f(4800,3200,4200)+vec3f(.41,.17,.29);}
fn chartSample(t:texture_3d<f32>,back:vec3f)->vec3f{
 let uv=coords(back);if(any(uv<vec3f(0))||any(uv>vec3f(1))){return restSeed(back);}return textureSampleLevel(t,linearClamp,uv,0.).xyz;
}
fn chartValue(t:texture_3d<f32>,backs:array<vec3f,7>,step:vec3f)->vec4f{
 let scale=vec3f(4800,3200,4200);
 let dx=(chartSample(t,backs[1])-chartSample(t,backs[2]))*scale/(2.*step.x);
 let dy=(chartSample(t,backs[3])-chartSample(t,backs[4]))*scale/(2.*step.y);
 let dz=(chartSample(t,backs[5])-chartSample(t,backs[6]))*scale/(2.*step.z);
 let metric=clamp(sqrt(3.)*max(max(length(dx),length(dy)),length(dz)),1.,128.);
 return vec4f(chartSample(t,backs[0]),metric);
}
@compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u){
 let n=textureDimensions(firstResult);if(any(id>=n)){return;}
 let x=pos((vec3f(id)+.5)/vec3f(n));let seed=vec4f(restSeed(x),sqrt(3.));
 if(p.domain.w==5.){textureStore(firstResult,id,seed);textureStore(secondResult,id,seed);return;}
 let step=(p.bmax.xyz-p.bmin.xyz)/vec3f(n);
 var points=array<vec3f,7>(x,x+vec3f(step.x,0,0),x-vec3f(step.x,0,0),x+vec3f(0,step.y,0),x-vec3f(0,step.y,0),x+vec3f(0,0,step.z),x-vec3f(0,0,step.z));
 for(var i=0;i<7;i++){points[i]-=velocity(points[i])*p.weather.y;}
 let tick=u32(p.lifecycle.x);let period=u32(p.lifecycle.z);let half=period/2u;
 var a=seed;var b=seed;
 if(tick%period!=0u){a=chartValue(oldRest,points,step);}
 if((tick+half)%period!=0u){b=chartValue(secondRest,points,step);}
 textureStore(firstResult,id,a);textureStore(secondResult,id,b);
}
