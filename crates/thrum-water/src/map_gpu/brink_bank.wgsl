// The lower chart continues to the rock without acquiring the upper chart's
// brink centres. Only the corners in the receiving chart supply water.
fn chart_brink_bank(cell: Cell, body: BodyRecord) -> bool {
    var has_receiver = false;
    for (var k = 0u; k < 4u; k++) {
        if cell.weights[k] <= 1e-12 { continue; }
        let footprint = textureLoad(bed_map, cell.corners[k], i32(body.layers.z), 0).w;
        if footprint == 0.0 { return false; }
        has_receiver = has_receiver || footprint > 0.0;
    }
    return has_receiver;
}
