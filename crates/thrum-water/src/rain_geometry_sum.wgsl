    let rain_wave = rain_geometry(point, spacing, row, motion);
    f.displacement.y += rain_wave.x;
    f.velocity.y += rain_wave.y;
