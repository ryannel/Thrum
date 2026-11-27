    var value = work[index].xz * weight;
    if cell.low == REST_DEPTH && cell.high == step.depth_index {
        let old = old_state(at);
        value += vec2(old.x, old.y - WATER_GRAVITY * step.seconds * old.x) * (1.0 - cell.blend);
    }
