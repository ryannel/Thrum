fn query_flow_column(point: QueryPoint, carriage: QueryCarriage, band: i32) -> CarriedColumn {
    let extent = FLOW_PATCHES[band];
    let lod = clamp(log2(max(1.0, point.radius * 512.0 / extent)), 0.0, 8.0);
    // Column sampling takes a band ordinal. Rain planes make the flow
    // start potentially unaligned, so pass the actual layer here.
    return carried_column_layer(carriage.motion, extent, FLOW_FIRST + band * 5, lod, carriage.gradient);
}

fn query_flows(index: u32, point: QueryPoint, carriage: QueryCarriage, out: ptr<function, Answer>) {
    let gains = points[index].flow;
    if gains.z == 0.0 { return; }
    for (var band = 0; band < 2; band++) {
        let carried = query_flow_column(point, carriage, band);
        let wave = carried.wave;
        (*out).flow[band] = vec4(wave.displacement.y, wave.slope, wave.velocity.y + carried.advection_rate);
        (*out).flow_velocity[band] = vec4(wave.velocity, 0.0);
    }
}
