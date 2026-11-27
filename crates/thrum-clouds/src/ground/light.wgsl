struct GroundOptics {
    droplet: vec4<f32>, // forward share, broad weight, broad mean cosine, mean cosine
    phase: array<f32, GROUND_PHASE_ENTRIES>,
}

// The last channel holds the depth contributed by the ground rows alone.
fn ground_sun_depths(point: vec3<f32>, surface: f32) -> vec4<f32> {
    let size = vec3<f32>(textureDimensions(ground_band_light));
    let region = ground_light_window.region;
    let uv = ((point.xz - region.xy) / (region.zw - region.xy) * (size.xy - 1.0) + 0.5) / size.xy;
    let edge = min(point.xz - region.xy, region.zw - point.xz);
    let w = ground_cubic(min(edge.x, edge.y) / ground_light_window.top_border.y);
    let fraction = clamp((point.y - surface) / ground_light_window.top_border.x, 0.0, 1.0);
    let layer = (fraction * (size.z - 1.0) + 0.5) / size.z;
    let band = textureSampleLevel(ground_band_light, noise_sampler, vec3(uv, layer), 0.0);
    let bottom = textureSampleLevel(ground_band_light, noise_sampler, vec3(uv, 0.5 / size.z), 0.0).r;
    let depths = vec4(band.r, max(band.r, bottom), band.b, band.a);
    if w >= 1.0 && fraction < 1.0 { return depths; }
    let cloud = cloud_window_depths(point, point.y, ground_frame.upper_heights).xy;
    let share = select(w, 0.0, fraction >= 1.0);
    return mix(vec4(cloud, 1.0, 0.0), depths, share);
}

fn ground_sky_limit(ray: vec3<f32>, true_sun: vec3<f32>) -> vec4<f32> {
    // Diffuse hemisphere averages include the glowing side of twilight.
    // Above a dark horizon that average can light droplets more than the
    // local sky. Limit their ambient source to the same azimuth at twilight.
    let weight = 1.0 - smoothstep(0.04, 0.11, true_sun.y);
    if weight == 0.0 || ground_frame.has_active_rows == 0u { return vec4(0.0); }
    let count = arrayLength(&cloud_ambient) - 3u;
    let turn = fract(atan2(ray.z, ray.x) / 6.28318530718 + 1.0) * f32(count);
    let lower = u32(turn) % count;
    let upper = (lower + 1u) % count;
    let sky = mix(cloud_ambient[lower + 2u].rgb, cloud_ambient[upper + 2u].rgb, fract(turn));
    return vec4(0.85 * sky, weight);
}

fn ground_light(point: vec3<f32>, ray: vec3<f32>, true_sun: vec3<f32>,
                distance: f32, transmission: f32, coarse: bool, sky_limit: vec4<f32>) -> GroundLight {
    let surface = ground_surface(point.xz);
    var result = GroundLight(0.0, 0.0, vec3(0.0), 1.0);
    var depths = vec3(0.0);
    let angle = dot(ray - true_sun, ray - true_sun) * 0.25;
    let phase_index = sqrt(sqrt(clamp(angle, 0.0, 1.0))) * f32(GROUND_PHASE_ENTRIES - 1u);
    let lo = min(u32(phase_index), GROUND_PHASE_ENTRIES - 2u);
    let angles = vec4(mapping.path_border.y, -ray.y, 0.0, 1.0);
    for (var row = 0u; row < arrayLength(&ground_rows); row++) {
        let local = ground_row(point, surface, ground_rows[row], coarse);
        if local.x == 0.0 { continue; }
        if result.extinction == 0.0 {
            depths = ground_sun_depths(point, surface.x).xyz;
            result.surface_visibility = depths.z;
        }
        let droplet = ground_phase[row].droplet;
        let first = mix(ground_phase[row].phase[lo], ground_phase[row].phase[lo+1u], phase_index-f32(lo));
        let phases = vec3(first, cloud_hg_phase(angle, droplet.w * droplet.z), 1.0);
        let kernel = cloud_sun_term(phases, depths.x, depths.xy, 1.0, angles, distance, transmission, droplet);
        let ambient_depth = 0.75 * (1.0 - droplet.w) * local.yz;
        result.extinction += local.x;
        result.solar_kernel += local.x * kernel;
        result.ambient += local.x * (cloud_ambient[0].rgb / (1.0 + ambient_depth.x)
                                  + cloud_ambient[1].rgb / (1.0 + ambient_depth.y));
    }
    if sky_limit.w > 0.0 && result.extinction > 0.0 {
        // At night the droplets receive diffuse light from the whole sky,
        // and that incident source is dimmer than a direct view of the horizon.
        let night = 1.0 - smoothstep(-0.27, -0.12, true_sun.y);
        let horizon = sky_limit.rgb * result.extinction;
        let ceiling = horizon * mix(1.0, 0.18, night);
        let floor = horizon * (0.10 * night);
        let twilight_ambient = clamp(result.ambient, floor, ceiling);
        result.ambient = mix(result.ambient, twilight_ambient, sky_limit.w);
    }
    return result;
}

fn ground_near_weight(distance: f32) -> f32 {
    return 1.0 - cloud_smooth((distance - GROUND_RAMP_START_M) /
                             (CLOUD_RAMP_END_M - GROUND_RAMP_START_M));
}
