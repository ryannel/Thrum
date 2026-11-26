    var clip = view.view_proj * vec4(world, 1.0);
    if levels.x <= -1e30 { clip = vec4(0.0, 0.0, -1.0, 1.0); }
    return VertexOut(clip, world, vertex.xy, vec3(0.0), current, previous, slot, 0.0, vec4(0.0), 0.0);
}

fn surface_pixel(point: vec2<f32>, slot: u32) -> SurfaceRow {
    let id = surface_body(slot);
    let answer = map_sample(point);
    var cell: Cell;
    cell.inside = answer.has_bed;
    cell.bed = answer.bed;
    return surface_row(point, cell, answer.row, id, answer.body == id);
}
