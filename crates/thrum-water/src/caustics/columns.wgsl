@vertex fn caustic_columns_vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let point = vec2<f32>(vec2(index & 1u, index >> 1u)) * 4.0 - 1.0;
    return vec4(point, 0.0, 1.0);
}

@fragment fn caustic_columns_fragment(@builtin(position) clip: vec4<f32>) -> @location(0) vec4<f32> {
    let bounds = caustic_draw.column_bounds;
    if bounds.z == 0.0 { return caustic_draw.outside_column; }
    let point = bounds.xy + clip.xy * (bounds.zw / 512.0);
    let row = surface_pixel(point, caustic_slot(point));
    return vec4(row.level, 0.0, 0.0, select(0.0, 1.0, row.owned));
}
