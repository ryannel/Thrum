// Point visibility for volumes and refracted surface exits. There is no
// receiver plane at these points. Use the same filtered comparison at
// the exit for both readers.
fn point_sun_visibility(point: vec3<f32>, filtered: sampler_comparison) -> f32 {
    let extent = vec2<f32>(textureDimensions(shadow_maps));
    for (var index = 0u; index < 2u; index++) {
        let clip = shadows.light_view_proj[index] * vec4(point, 1.0);
        let p = clip.xyz / clip.w;
        let uv = p.xy * vec2(0.5, -0.5) + 0.5;
        if any(uv < vec2(0.0)) || any(uv >= vec2(1.0)) || p.z < 0.0 || p.z > 1.0 {
            continue;
        }
        // The far map owns upstream coverage cropped from the near map.
        if index == 0u && shadows.near_partition.x > 0.0 { continue; }
        let bias = select(shadows.penumbra.w, shadows.penumbra.z, index == 0u) + 4e-7;
        return textureSampleCompareLevel(shadow_maps, filtered, uv, index, p.z + bias);
    }
    if shadows.horizon.x <= 0.0 { return 1.0; }
    let clip = shadows.height_view_proj * vec4(point.x, 0.0, point.z, 1.0);
    let uv = clip.xy * vec2(0.5, -0.5) + 0.5;
    if any(uv < vec2(0.0)) || any(uv >= vec2(1.0)) { return 1.0; }
    let caster = textureLoad(shadow_maps, vec2<i32>(uv * extent), 2, 0);
    let depth = shadows.horizon.x * point.y + shadows.horizon.y;
    return select(0.0, 1.0, caster == 0.0 || caster <= depth);
}
