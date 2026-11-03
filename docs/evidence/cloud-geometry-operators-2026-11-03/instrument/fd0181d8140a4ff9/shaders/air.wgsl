@group(0) @binding(8) var airOut:texture_storage_2d<rgba16float,write>;
@compute @workgroup_size(8,8) fn main(@builtin(global_invocation_id) id:vec3u) {
  let n=textureDimensions(airOut);
  if(any(id.xy>=n)) {
    return;
  }
  let uv=(vec2f(id.xy)+.5)/vec2f(n);
  let h=uv.y*uv.y*100000.;
  let mu=uv.x*2.-1.;
  let dir=vec3f(sqrt(max(1.-mu*mu,0.)),mu,0.);
  textureStore(airOut,id.xy,vec4f(airRayT(vec3f(0,h,0),dir),1.));
}
