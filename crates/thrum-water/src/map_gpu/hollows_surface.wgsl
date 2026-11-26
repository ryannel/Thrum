
fn hollows_vertex(vertex: vec4<f32>, slot: u32) -> VertexOut {
    let rectangle = hollows.rectangles[surface_body(slot)];
    let corner = vec2<f32>(CORNERS[u32(-vertex.z) - 1u]);
    let centre = vertex.xy + (vec2(0.5) - corner) * rectangle.origin_spacing.z;
    let levels = hollows_surface(rectangle, hollows_cell_at(rectangle, centre));
    let world = vec3(vertex.x, levels.x, vertex.y);
    let current = view.unjittered_view_proj * vec4(world, 1.0);
    let previous = view.previous_view_proj * vec4(vertex.x, levels.y, vertex.y, 1.0);
    var clip = view.view_proj * vec4(world, 1.0);
    if levels.x <= -1e30 { clip = vec4(0.0, 0.0, -1.0, 1.0); }
    return VertexOut(clip, world, vertex.xy, vec3(0.0), current, previous, slot, 0.0, vec4(0.0), 0.0);
}

fn surface_pixel(point: vec2<f32>, slot: u32) -> SurfaceRow {
    let id = surface_body(slot);
    let answer = hollows_map_sample(point);
    var cell: Cell;
    cell.inside = answer.has_bed;
    cell.bed = answer.bed;
    return surface_row(point, cell, answer.row, id, answer.body == id);
}
