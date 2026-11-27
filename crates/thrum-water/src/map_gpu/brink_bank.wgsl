// The lower chart continues to the rock without acquiring the upper chart's
// brink centres. Only the corners in the receiving chart supply water.
fn chart_brink_bank(cell: Cell, body: BodyRecord) -> bool {
    var side = vec4(0.0);
    for (var k = 0u; k < 4u; k++) {
        if cell.weights[k] <= 1e-12 { continue; }
        side[k] = textureLoad(bed_map, cell.corners[k], i32(body.layers.z), 0).w;
    }
    return side.x * side.y < 0.0 || side.x * side.z < 0.0
        || side.y * side.w < 0.0 || side.z * side.w < 0.0;
}
