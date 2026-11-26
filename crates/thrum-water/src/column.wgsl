struct Wave {
    displacement: vec3<f32>,
    slope: vec2<f32>,
    differential: vec3<f32>,
    velocity: vec3<f32>,
}
fn world_slope(wave: Wave) -> vec2<f32> {
    let j = wave.differential + vec3(1.0, 0.0, 1.0);
    let det = max(0.01, j.x * j.z - j.y * j.y);
    return vec2(j.z * wave.slope.x - j.y * wave.slope.y, j.x * wave.slope.y - j.y * wave.slope.x) / det;
}

fn column_phase(c: Component, point: vec2<f32>, offset: vec2<f32>, dt: f32) -> f32 {
    return dot(c.vector, point) + c.phase - c.omega * dt + dot(c.vector, offset);
}

fn long_wave(p: vec2<f32>, dt: f32, gain: f32, chop: f32, count: u32) -> Wave {
    var wave: Wave;
    if gain == 0.0 { return wave; }
    var offset = vec2(0.0);
    // The profile admits a contraction below 0.9. Solve in local offsets so
    // adding a submillimetre correction does not round at world coordinates.
    for (var iteration = 0u; iteration < 192u; iteration++) {
        var next = vec2(0.0);
        for (var i = 0u; i < count; i++) {
            let c = column_component(i);
            let phase = column_phase(c, p, offset, dt);
            next += (gain * chop * c.amplitude * c.inverse_k * sin(phase)) * c.vector;
        }
        let residual = length(next - offset);
        offset = next;
        if residual < 0.00001 { break; }
    }
    for (var i = 0u; i < count; i++) {
        let c = column_component(i);
        let k = c.vector;
        let phase = column_phase(c, p, offset, dt);
        let h = gain * c.amplitude * cos(phase);
        let s = gain * c.amplitude * sin(phase);
        let direction = chop * c.inverse_k * k;
        wave.displacement.y += h;
        wave.slope -= s * k;
        wave.differential -= h * chop * c.inverse_k * vec3(k.x * k.x, k.x * k.y, k.y * k.y);
        wave.velocity += c.omega * vec3(direction.x * h, s, direction.y * h);
    }
    return wave;
}

fn periodic_texel(uv: vec2<f32>, layer: i32, mip: i32) -> vec4<f32> {
    // Query tolerances apply to the exact box mips. Hardware uses an
    // implementation-defined precision for the interpolation weights.
    let extent = vec2<i32>(textureDimensions(cascades, mip));
    let coord = fract(uv) * vec2<f32>(extent) - 0.5;
    let corner = vec2<i32>(floor(coord));
    let weight = fract(coord);
    let mask = extent - 1;
    let a = textureLoad(cascades, corner & mask, layer, mip);
    let b = textureLoad(cascades, (corner + vec2(1, 0)) & mask, layer, mip);
    let c = textureLoad(cascades, (corner + vec2(0, 1)) & mask, layer, mip);
    let d = textureLoad(cascades, (corner + 1) & mask, layer, mip);
    return mix(mix(a, b, weight.x), mix(c, d, weight.x), weight.y);
}

fn periodic_field(uv: vec2<f32>, layer: i32, lod: f32) -> vec4<f32> {
    let low = i32(lod);
    let a = periodic_texel(uv, layer, low);
    if lod == f32(low) { return a; }
    return mix(a, periodic_texel(uv, layer, low + 1), fract(lod));
}

fn cascade_column(uv: vec2<f32>, band: u32, lod: f32) -> Wave {
    let layer = i32(band * 5u);
    let displacement = periodic_field(uv, layer, lod).xyz
        + periodic_field(uv, layer + 4, lod).yzw;
    let slope = periodic_field(uv, layer + 1, lod);
    let velocity = periodic_field(uv, layer + 2, lod);
    return Wave(displacement, slope.xy, vec3(slope.zw, velocity.w), velocity.xyz);
}

struct CarriedColumn {
    wave: Wave,
    advection_rate: f32,
}

// The rendered surface, query and wet support read the same departure point.
fn carried_column(point: vec2<f32>, motion: CascadeMotion, extent: f32, band: u32, lod: f32) -> CarriedColumn {
    let carried = cascade_column(motion.departure / extent, band, lod);
    let advection_rate = -dot(carried.slope, motion.drift) * motion.blend;
    if motion.blend >= 1.0 { return CarriedColumn(carried, advection_rate); }
    let ambient = cascade_column(point / extent, band, lod);
    return CarriedColumn(Wave(mix(ambient.displacement, carried.displacement, motion.blend),
        mix(ambient.slope, carried.slope, motion.blend),
        mix(ambient.differential, carried.differential, motion.blend),
        mix(ambient.velocity, carried.velocity, motion.blend)), advection_rate);
}
