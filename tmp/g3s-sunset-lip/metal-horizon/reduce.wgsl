// Parent 0 covers 2x2 source texels. Depth zero denotes an empty caster cell.
@group(0) @binding(0) var base: texture_depth_2d;
@group(0) @binding(1) var previous: texture_2d<f32>;
@group(0) @binding(2) var output: texture_storage_2d<r32float, write>;

@compute @workgroup_size(8,8)
fn depth_parent(@builtin(global_invocation_id) id: vec3<u32>) {
    let p = vec2<i32>(id.xy) * 2;
    let a = textureLoad(base,p,0);
    let b = textureLoad(base,p+vec2(1,0),0);
    let c = textureLoad(base,p+vec2(0,1),0);
    let d = textureLoad(base,p+vec2(1,1),0);
    textureStore(output,id.xy,vec4(max(max(a,b),max(c,d))));
}

@compute @workgroup_size(8,8)
fn mip_parent(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= textureDimensions(output)) { return; }
    let p = vec2<i32>(id.xy) * 2;
    let a = textureLoad(previous,p,0).x;
    let b = textureLoad(previous,p+vec2(1,0),0).x;
    let c = textureLoad(previous,p+vec2(0,1),0).x;
    let d = textureLoad(previous,p+vec2(1,1),0).x;
    textureStore(output,id.xy,vec4(max(max(a,b),max(c,d))));
}
