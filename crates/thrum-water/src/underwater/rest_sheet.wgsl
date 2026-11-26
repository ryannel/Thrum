fn lens_river_data(_a: VertexOut, _b: VertexOut, _c: VertexOut) -> vec4<f32> {
    return vec4(0.0);
}

fn lens_sheet_height(face: LensFace, _weights: vec3<f32>, _point: vec2<f32>, rest: vec2<f32>, raster: f32, current: f32) -> f32 {
    if raster <= current { return current; }
    return select(current, raster, surface_pixel(rest, face.ownership & 255u).owned);
}
