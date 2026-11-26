// Ground geometry is in world coordinates. Only extent and noise use air coordinates.
struct GroundFrame {
    origin: vec2<f32>,
    spacing: f32,
    shore_margin: f32,
    travel: vec2<f32>,
    time: f32,
    seed: u32,
    upper_heights: vec2<f32>,
    has_active_rows: u32,
    padding: u32,
    ground_heights: vec4<f32>,
}
struct GroundRow {
    shape: vec4<f32>,  // depth, depression, softness, pooling
    bank: vec4<f32>,   // air centre and radii
    optics: vec4<f32>, // extinction, amount, noise share, inverse scale
    affinity: vec2<u32>,
    padding: vec2<f32>,
}

fn ground_cubic(t: f32) -> f32 {
    let x = clamp(t, 0.0, 1.0);
    return x * x * (3.0 - 2.0 * x);
}

fn ground_surface(world: vec2<f32>) -> vec3<f32> {
    let last = vec2<i32>(textureDimensions(ground_surface_texture)) - 1;
    let at = clamp((world - ground_frame.origin) / ground_frame.spacing, vec2(0.0), vec2<f32>(last));
    let lo = vec2<i32>(floor(at));
    let hi = min(lo + 1, last);
    let f = fract(at);
    let a = textureLoad(ground_surface_texture, lo, 0).xyz;
    let b = textureLoad(ground_surface_texture, vec2(hi.x, lo.y), 0).xyz;
    let c = textureLoad(ground_surface_texture, vec2(lo.x, hi.y), 0).xyz;
    let d = textureLoad(ground_surface_texture, hi, 0).xyz;
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

fn ground_random(cell: vec3<i32>) -> f32 {
    let i = bitcast<vec3<u32>>(cell);
    var x = i.x * 0x9e3779b1u + i.y * 0x85ebca77u + i.z * 0xc2b2ae3du
        + (70u + 131u * ground_frame.seed) * 0x27d4eb2fu;
    x = (x ^ (x >> 15u)) * 0x2c1b3c6du;
    x = (x ^ (x >> 12u)) * 0x297a2d39u;
    return f32(x ^ (x >> 15u)) / 4294967296.0;
}

fn ground_noise(at: vec3<f32>) -> f32 {
    let cell = vec3<i32>(floor(at));
    let f = smoothstep(vec3(0.0), vec3(1.0), fract(at));
    var value = 0.0;
    for (var x = 0; x <= 1; x++) {
        for (var y = 0; y <= 1; y++) {
            for (var z = 0; z <= 1; z++) {
                let offset = vec3(x, y, z);
                let w = select(1.0 - f, f, offset != vec3(0));
                value += w.x * w.y * w.z * ground_random(cell + offset);
            }
        }
    }
    return value;
}

// Coefficient and vertical optical depths of a row at the point.
fn ground_row(point: vec3<f32>, surface: vec3<f32>, row: GroundRow, coarse: bool) -> vec3<f32> {
    let amount = row.optics.y;
    let top = surface.x + amount * row.shape.x
        + row.shape.w * (surface.y - surface.x - row.shape.y);
    if amount == 0.0 || point.y < surface.x || point.y >= top { return vec3(0.0); }
    let affinity = row.affinity.x;
    let shore = surface.z / ground_frame.shore_margin;
    let mask = select(select(1.0, ground_cubic(1.0 + shore), affinity == 1u),
                      ground_cubic(-shore), affinity == 2u);
    let air = point.xz - ground_frame.travel;
    let uv = 2.0 * abs(air - row.bank.xy) / row.bank.zw - 1.0;
    let extent = select(1.0, (1.0 - ground_cubic(uv.x)) * (1.0 - ground_cubic(uv.y)),
                        row.affinity.y != 0u);
    let peak = row.optics.x * amount * mask * extent;
    if peak == 0.0 { return vec3(0.0); }
    var noise = 0.5;
    if !coarse && row.optics.z > 0.0 {
        noise = ground_noise(vec3(air.x, point.y, air.y) * row.optics.w
                             + vec3(0.0, 0.0, ground_frame.time / 1000.0));
    }
    let eroded = peak * mix(1.0, ground_cubic((noise - (1.0 - amount)) / 0.2), row.optics.z);
    let above = top - point.y;
    let upper = ground_profile_integral(above, row.shape.z);
    let whole = ground_profile_integral(top - surface.x, row.shape.z);
    return eroded * vec3(ground_cubic(above / row.shape.z), upper, whole - upper);
}

fn ground_profile_integral(height: f32, softness: f32) -> f32 {
    let t = min(height / softness, 1.0);
    return softness * (t * t * t) * (1.0 - 0.5 * t) + max(height - softness, 0.0);
}

fn ground_extinction(point: vec3<f32>, surface: vec3<f32>, coarse: bool) -> f32 {
    var extinction = 0.0;
    for (var row = 0u; row < arrayLength(&ground_rows); row++) {
        extinction += ground_row(point, surface, ground_rows[row], coarse).x;
    }
    return extinction;
}
