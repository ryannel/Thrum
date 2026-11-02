@group(0) @binding(0) var curlOut:texture_storage_3d<rgba16float,write>;
fn h(p:vec3f)->vec3f {
  return fract(sin(vec3f(dot(p,vec3f(127.1,311.7,74.7)),dot(p,vec3f(269.5,183.3,246.1)),dot(p,vec3f(113.5,271.9,124.6))))*43758.5453);
}
fn curl(p:vec3f)->vec3f {
  let i=floor(p);
  let f=fract(p);
  let u=f*f*(3.-2.*f);
  let du=6.*f*(1.-f);
  var gx=vec3f(0);
  var gy=vec3f(0);
  var gz=vec3f(0);
  for(var k=0;k<8;k++) {
    let o=vec3f(f32(k&1),f32((k>>1)&1),f32((k>>2)&1));
    let w=mix(vec3f(1)-u,u,o);
    let dw=(2.*o-1.)*du;
    let v=h((i+o)%vec3f(8.));
    gx+=v*dw.x*w.y*w.z;
    gy+=v*w.x*dw.y*w.z;
    gz+=v*w.x*w.y*dw.z;
  }
  return vec3f(gy.z-gz.y,gz.x-gx.z,gx.y-gy.x);
}
@compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u) {
  let n=textureDimensions(curlOut);
  if(any(id>=n)) {
    return;
  }
  let p=(vec3f(id)+.5)/vec3f(n)*8.;
  let v=(curl(p)+.5*curl(p*2.))/6.;
  textureStore(curlOut,id,vec4f(v,1.));
}
