@group(0) @binding(16) var medium_coefficients: texture_2d_array<f32>;

fn medium_cell(xy: vec2<i32>, layer: u32, point: vec3<f32>) -> vec3<f32> {
    if layer >= VOLUME_NEAR_LAYERS { return vec3(0.0); }
    let fresh = textureLoad(medium_coefficients, xy, i32(layer), 0).rgb;
    var coefficients = fresh;
    if volume_history.previous_eye_valid.w != 0.0 {
        let clip = volume_history.previous_view_projection * vec4(point, 1.0);
        let distance = length(point - volume_history.previous_eye_valid.xyz);
        if clip.w > 0.0 && all(abs(clip.xy) < vec2(clip.w)) && distance < VOLUME_NEAR_DISTANCE {
            let uv = clip.xy / clip.w * vec2(0.5, -0.5) + vec2(0.5);
            let z = max(0.0, volume_coordinate(distance) - 0.5);
            let low = i32(floor(z));
            let high = min(low + 1, i32(VOLUME_NEAR_LAYERS - 1u));
            let old_low = textureSampleLevel(history_lo, lut_sampler, uv, i32(VOLUME_INTERVALS) + low, 0.0).rgb;
            let old_high = textureSampleLevel(history_lo, lut_sampler, uv, i32(VOLUME_INTERVALS) + high, 0.0).rgb;
            coefficients = mix(fresh, mix(old_low, old_high, fract(z)), 0.40);
        }
    }
    textureStore(history_lo_out, xy, i32(VOLUME_INTERVALS + layer), vec4(coefficients, 0.0));
    return coefficients;
}

fn joint_step(source: vec4<f32>, sigma: vec4<f32>, direct: vec4<f32>,
              medium: vec3<f32>, sun_visibility: f32, delta: f32) -> Segment {
    let lit_medium = medium.y * direct * sun_visibility + vec4(medium.z);
    return integrate_segment(source + lit_medium, sigma + vec4(medium.x), delta);
}
