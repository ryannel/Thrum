@group(0) @binding(6) var lightOut:texture_storage_3d<rgba16float,write>;
@compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u) {
  let n=textureDimensions(lightOut);
  if(any(id>=n)) {
    return;
  }
  let uv=(vec3f(id)+.5)/vec3f(n);
  let x=mix(p.bmin.xyz,p.bmax.xyz,uv);
  let tau=tauRay(x,p.sun.xyz,32u,1.);
  var a=0.;
  let dirs=array<vec3f,4>(vec3f(.6,.8,0.),vec3f(-.6,.8,0.),vec3f(0.,.8,.6),vec3f(0.,.8,-.6));
  for(var i=0u;i<4u;i++) {
    a+=exp(-tauRay(x,dirs[i],20u,1.));
  }
  textureStore(lightOut,id,vec4f(exp(-tau),exp(-p.domain.y*tau),a*.25,1.));
}
