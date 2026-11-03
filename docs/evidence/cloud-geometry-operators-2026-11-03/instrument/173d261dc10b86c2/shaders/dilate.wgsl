@group(0) @binding(0) var q:texture_3d<f32>;
@group(0) @binding(1) var<storage,read> support:array<u32>;
@group(0) @binding(2) var<storage,read_write> enabled:array<u32>;
@compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) brick:vec3u) {
  let n=(textureDimensions(q)+7u)/8u;if(any(brick>=n)){return;}
  var value=0u;
  for(var z=-1;z<=1;z++){for(var y=-1;y<=1;y++){for(var x=-1;x<=1;x++){
    let b=vec3i(brick)+vec3i(x,y,z);
    if(all(b>=vec3i(0))&&all(b<vec3i(n))){value|=support[(u32(b.z)*n.y+u32(b.y))*n.x+u32(b.x)];}
  }}}
  enabled[(brick.z*n.y+brick.y)*n.x+brick.x]=value;
}
