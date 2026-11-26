fn lens_river_data(a: VertexOut, b: VertexOut, c: VertexOut) -> vec4<f32> {
    return vec4(a.river.x, b.river.x, c.river.x, a.river.y);
}

fn lens_sheet_height(face: LensFace, weights: vec3<f32>, point: vec2<f32>, rest: vec2<f32>, raster: f32, current: f32) -> f32 {
    let slot = face.ownership & 255u;
    let flags = vec3(face.ownership) >> vec3(8u, 9u, 10u);
    let transformed = dot(weights, vec3<f32>(flags & vec3(1u))) > 0.0;
    let row = surface_pixel(river_ownership_point(point, rest, transformed), slot);
    if !row.owned { return current; }
    let spacing = face.river.w;
    let weight = river_refinement(point, surface_body(slot), spacing);
    return max(current, river_sheet_height(point, spacing, row, raster, dot(weights, face.river.xyz), transformed, weight));
}
