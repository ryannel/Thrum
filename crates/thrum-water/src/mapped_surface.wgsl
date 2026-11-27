fn reflection_bed(point: vec2<f32>) -> f32 {
    let cell = map_cell(point);
    return select(-1e30, cell.bed, cell.inside);
}

fn surface_body(slot: u32) -> u32 {
    return select(slot - 1u, map.size_count_sea.w, slot == 0u);
}
