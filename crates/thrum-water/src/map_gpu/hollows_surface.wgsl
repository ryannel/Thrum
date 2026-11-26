
fn hollows_vertex(vertex: vec4<f32>, slot: u32) -> VertexOut {
    let rectangle = hollows.rectangles[surface_body(slot)];
    let corner = vec2<f32>(CORNERS[u32(-vertex.z) - 1u]);
    let centre = vertex.xy + (vec2(0.5) - corner) * rectangle.origin_spacing.z;
    let levels = hollows_surface(rectangle, hollows_cell_at(rectangle, centre)).levels;
