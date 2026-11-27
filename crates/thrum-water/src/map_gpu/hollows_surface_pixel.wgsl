    var clip = view.view_proj * vec4(world, 1.0);
    if levels.x <= -1e30 { clip = vec4(0.0, 0.0, -1.0, 1.0); }
    // Hollows use neither the ordinary velocity nor the sampled-river data.
    // Leave the sheet-specific fields zero in either vertex layout.
    var output: VertexOut;
    output.clip = clip;
    output.world = world;
    output.rest = vertex.xy;
    output.current = current;
    output.previous = previous;
    output.body = slot;
    return output;
}

fn surface_pixel(point: vec2<f32>, slot: u32) -> SurfaceRow {
    let id = surface_body(slot);
    let answer = map_sample(point);
    var cell: Cell;
    cell.inside = answer.has_bed;
    cell.bed = answer.bed;
    return surface_row(point, cell, answer.row, id, answer.body == id);
}
