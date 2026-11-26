            water += raw.xyz * cell.weights[k];
        }
        row.water = water;
    }
    return chart_row(point, body, row, rows);
}

fn chart_contains(cell: Cell, body: BodyRecord) -> bool {
    for (var k = 0u; k < 4u; k++) {
        if cell.weights[k] > 1e-12 && textureLoad(bed_map, cell.corners[k], i32(body.layers.z), 0).w == 0.0 {
            return false;
        }
    }
    return true;
}

fn continued_chart(cell: Cell, body: BodyRecord, own: MapRow) -> MapRow {
    var row = own;
    var total = 0.0;
    for (var k = 0u; k < 4u; k++) {
        let water = textureLoad(bed_map, cell.corners[k], i32(body.layers.y), 0);
        let weight = cell.weights[k];
        if weight > 0.0 && water.w > 0.0 {
            total += weight;
            let gains = textureLoad(bed_map, cell.corners[k], i32(body.layers.z + 1u), 0);
            row = MapRow(mix(row.water, water.xyz, weight / total), mix(row.gains, gains, weight / total));
        }
    }
    return row;
}

fn map_sample(point: vec2<f32>) -> MapAnswer {
