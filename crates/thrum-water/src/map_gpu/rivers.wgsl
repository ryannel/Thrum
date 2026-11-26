
// River-only specialization. The upper layer stores the per-corner wetting
// weight for its interval, or -1 for ordinary interpolation.
fn river_chart_cell(cell: Cell, k: u32, body: BodyRecord) -> vec4<f32> {
    let upper = i32(body.joins[0].w);
    let first = i32(body.level.w);
    let base = i32(body.layers.z);
    let corner = cell.corners[k];
    let lower = select(upper - 1, base, upper <= first);
    let a = textureLoad(bed_map, corner, lower, 0);
    if upper == 0 { return a; }
    let b = textureLoad(bed_map, corner, upper, 0);
    let weight = body.joins[1].w;
    if b.w < 0.0 { return mix(a, b, weight); }
    let bed = vec4(cell.heights[k], 0.0, 0.0, 0.0);
    if weight <= b.w && weight < 1.0 {
        let rise = select(1.0, weight / b.w, b.w > 0.0);
        return mix(a, bed, rise);
    }
    let rise = select(1.0, (weight - b.w) / (1.0 - b.w), b.w < 1.0);
    return mix(bed, b, rise);
}
