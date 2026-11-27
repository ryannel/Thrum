// The evenly scattered term remains in water_path. These terms replace the
// beam's even scattering with its directional phase and sampled visibility.
struct ShaftBeam {
    direction: vec3<f32>,
    phase: f32,
    loss: vec3<f32>,
    source: vec3<f32>,
}

fn shaft_beam(ray: vec3<f32>, surface: vec3<f32>) -> ShaftBeam {
    let sun = environment_sun_direction();
    let refracted = -water_sun_beam(sun, vec3(0.0, 1.0, 0.0));
    // The forward lobe of the sea-water phase function. Unit mean over 4π.
    let g = 0.90;
    let phase = (1.0 - g * g) / pow(1.0 + g * g - 2.0 * g * clamp(dot(ray, refracted), -1.0, 1.0), 1.5);
    let c = optics.extinction_sight.xyz;
    let k = optics.downwelling_mirror.xyz;
    let rate = max(c - k, vec3(0.0)) / max(refracted.y, 0.05);
    let source = water_sun(surface) * max(sun.y, 0.0) * water.scatter.rgb * c / 3.14159265;
    return ShaftBeam(refracted, phase, k + rate, source);
}

fn shaft_directional(beam: ShaftBeam, length_m: f32, start_depth: f32, end_depth: f32) -> vec3<f32> {
    let entry = beam.loss * start_depth;
    let end = optics.extinction_sight.xyz * length_m + beam.loss * end_depth;
    let delta = abs(end - entry);
    let series = 1.0 - 0.5 * delta + delta * delta / 6.0;
    let exact = (1.0 - exp(-delta)) / max(delta, vec3(0.00001));
    let integral = length_m * exp(-min(entry, end)) * select(exact, series, delta < vec3(0.01));
    return beam.source * (beam.phase - 1.0) * integral;
}
