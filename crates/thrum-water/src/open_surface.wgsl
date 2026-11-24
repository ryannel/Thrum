fn surface_vertex(point: vec2<f32>, slot: u32) -> SurfaceRow {
    return SurfaceRow(water.level_dt_reset.x, water.level_dt_reset.w, water.gains, -1e30,
        true, 0.0, 0.0, 0.0);
}
fn tail_gain(row: SurfaceRow) -> f32 { return 1.0; }
fn surface_opacity(row: SurfaceRow, world: vec3<f32>, view_y: f32) -> f32 {
    return 1.0;
}
fn surface_pixel(point: vec2<f32>, slot: u32) -> SurfaceRow {
    return surface_vertex(point, slot);
}
