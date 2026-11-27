    var value = work[index].xz * weight;
    if step.level >> 8u != 0u {
        value *= carriage_weight(index);
    }
    if cell.low == REST_DEPTH && cell.high == step.depth_index && step.level >> 16u == 0u {
        let old = old_state(at);
        value += vec2(old.x, old.y - WATER_GRAVITY * step.seconds * old.x) * (1.0 - cell.blend);
    }
