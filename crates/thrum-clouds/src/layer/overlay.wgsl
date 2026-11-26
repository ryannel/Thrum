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
            let sigma = cloud_eroded_density(column.x, point, footprint, cloud.physical.x, column.w, cloud.density_profile.x) + placed.x;
            if sigma > 0.0 {
                let vertical = column.yz + placed.yz;
                let depths = cloud_sun_depths(camera + ray * at, point.y);
                let solar = mix(base_sun, top_sun, clamp((point.y - cloud.heights.x) /
                    (cloud.heights.y - cloud.heights.x), 0.0, 1.0));
                let source = cloud_source(solar, phases, depths.xy, vertical, angles, at, transmission);
                let depth = sigma * step;
                scattering += transmission * cloud_removed(depth) * source;
                transmission *= exp(-depth);
            }
            distance += step;
        }
    }
    textureStore(overlay_output, vec2<i32>(pixel), i32(id.z), vec4(scattering, transmission));
}
