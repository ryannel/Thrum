struct RiverReceiver {
    current_weight_gradient: vec4<f32>,
    track: vec4<f32>,
    current_gradient: vec4<f32>,
    track_gradient: vec4<f32>,
}

struct RiverQueryMotion {
    motion: CascadeMotion,
    gradient: CarriageGradient,
}

fn joined_carriage_jacobian(a: mat2x2<f32>, b: mat2x2<f32>,
    delta: vec2<f32>, weight: f32, slope: vec2<f32>) -> mat2x2<f32> {
    return mat2x2<f32>(mix(a[0], b[0], weight) + delta * slope.x,
        mix(a[1], b[1], weight) + delta * slope.y);
}

fn river_query_motion(point: QueryPoint, motion: CascadeMotion, gradient: CarriageGradient) -> RiverQueryMotion {
    if point.carry.z == 0.0 { return RiverQueryMotion(motion, gradient); }
    let weight = point.carry.w;
    let slope = point.receiver.current_weight_gradient.zw;
    let current = point.receiver.current_weight_gradient.xy;
    let receiver = cascade_motion(point.position, current, point.receiver.track,
        settings.swash_margin.z, settings.swash_margin.w);
    var receiving_point = point;
    receiving_point.level_current_bed = vec4(0.0, current, 0.0);
    receiving_point.track = point.receiver.track;
    receiving_point.current_gradient = point.receiver.current_gradient;
    receiving_point.track_gradient = point.receiver.track_gradient;
    let other = carriage_gradient(receiving_point);
    return RiverQueryMotion(
        mix_cascade_motion(receiver, motion, weight),
        CarriageGradient(
            joined_carriage_jacobian(other.first, gradient.first, motion.departure - receiver.departure, weight, slope),
            joined_carriage_jacobian(other.second, gradient.second, motion.second - receiver.second, weight, slope),
            mix(other.blend, gradient.blend, weight) + (motion.blend - receiver.blend) * slope,
        ),
    );
}
