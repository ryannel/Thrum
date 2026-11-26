    var swell = 0.0;
    if surf_weight == 0.0 && row.gains.x * water.heights.x > 0.0 && (!sea || vertex.z <= 2.0) {
        swell = long_at(rest,vertex.z).displacement.y * row.gains.x;
    }
