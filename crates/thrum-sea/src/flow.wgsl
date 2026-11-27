// The flow model has vertical displacement only. Keep the material velocity
// and rest slope and use the ordinary five planes and mip reduction.
@compute @workgroup_size(8,8)
fn pack_flow(@builtin(global_invocation_id) id: vec3<u32>) {
    let value = cascades[(id.z / 5u) * 65536u + id.y * 256u + id.x];
    let rate = displacement_high(vec3(value.velocity.y)).x;
    let vertical = Field(vec4(0.0, value.displacement.y, 0.0, 1.0),
        vec4(value.slope.xy, 0.0, 0.0), vec4(value.velocity.x, rate, value.velocity.z, value.velocity.y - rate));
    textureStore(cascade_out, vec2<i32>(id.xy), i32(id.z), plane(vertical, id.z % 5u));
}
