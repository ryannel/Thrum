// Direction of the Sun's beam entering water, from the air toward the bed.
fn water_sun_beam(toward_sun: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    return refract(-toward_sun, normal, 1.0 / 1.333);
}

// Horizontal travel per metre of vertical descent.
fn water_sun_slope(beam: vec3<f32>) -> vec2<f32> {
    return beam.xz / max(-beam.y, 0.05);
}

fn water_mean_sun_slope(toward_sun: vec3<f32>) -> vec2<f32> {
    return water_sun_slope(water_sun_beam(toward_sun, vec3(0.0, 1.0, 0.0)));
}

fn water_sun_exit(point: vec3<f32>, level: f32, toward_sun: vec3<f32>) -> vec3<f32> {
    let depth = level - point.y;
    let exit = point.xz - depth * water_mean_sun_slope(toward_sun);
    return vec3(exit.x, level, exit.y);
}
