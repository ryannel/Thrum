// Point visibility for volumes and refracted surface exits. There is no
// receiver plane at these points. Gather the four comparisons in one
// texture operation, then reconstruct the mapped edge at the exit.
fn point_sun_visibility(point: vec3<f32>, filtered: sampler) -> f32 {
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
        let casters = textureGather(shadow_maps, filtered, uv, index);
        let bias = select(shadows.penumbra.w, shadows.penumbra.z, index == 0u) + 4e-7;
        let taps = select(vec4(0.0), vec4(1.0), casters <= vec4(p.z + bias));
        let blend = fract(uv * extent - 0.5);
        return mix(mix(taps.w, taps.z, blend.x), mix(taps.x, taps.y, blend.x), blend.y);
    }
    if shadows.horizon.x <= 0.0 { return 1.0; }
    let clip = shadows.height_view_proj * vec4(point.x, 0.0, point.z, 1.0);
    let uv = clip.xy * vec2(0.5, -0.5) + 0.5;
    if any(uv < vec2(0.0)) || any(uv >= vec2(1.0)) { return 1.0; }
    let caster = textureLoad(shadow_maps, vec2<i32>(uv * extent), 2, 0);
    let depth = shadows.horizon.x * point.y + shadows.horizon.y;
    return select(0.0, 1.0, caster == 0.0 || caster <= depth);
}
