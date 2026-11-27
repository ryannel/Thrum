@vertex fn caustic_columns_vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let point = vec2<f32>(vec2(index & 1u, index >> 1u)) * 4.0 - 1.0;
    return vec4(point, 0.0, 1.0);
}

@fragment fn caustic_columns_fragment(@builtin(position) clip: vec4<f32>) -> @location(0) vec4<f32> {
    let window = caustic_draw.window;
    let point = window.xy + clip.xy * (window.z / 512.0);
    let row = surface_pixel(point, caustic_slot(point));
    let beam = water_sun_beam(caustic_draw.sun.xyz, vec3(0.0, 1.0, 0.0));
    let rate = CAUSTIC_BEAM_DIFFERENCE / max(-beam.y, 0.05);
    return vec4(row.level, rate, 0.0, select(0.0, 1.0, row.owned));
}
