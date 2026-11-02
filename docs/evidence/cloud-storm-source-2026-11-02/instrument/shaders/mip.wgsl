@group(0) @binding(0) var parent:texture_3d<f32>;
@group(0) @binding(1) var child:texture_storage_3d<rgba8unorm,write>;
@compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u){
  let n=textureDimensions(child);if(any(id>=n)){return;}var value=vec4f(0);
  for(var k=0u;k<8u;k++){let offset=vec3u(k&1u,(k>>1u)&1u,(k>>2u)&1u);value+=textureLoad(parent,vec3i(id*2u+offset),0);}
  textureStore(child,id,value*.125);
}
