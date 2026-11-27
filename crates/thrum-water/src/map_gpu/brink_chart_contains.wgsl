fn chart_contains(cell: Cell, body: BodyRecord) -> bool {
    for (var k = 0u; k < 4u; k++) {
        if cell.weights[k] > 1e-12 && textureLoad(bed_map, cell.corners[k], i32(body.layers.z), 0).w <= 0.0 {
            return false;
        }
    }
    return true;
}
