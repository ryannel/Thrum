// Separate positive diffusion operator; its parameters are the immutable reaction uniform.
struct Params {camera:vec4f,focus:vec4f,sun:vec4f,bmin:vec4f,bmax:vec4f,weather:vec4f,control:vec4f,front:vec4f,image:vec4f,domain:vec4f, lifecycle:vec4f};
@group(0) @binding(0) var<uniform> p:Params;
@group(0) @binding(1) var source:texture_3d<f32>;
@group(0) @binding(2) var result:texture_storage_3d<rgba16float,write>;
fn moment(q:vec4f)->vec4f {return vec4f(q.xy,(q.x+q.y)*q.z,q.w);}
@compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u){
  let n=textureDimensions(result);if(any(id>=n)){return;}
  let pitch=(p.bmax.xyz-p.bmin.xyz)/vec3f(n);
  let a=1000.*p.weather.y/(pitch*pitch);
  let c=1.-2.*(a.x+a.y+a.z);
  let i=vec3i(id);let hi=vec3i(n)-1;
  var m=moment(textureLoad(source,i,0))*c;
  m+=a.x*(moment(textureLoad(source,clamp(i+vec3i(1,0,0),vec3i(0),hi),0))+moment(textureLoad(source,clamp(i-vec3i(1,0,0),vec3i(0),hi),0)));
  m+=a.y*(moment(textureLoad(source,clamp(i+vec3i(0,1,0),vec3i(0),hi),0))+moment(textureLoad(source,clamp(i-vec3i(0,1,0),vec3i(0),hi),0)));
  m+=a.z*(moment(textureLoad(source,clamp(i+vec3i(0,0,1),vec3i(0),hi),0))+moment(textureLoad(source,clamp(i-vec3i(0,0,1),vec3i(0),hi),0)));
  let q=m.x+m.y;let age=select(0.,m.z/max(q,1e-12),q>1e-10);
  textureStore(result,id,vec4f(m.xy,age,m.w));
}
