// A coherent periodic field can cross its lattice edge with no renewal.
fn cascade_departure(point: vec2<f32>, current: vec2<f32>, seconds: f32) -> vec2<f32> {
    return point - current * seconds;
}

struct CascadeMotion {
    departure: vec2<f32>,
    drift: vec2<f32>,
    blend: f32,
}

fn cascade_motion(point: vec2<f32>, current: vec2<f32>, track: vec4<f32>, seconds: f32) -> CascadeMotion {
    if track.w == 0.0 {
        return CascadeMotion(cascade_departure(point, current, seconds), current, 1.0);
    }
    let axis = u32(track.w) - 1u;
    var coordinate = point;
    coordinate[axis] = track.x;
    var drift = vec2(0.0);
    drift[axis] = track.y;
    return CascadeMotion(cascade_departure(coordinate, drift, seconds), drift, track.z);
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
