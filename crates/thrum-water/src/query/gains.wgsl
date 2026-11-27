fn query_gains(point: QueryPoint) -> vec4<f32> {
    let row = point.level_current_bed;
    let depth = select(1e30, row.x - row.w, (point.flags & 1u) != 0u);
    return limit_gains(point.gains, depth, settings.heights, settings.swash_margin.x, settings.swash_margin.y);
}

