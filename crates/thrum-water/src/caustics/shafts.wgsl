@group(3) @binding(27) var shaft_image: texture_2d<f32>;
@group(3) @binding(28) var shaft_output: texture_storage_2d<rgba16float, write>;
@group(3) @binding(29) var shaft_field: texture_2d_array<f32>;
@group(3) @binding(30) var shaft_sampler: sampler;
@group(3) @binding(31) var<uniform> shaft_mapping: ShaftMapping;
@group(3) @binding(32) var shaft_columns: texture_2d<f32>;

struct ShaftMapping {
    window: vec4<f32>,
    sun: vec4<f32>,
    quality: vec4<u32>,
}

fn shaft_extent(capacity: vec2<u32>) -> vec2<u32> {
    let divisor = max(shaft_mapping.quality.w, 2u);
    return min((optics_extent() + divisor - 1u) / divisor, capacity);
}

fn shaft_range(uv: vec2<f32>, camera: vec3<f32>, ray: vec3<f32>, level: f32) -> f32 {
    let hit = gather_at(uv);
    var reach = min(optics.extinction_sight.w * 4.0, 4000.0);
    if hit.distance > 0.0 { reach = min(reach, length(hit.point - camera)); }
    if ray.y > 0.000001 { reach = min(reach, max(0.0, (level - camera.y) / ray.y)); }
    return reach;
}

@compute @workgroup_size(8, 8)
fn march_shafts(@builtin(global_invocation_id) slot: vec3<u32>) {
    let extent = shaft_extent(textureDimensions(shaft_output));
    if any(slot.xy >= extent) { return; }
    let uv = (vec2<f32>(slot.xy) + 0.5) / vec2<f32>(extent);
    let camera = view.camera_xyz_time.xyz;
    let window = shaft_mapping.window;
    let column = caustic_column(camera.xz, window, shaft_columns);
    let level = select(water.level_dt_reset.x, column.x, column.w > 0.0);
    let ray = normalize(lens_point(uv) - camera);
    let reach = shaft_range(uv, camera, ray, level);
    // Downward paths can enter the mean water plane in front of a split lens.
    // March only the submerged interval, including as the entry approaches 0.
    var entry = 0.0;
    if camera.y >= level {
        entry = reach;
        if ray.y < -0.000001 {
            entry = clamp((level - camera.y) / ray.y, 0.0, reach);
        }
    }
    let length_m = reach - entry;
    let sun = environment_sun_direction();
    let surface = vec3(camera.x, level, camera.z);
    let beam = shaft_beam(ray, surface);
    let count = shaft_mapping.quality.z;
    let step = length_m / f32(max(count, 1u));
    let sun_slope = water_mean_sun_slope(sun);
    let footprint = step * length(ray.xz + ray.y * sun_slope);
    let noise = fract(52.9829189 * fract(dot(vec2<f32>(slot.xy), vec2(0.06711056, 0.00583715))));
    var result = vec3(0.0);
    if shaft_mapping.window.w > 0.0 && length_m > 0.0 {
        for (var index = 0u; index < count; index++) {
            let t = (f32(index) + noise) * step;
            let point = camera + ray * (entry + t);
            let column = caustic_column(point.xz, window, shaft_columns);
            let level_here = select(level, column.x, column.w > 0.0);
            let depth = max(0.0, level_here - point.y);
            let exit = water_sun_exit(point, level_here, sun);
            let focused = caustic_field_at(exit.xz, depth, footprint, shaft_mapping.window, shaft_field, shaft_sampler, shaft_mapping.quality.y);
            let visibility = point_sun_visibility(exit);
            let transmission = exp(-beam.loss * depth - optics.extinction_sight.xyz * t);
            result += transmission * (focused * visibility - 1.0);
        }
    }
    // Store signed radiance in the scene's storage units, retaining depth for the join.
    let radiance = result * beam.source * (step * beam.phase);
    textureStore(shaft_output, vec2<i32>(slot.xy), vec4(radiance * environment_storage_scale(), reach));
}

fn shaft_join(uv: vec2<f32>, reach: f32) -> vec3<f32> {
    let extent = shaft_extent(textureDimensions(shaft_image));
    let position = uv * vec2<f32>(extent) - 0.5;
    let first = vec2<i32>(floor(position));
    let blend = fract(position);
    var result = vec3(0.0);
    var weight_sum = 0.0;
    for (var index = 0u; index < 4u; index++) {
        let offset = vec2<i32>(i32(index & 1u), i32(index >> 1u));
        let at = clamp(first + offset, vec2(0), vec2<i32>(extent) - 1);
        let value = textureLoad(shaft_image, at, 0);
        let weight_xy = mix(1.0 - blend, blend, vec2<f32>(offset));
        let weight = weight_xy.x * weight_xy.y / (1.0 + 16.0 * abs(value.a - reach));
        result += value.rgb * weight;
        weight_sum += weight;
    }
    return result / (max(weight_sum, 0.000001) * max(environment_storage_scale(), 0.000001));
}

fn shaft_scattering(
    uv: vec2<f32>, level: f32, ray: vec3<f32>,
    reach: f32, start_depth: f32, end_depth: f32,
) -> vec3<f32> {
    let camera = view.camera_xyz_time.xyz;
    let beam = shaft_beam(ray, vec3(camera.x, level, camera.z));
    return shaft_directional(beam, reach, start_depth, end_depth) + shaft_join(uv, reach);
}
