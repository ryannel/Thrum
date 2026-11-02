@group(0) @binding(12) var receiverOut:texture_storage_2d<rgba16float,write>;
@compute @workgroup_size(8,8) fn main(@builtin(global_invocation_id) id:vec3u) {
  let n=textureDimensions(receiverOut);
  if(any(id.xy>=n)) {
    return;
  }
  let uv=(vec2f(id.xy)+.5)/vec2f(n);
  let center=(p.bmin.xyz+p.bmax.xyz)*.5;
  let extent=(p.bmax.xyz-p.bmin.xyz)*2.;
  let x=vec3f(center.x+(uv.x-.5)*extent.x,2.,center.z+(uv.y-.5)*extent.z);
  let tau=tauRay(x,p.sun.xyz,64u,1.);
  textureStore(receiverOut,id.xy,vec4f(exp(-tau),tau,0,1));
}
