// A lent Sun tap has no receiver plane. Each selected cascade uses the first
// drawing and a comparison filter, as the view volume does.
fn point_sun_cascade(index: u32, point: vec3<f32>, filtered: sampler_comparison) -> f32 {
    let p = (shadows.light_view_proj[index] * vec4(point, 1.0)).xyz;
    let uv = p.xy * vec2(0.5, -0.5) + 0.5;
    if any(uv < vec2(0.0)) || any(uv >= vec2(1.0)) || p.z < 0.0 || p.z > 1.0 {
        return 1.0;
    }
    let bias = shadows.cascade_metrics[index].w;
    return textureSampleCompareLevel(shadow_maps, filtered, uv, index, p.z + bias);
}

fn point_sun_height(point: vec3<f32>) -> f32 {
    if shadows.horizon.x <= 0.0 { return 1.0; }
    let clip = shadows.height_view_proj * vec4(point.x, 0.0, point.z, 1.0);
    let uv = clip.xy * vec2(0.5, -0.5) + 0.5;
    if any(uv < vec2(0.0)) || any(uv >= vec2(1.0)) { return 1.0; }
    let caster = textureLoad(shadow_maps, vec2<i32>(uv * f32(SHADOW_HEIGHT_SIZE)),
        i32(SHADOW_HEIGHT_LAYER), 0);
    let depth = shadows.horizon.x * point.y + shadows.horizon.y;
    return select(0.0, 1.0, caster == 0.0 || caster <= depth);
}

fn point_sun_visibility(point: vec3<f32>, filtered: sampler_comparison) -> f32 {
    let choice = sun_cascade_choice(shadows.profile, shadows.cascade_metrics[0].x,
        distance(point, shadows.eye.xyz));
    if choice.fade >= 1.0 { return point_sun_height(point); }
    var visible = point_sun_cascade(choice.layer, point, filtered);
    if choice.blend > 0.0 {
        visible = mix(visible, point_sun_cascade(choice.layer + 1u, point, filtered), choice.blend);
    }
    if choice.fade > 0.0 {
        visible = mix(visible, point_sun_height(point), choice.fade);
    }
    return visible;
}
