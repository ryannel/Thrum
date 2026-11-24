struct BankMapping {
    region: vec4<f32>,
    top_m: f32,
    border_m: f32,
    padding: vec2<f32>,
}
@group(1) @binding(0) var<uniform> bank_mapping: BankMapping;
@group(1) @binding(1) var bank_out: texture_storage_3d<rgba16float, write>;

const BANK_CENTRE = vec3(-14.0, 242.0, -7.0);
const BANK_RADIUS = vec3(38.0, 8.0, 34.0);
const EXTINCTION = 0.12;

fn bank_extinction(point: vec3<f32>) -> f32 {
    let delta = (point - BANK_CENTRE) / BANK_RADIUS;
    let core = max(0.0, 1.0 - dot(delta, delta));
    return EXTINCTION * core * core;
}

// Integrate the same quadratic density profile along the actual solar path.
fn bank_sun_depth(point: vec3<f32>, sun: vec3<f32>) -> f32 {
    let origin = (point - BANK_CENTRE) / BANK_RADIUS;
    let velocity = sun / BANK_RADIUS;
    let speed = length(velocity);
    let direction = velocity / speed;
    let centre = -dot(origin, direction);
    let perpendicular = origin + centre * direction;
    let remaining = 1.0 - dot(perpendicular, perpendicular);
    if remaining <= 0.0 { return 0.0; }
    let radius = sqrt(remaining);
    let near = clamp(-centre, -radius, radius);
    let far = radius;
    let depth = bank_antiderivative(far, remaining) - bank_antiderivative(near, remaining);
    return max(0.0, EXTINCTION * depth / speed);
}

fn bank_antiderivative(t: f32, remaining: f32) -> f32 {
    let t2 = t * t;
    return t * (remaining * remaining + t2 * (-2.0 / 3.0 * remaining + t2 / 5.0));
}

@compute @workgroup_size(8, 8)
fn coefficients(@builtin(global_invocation_id) cell: vec3<u32>) {
    if any(cell.xy >= textureDimensions(medium_out)) { return; }
    let point = medium_sample_point(cell);
    let weight = 1.0 - smoothstep(300.0, VOLUME_NEAR_DISTANCE, point.w);
    let extinction = bank_extinction(point.xyz) * weight;
    if extinction <= 0.0 {
        textureStore(medium_out, vec2<i32>(cell.xy), i32(cell.z), vec4(0.0));
        return;
    }
    let sun = medium_observer.sun_lux.xyz;
    let ray = normalize(point.xyz - medium_observer.camera_altitude.xyz);
    let cosine = dot(ray, sun);
    let forward = 0.45;
    let phase = (1.0 - forward * forward)
        / (12.566371 * pow(1.0 + forward * forward - 2.0 * forward * cosine, 1.5));
    let depth = bank_sun_depth(point.xyz, sun);
    let beam = phase * exp(-depth);
    let diffuse = 0.033 * exp(-0.28 * depth);
    textureStore(medium_out, vec2<i32>(cell.xy), i32(cell.z),
        vec4(extinction, extinction * (beam + diffuse), extinction * 0.014 * medium_observer.sun_lux.w * smoothstep(-0.15, 0.20, sun.y), 0.0));
}

@compute @workgroup_size(8, 8)
fn ground(@builtin(global_invocation_id) cell: vec3<u32>) {
    let size = textureDimensions(bank_out);
    if any(cell >= size) { return; }
    let uvw = vec3<f32>(cell) / vec3<f32>(size - 1u);
    let xz = mix(bank_mapping.region.xy, bank_mapping.region.zw, uvw.xy);
    // The test bank lies on the authored valley lookout at approximately 240 m.
    let surface = 240.0;
    let point = vec3(xz.x, surface + uvw.z * bank_mapping.top_m, xz.y);
    let depth = bank_sun_depth(point, medium_observer.sun_lux.xyz);
    textureStore(bank_out, vec3<i32>(cell), vec4(depth, surface, 1.0, 0.0));
}
