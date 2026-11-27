
fn query_gains(point: QueryPoint) -> vec4<f32> {
    let row = point.level_current_bed;
    let depth = select(1e30, row.x - row.w, (point.flags & 1u) != 0u);
    let swash = flow_swatch(point.gains * settings.heights, point.flow.xy, settings.swash_margin.x, settings.swash_margin.y);
    return limit_wind_with_flow(point.gains, settings.heights, depth, swash);
}
