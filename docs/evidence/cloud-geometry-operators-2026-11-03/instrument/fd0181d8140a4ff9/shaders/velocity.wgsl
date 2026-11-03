// Source functions/declarations precede this entry point; the original step is unreachable.
@compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u) {
  let dims=textureDimensions(oldQ);if(any(id>=dims)){return;}
  let flat=(id.z*dims.y+id.y)*dims.x+id.x;
  if(p.domain.z>.5){let n=(dims+7u)/8u;let b=id/8u;
    if(activeBricks[(b.z*n.y+b.y)*n.x+b.x]==0u){velocityField[flat]=vec4f(0);return;}}
  let x=pos((vec3f(id)+.5)/vec3f(dims));
  velocityField[flat]=flow(x);
}
