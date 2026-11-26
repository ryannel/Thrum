// A coherent periodic field can cross its lattice edge with no renewal.
fn cascade_departure(point: vec2<f32>, current: vec2<f32>, seconds: f32) -> vec2<f32> {
    return point - current * seconds;
}

struct CascadeMotion {
    departure: vec2<f32>,
    second: vec2<f32>,
    drift: vec2<f32>,
    blend: f32,
    blend_rate: f32,
    second_drift: vec2<f32>,
}

// Offsets are in seconds, followed by the first sample's weight and rate.
fn cascade_renewal(seconds: f32, span: f32) -> vec4<f32> {
    let a = fract(seconds / span);
    let b = fract(a + 0.5);
    let u = 2.0 * min(a, 1.0 - a);
    let weight = u * u * (3.0 - 2.0 * u);
    let rate = 12.0 * u * (1.0 - u) * select(-1.0, 1.0, a < 0.5) / span;
    return vec4((a - 0.5) * span, (b - 0.5) * span, weight, rate);
}

fn cascade_motion(point: vec2<f32>, current: vec2<f32>, track: vec4<f32>, seconds: f32, period: f32) -> CascadeMotion {
    if track.w == 0.0 {
        return CascadeMotion(cascade_departure(point, current, seconds), point, current, 1.0, 0.0, vec2(0.0));
    }
    if track.w < 0.0 {
        let reference = track.xy;
        let residual = current - reference;
        let base = cascade_departure(point, reference, seconds);
        if all(abs(residual) < vec2(1e-6)) {
            return CascadeMotion(base, point, reference, 1.0, 0.0, vec2(0.0));
        }
        let renewal = cascade_renewal(seconds, min(period, track.z));
        return CascadeMotion(base - residual * renewal.x,
            base - residual * renewal.y, current, renewal.z, renewal.w, current);
    }
    let axis = u32(track.w) - 1u;
    var coordinate = point;
    coordinate[axis] = track.x;
    var drift = vec2(0.0);
    drift[axis] = track.y;
    return CascadeMotion(cascade_departure(coordinate, drift, seconds), point, drift, track.z, 0.0, vec2(0.0));
}

// Two readings of a periodic field, half a cycle apart. The weight of a
// reading and its derivative are zero when that reading wraps.
struct Carriage {
    first: vec2<f32>,
    second: vec2<f32>,
    weight: f32,
}

fn carried_points(point: vec2<f32>, current: vec2<f32>, phase: vec2<f32>) -> Carriage {
    let a = fract(phase.x);
    let b = fract(a + 0.5);
    let weight = smoothstep(0.0, 0.5, min(a, 1.0 - a));
    return Carriage(
        point - current * ((a - 0.5) * phase.y),
        point - current * ((b - 0.5) * phase.y),
        weight,
    );
}
