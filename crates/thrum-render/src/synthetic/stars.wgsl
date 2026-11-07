// Immutable equatorial unit vectors and perpendicular top-of-atmosphere lux.
struct StarView {
    clip_from_catalogue: mat4x4<f32>,
    extent_and_tan_half_fov: vec4<f32>,
    catalogue_zenith_and_strength: vec4<f32>,
}
struct ExposureSample {
    values: vec4<f32>,
    metadata: vec4<u32>,
}
@group(0) @binding(0) var<storage, read> catalogue: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> view: StarView;
@group(0) @binding(2) var depth: texture_depth_2d;
@group(0) @binding(3) var<uniform> exposure: ExposureSample;
@group(0) @binding(4) var transmittance: texture_2d<f32>;
@group(0) @binding(5) var lut_sampler: sampler;
struct Observer {
    inverse_view_proj: mat4x4<f32>,
    world_position_and_altitude: vec4<f32>,
}
@group(0) @binding(6) var<uniform> observer: Observer;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) centre: vec2<f32>,
    @location(1) @interpolate(flat) sigma: f32,
    @location(2) @interpolate(flat) scale: vec3<f32>,
}

fn rejected() -> VertexOut {
    return VertexOut(vec4(2.0, 2.0, 0.0, 1.0), vec2(0.0), 1.0, vec3(0.0));
}

@vertex
fn vertex(@builtin(vertex_index) index: u32, @builtin(instance_index) star: u32) -> VertexOut {
    let source = catalogue[star];
    let clip = view.clip_from_catalogue * vec4(source.xyz, 0.0);
    if clip.w <= 0.0 { return rejected(); }
    let centre_ndc = clip.xy / clip.w;
    if any(abs(centre_ndc) > vec2(MAX_CENTRE_NDC)) { return rejected(); }
    let mu = dot(source.xyz, view.catalogue_zenith_and_strength.xyz);
    let altitude_m = observer.world_position_and_altitude.w;
    if blocked(altitude_m, mu) { return rejected(); }
    let extent = view.extent_and_tan_half_fov.xy;
    let tan_half_fov = view.extent_and_tan_half_fov.zw;
    let centre = (centre_ndc * vec2(0.5, -0.5) + 0.5) * extent;
    // Gnomonic pixel solid angle at the source centre. This keeps integrated
    // flux constant as pixel density and perspective distortion change.
    let slopes = centre_ndc * tan_half_fov;
    let pixel_step = 2.0 * tan_half_fov / extent;
    let slope_squared = 1.0 + dot(slopes, slopes);
    let omega = pixel_step.x * pixel_step.y / (slope_squared * sqrt(slope_squared));
    let sigma = max(PIXEL_SIGMA, radians(ANGULAR_SIGMA_DEGREES) / sqrt(omega));
    let radius = RADIUS_SIGMAS * sigma + PIXEL_HALF_WIDTH;
    if any(centre + radius < vec2(0.0)) || any(centre - radius >= extent) { return rejected(); }
    if all(centre >= vec2(0.0)) && all(centre < extent) {
        if textureLoad(depth, vec2<i32>(centre), 0) > 0.0 { return rejected(); }
    }
    let corner = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0),
        vec2(-1.0, 1.0), vec2(1.0, -1.0), vec2(1.0, 1.0),
    );
    let point = centre + corner[index] * radius;
    let position = point / extent * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    let transmission = sample_sun_transmittance(altitude_m, mu, transmittance, lut_sampler);
    let scale = transmission * (source.w * view.catalogue_zenith_and_strength.w * exposure.values.x / omega);
    return VertexOut(vec4(position, 0.0, 1.0), centre, sigma, scale);
}

// Normal CDF via Abramowitz and Stegun 7.1.26.
fn normal_cdf(t: vec2<f32>) -> vec2<f32> {
    let z = abs(t) * 0.7071067811865476;
    let a = 1.0 / (1.0 + 0.3275911 * z);
    let p = (((1.061405429 * a - 1.453152027) * a + 1.421413741) * a - 0.284496736) * a + 0.254829592;
    let tail = 0.5 * p * a * exp(-z * z);
    return select(tail, 1.0 - tail, t >= vec2(0.0));
}

@fragment
fn fragment(input: VertexOut) -> @location(0) vec4<f32> {
    if textureLoad(depth, vec2<i32>(input.position.xy), 0) > 0.0 { discard; }
    let offset = input.position.xy - input.centre;
    let weight = max(normal_cdf((offset + PIXEL_HALF_WIDTH) / input.sigma) - normal_cdf((offset - PIXEL_HALF_WIDTH) / input.sigma), vec2(0.0));
    return vec4(input.scale * weight.x * weight.y, 0.0);
}
