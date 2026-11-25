@group(1) @binding(0) var overlay_output: texture_storage_2d_array<rgba16float,write>;
@group(1) @binding(1) var<uniform> overlay_rows: vec4<u32>;

fn overlay_direction(face: u32, uv: vec2<f32>) -> vec3<f32> {
    let q = 2.0 * uv - 1.0;
    switch face {
        case 0u: { return normalize(vec3(1.0, -q.y, -q.x)); }
        case 1u: { return normalize(vec3(-1.0, -q.y, q.x)); }
        case 2u: { return normalize(vec3(q.x, 1.0, q.y)); }
        case 3u: { return normalize(vec3(q.x, -1.0, -q.y)); }
        case 4u: { return normalize(vec3(q.x, -q.y, 1.0)); }
        default: { return normalize(vec3(-q.x, -q.y, -1.0)); }
    }
}

fn overlay_density(column: vec4<f32>, point: vec3<f32>, footprint: f32) -> f32 {
    if column.x <= 0.0 || cloud.physical.x <= 0.0 || cloud.physical.y == 0.0 {
        return column.x;
    }
    let uv = cloud_shape_uv(point);
    let eroded = cloud_erode(column.x, cloud.physical.x, cloud.physical.y,
        cloud_billow(uv, footprint), 0.5);
    return eroded * cloud_bank_shape(shape_noise, noise_sampler, point, uv, cloud.bank_noise,
        footprint, column.w, cloud_bank_weight(cloud.bank_noise.w));
}

@compute @workgroup_size(8, 4)
fn trace_overlay(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(overlay_output);
    if id.x >= size.x || id.y >= overlay_rows.y { return; }
    let pixel = vec2(id.x, id.y + overlay_rows.x);
    let ray = overlay_direction(id.z, (vec2<f32>(pixel) + 0.5) / vec2<f32>(size));
    let camera = atmosphere.camera_altitude.xyz;
    let start = camera - vec3(cloud.drift.x, 0.0, cloud.drift.y);
    var begin = 0.0;
    var end = cloud.heights.w;
    if abs(ray.y) > 1e-7 {
        let a = (cloud.heights.x - start.y) / ray.y;
        let b = (cloud.heights.y - start.y) / ray.y;
        begin = max(0.0, min(a, b));
        end = min(end, max(a, b));
    } else if start.y < cloud.heights.x || start.y > cloud.heights.y {
        end = 0.0;
    }
    var scattering = vec3(0.0);
    var transmission = 1.0;
    if end > begin {
        let zenith = environment_sky_radiance(vec3(0.0, 1.0, 0.0)) * 0.25;
        let nadir = environment_sky_radiance(vec3(0.0, -1.0, 0.0)) * 0.25;
        let base_sun = sun_irradiance(vec3(camera.x, cloud.heights.x, camera.z));
        let top_sun = sun_irradiance(vec3(camera.x, cloud.heights.y, camera.z));
        let phases = cloud_phases(ray, atmosphere.sun_lux.xyz);
        let angles = cloud_light_angles(-ray);
        var distance = begin;
        for (var i = 0u; i < 192u && distance < end && transmission > 0.0001; i++) {
            let step = min(end - distance, max(32.0, distance * 0.022));
            let at = distance + 0.5 * step;
            let point = start + ray * at;
            let footprint = max(step * 0.5, at / f32(size.y));
            let column = cloud_column(point);
            let placed = cloud_placed(point, footprint);
            let sigma = overlay_density(column, point, footprint) + placed.x;
            if sigma > 0.0 {
                let vertical = column.yz + placed.yz;
                let depths = cloud_sun_depths(camera + ray * at, point.y);
                let solar = mix(base_sun, top_sun, clamp((point.y - cloud.heights.x) /
                    (cloud.heights.y - cloud.heights.x), 0.0, 1.0));
                let direct = cloud_solar_source(phases, depths.xy, angles, -log(transmission),
                    smoothstep(500.0, 1000.0, at), cloud.scattering.xyz, cloud.erosion.z);
                let ambient_depth = 0.75 * (1.0 - cloud.erosion.z) * vertical;
                let ambient = zenith / (1.0 + ambient_depth.x) + nadir / (1.0 + ambient_depth.y);
                let depth = sigma * step;
                scattering += transmission * cloud_removed(depth) * (solar * direct + ambient);
                transmission *= exp(-depth);
            }
            distance += step;
        }
    }
    textureStore(overlay_output, vec2<i32>(pixel), i32(id.z), vec4(scattering, transmission));
}
