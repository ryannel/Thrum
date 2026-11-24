struct VolumeHistory {
    previous_view_projection: mat4x4<f32>,
    previous_eye_valid: vec4<f32>,
    current_view_forward_phase: vec4<f32>,
}

fn volume_shadow_jitter(id: vec2<u32>, layer: u32, phase: f32) -> f32 {
    let bits = (id.x * 1973u) ^ (id.y * 9277u) ^ (layer * 26699u);
    let seed = f32((bits * 747796405u) >> 16u) / 65536.0;
    // Keep the point away from the cell faces on the first populated frame.
    return 0.125 + 0.75 * fract(seed + phase * 0.61803398875);
}

