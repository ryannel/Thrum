fn surface_vertex(point: vec2<f32>, slot: u32) -> SurfaceRow {
    return SurfaceRow(water.level_dt_reset.x, water.level_dt_reset.w, water.gains, -1e30, true);
}
fn tail_gain(row: SurfaceRow) -> f32 { return 1.0; }
fn shallow_colour(row: SurfaceRow, world: vec3<f32>, facing: f32, incident: vec3<f32>, fresnel: f32, light: vec3<f32>) -> vec3<f32> {
    return light;
}
fn surface_pixel(point: vec2<f32>, slot: u32) -> SurfaceRow {
    return surface_vertex(point, slot);
}
