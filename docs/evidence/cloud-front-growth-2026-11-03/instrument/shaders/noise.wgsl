@group(0) @binding(0) var dest:texture_storage_3d<rgba8unorm,write>;
fn hash3(p:vec3f)->vec3f {
  return fract(sin(vec3f(dot(p,vec3f(127.1,311.7,74.7)),dot(p,vec3f(269.5,183.3,246.1)),dot(p,vec3f(113.5,271.9,124.6))))*43758.5453);
}
fn vn(p:vec3f)->f32 {
  let i=floor(p);
  let f=fract(p);
  let u=f*f*(3.-2.*f);
  var v=0.;
  for(var k=0;k<8;k++) {
    let o=vec3f(f32(k&1),f32((k>>1)&1),f32((k>>2)&1));
    let w=mix(vec3f(1)-u,u,o);
    v+=hash3((i+o)%vec3f(8.)).x*w.x*w.y*w.z;
  }
  return v;
}
fn worley(p:vec3f,period:f32)->f32 {
  let cell=floor(p);
  let f=fract(p);
  var d=2.;
  for(var z=-1;z<=1;z++) {
    for(var y=-1;y<=1;y++) {
      for(var x=-1;x<=1;x++) {
        let o=vec3f(f32(x),f32(y),f32(z));
        let wrapped=(cell+o+period)%vec3f(period);
        d=min(d,length(o+hash3(wrapped)-f));
      }
    }
  }
  return 1.-clamp(d/1.12,0.,1.);
}
@compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u) {
  let n=textureDimensions(dest);
  if(any(id>=n)) {
    return;
  }
  let uv=(vec3f(id)+.5)/vec3f(n);
  let w=worley(uv*8.,8.);
  let w2=worley(uv*16.,16.);
  let a=vn(uv*8.);
  let f=.6*a+.25*vn(uv*16.)+.15*vn(uv*32.);
  let pw=clamp((f*.7+w*.3-.12)/.75,0.,1.);
  textureStore(dest,id,vec4f(pw,w,w2,f));
}
