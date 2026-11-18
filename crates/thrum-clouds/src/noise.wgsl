@group(0) @binding(0) var output: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(1) var finer: texture_3d<f32>;
override seed: u32;

fn random(cell: vec3<i32>, stream: u32) -> f32 {
    let i = bitcast<vec3<u32>>(cell);
    var x = i.x * 0x9e3779b1u + i.y * 0x85ebca77u + i.z * 0xc2b2ae3du
        + (stream + 131u * seed) * 0x27d4eb2fu;
    x = (x ^ (x >> 15u)) * 0x2c1b3c6du;
    x = (x ^ (x >> 12u)) * 0x297a2d39u;
    return f32(x ^ (x >> 15u)) / 4294967296.0;
}

fn wrap(cell: vec3<i32>, period: i32) -> vec3<i32> {
    return (cell + vec3(period)) % vec3(period);
}

fn billow(uv: vec3<f32>, period: i32, stream: u32) -> f32 {
    let position = uv * f32(period);
    let site = vec3<i32>(floor(position));
    let local = fract(position);
    var nearest = 9.0;
    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let offset = vec3(x, y, z);
                let cell = wrap(site + offset, period);
                let centre = vec3(random(cell, stream), random(cell, stream + 1u), random(cell, stream + 2u));
                let delta = vec3<f32>(offset) + centre - local;
                nearest = min(nearest, dot(delta, delta));
            }
        }
    }
    return max(0.0, 1.0 - sqrt(nearest));
}

@compute @workgroup_size(4, 4, 4)
fn shape(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(output);
    if any(id >= size) { return; }
    let uv = (vec3<f32>(id) + 0.5) / vec3<f32>(size);
    let sum = 0.55 * billow(uv, 8, 50u) + 0.30 * billow(uv, 20, 53u)
        + 0.15 * billow(uv, 50, 56u);
    textureStore(output, id, vec4(clamp((sum - 0.22) / 0.56, 0.0, 1.0), 0.0, 0.0, 1.0));
}

@compute @workgroup_size(4, 4, 4)
fn detail(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(output);
    if any(id >= size) { return; }
    let position = (vec3<f32>(id) + 0.5) * 32.0 / vec3<f32>(size);
    let site = vec3<i32>(floor(position));
    let f = smoothstep(vec3(0.0), vec3(1.0), fract(position));
    var value = 0.0;
    for (var z = 0; z <= 1; z++) {
        for (var y = 0; y <= 1; y++) {
            for (var x = 0; x <= 1; x++) {
                let offset = vec3(x, y, z);
                let weight = select(1.0 - f, f, offset != vec3(0));
                value += weight.x * weight.y * weight.z * random(wrap(site + offset, 32), 41u);
            }
        }
    }
    textureStore(output, id, vec4(value, 0.0, 0.0, 1.0));
}

@compute @workgroup_size(4, 4, 4)
fn mip(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id >= textureDimensions(output)) { return; }
    var value = 0.0;
    for (var z = 0; z <= 1; z++) {
        for (var y = 0; y <= 1; y++) {
            for (var x = 0; x <= 1; x++) {
                value += textureLoad(finer, vec3<i32>(id) * 2 + vec3(x, y, z), 0).r;
            }
        }
    }
    textureStore(output, id, vec4(value * 0.125, 0.0, 0.0, 1.0));
}
