// Flow carries vertical displacement only. Use the cascade pyramid's
// reconstruction and the same two departures and their derivatives.
fn flow_column_at(point: vec2<f32>, extent: f32, layer: i32, lod: f32) -> Wave {
    let uv = point / extent;
    let height = periodic_field(uv, layer, lod).y + periodic_field(uv, layer + 4, lod).z;
    let slope = periodic_field(uv, layer + 1, lod).xy;
    let velocity = periodic_field(uv, layer + 2, lod);
    return Wave(vec3(0.0, height, 0.0), slope, vec3(0.0),
        vec3(velocity.x, velocity.y + velocity.w, velocity.z));
}

fn query_flow_column(point: QueryPoint, carriage: QueryCarriage, band: i32) -> CarriedColumn {
    let extent = FLOW_PATCHES[band];
    let lod = clamp(log2(max(1.0, point.radius * 512.0 / extent)), 0.0, 8.0);
    let layer = FLOW_FIRST + band * 5;
    let motion = carriage.motion;
    let gradient = carriage.gradient;
    var wave = flow_column_at(motion.departure, extent, layer, lod);
    var rate = -dot(wave.slope, motion.drift);
    wave.slope *= gradient.first;
    if motion.blend < 1.0 || any(gradient.blend != vec2(0.0)) {
        let second = flow_column_at(motion.second, extent, layer, lod);
        let difference = wave.displacement.y - second.displacement.y;
        let other_rate = -dot(second.slope, motion.second_drift);
        rate = mix(other_rate, rate, motion.blend) + motion.blend_rate * difference;
        wave.displacement.y = mix(second.displacement.y, wave.displacement.y, motion.blend);
        wave.slope = mix(second.slope * gradient.second, wave.slope, motion.blend)
            + difference * gradient.blend;
        wave.velocity = mix(second.velocity, wave.velocity, motion.blend);
    }
    return CarriedColumn(wave, rate, vec4(0.0));
}

fn query_flows(point: QueryPoint, carriage: QueryCarriage, out: ptr<function, Answer>) {
    if point.flow.z == 0.0 { return; }
    for (var band = 0; band < 2; band++) {
        let carried = query_flow_column(point, carriage, band);
        let wave = carried.wave;
        (*out).flow[band] = vec4(wave.displacement.y, wave.slope, wave.velocity.y + carried.advection_rate);
        (*out).flow_velocity[band] = vec4(wave.velocity, 0.0);
    }
}
