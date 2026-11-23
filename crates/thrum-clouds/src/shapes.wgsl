struct PlacedInstance {
    centre_yaw: vec4<f32>,
    form: vec4<f32>,
    geometry: vec4<f32>,
    keys: vec4<u32>,
    footprint: vec4<u32>,
}
struct PlacedInstances {
    bounds: vec4<f32>,
    optical: vec4<f32>,
    count: vec4<u32>,
    grid: vec4<f32>,
    slots: array<u32, 4096>,
    items: array<PlacedInstance, 1024>,
}

fn placed_smooth(value: f32) -> f32 {
    let x = clamp(value, 0.0, 1.0);
    return x * x * (3.0 - 2.0 * x);
}

fn placed_footprint(local: vec2<f32>, side: u32, offset: u32) -> vec3<f32> {
    let at = local * f32(side) - vec2(0.5);
    let lo = vec2<i32>(floor(at));
    let frac = fract(at);
    var result = vec3(0.0);
    for (var z = 0; z < 2; z++) {
        for (var x = 0; x < 2; x++) {
            let p = lo + vec2(x, z);
            if any(p < vec2(0i)) || any(p >= vec2(i32(side))) { continue; }
            let w = select(vec2(1.0) - frac, frac, vec2(x != 0, z != 0));
            let c = placed_footprints[offset + u32(p.y) * side + u32(p.x)].xyz;
            if c.x > 0.0 && w.x * w.y > 0.0 {
                result.y = select(min(result.y, c.y), c.y, result.x == 0.0);
                result.z = select(max(result.z, c.z), c.z, result.x == 0.0);
                result.x += c.x * w.x * w.y;
            }
        }
    }
    return result;
}

fn placed_mix_columns(a: vec3<f32>, b: vec3<f32>, weight: f32) -> vec3<f32> {
    if weight == 0.0 { return a; }
    let integral = mix(a.x, b.x, weight);
    if a.x == 0.0 { return vec3(integral, b.yz); }
    if b.x == 0.0 { return vec3(integral, a.yz); }
    return vec3(integral, min(a.y, b.y), max(a.z, b.z));
}

fn placed_column(shape: PlacedInstance, uv: vec2<f32>) -> vec3<f32> {
    let first = placed_footprint(uv, shape.footprint.x, shape.keys.z);
    if shape.form.w == 0.0 { return first; }
    let second = placed_footprint(uv, shape.footprint.x, shape.keys.w);
    return placed_mix_columns(first, second, shape.form.w);
}

fn placed_volume(shape: PlacedInstance, uvw: vec3<f32>, level: f32) -> f32 {
    if shape.footprint.y == 0u {
        return placed_slab(placed_library, placed_sampler, shape.keys.x, uvw, level);
    }
    let first = placed_slab(placed_growth, placed_sampler, shape.keys.x, uvw, level);
    if shape.form.w == 0.0 { return first; }
    let second = placed_slab(placed_growth, placed_sampler, shape.keys.y, uvw, level);
    return mix(first, second, shape.form.w);
}

fn placed_inside_bounds(point: vec2<f32>) -> bool {
    let bounds = placed_instances.bounds;
    return placed_instances.count.x > 0u
        && all(point >= bounds.xy) && all(point <= bounds.zw);
}

fn placed_candidates(point: vec2<f32>) -> vec2<u32> {
    let count = placed_instances.count;
    if count.y == 0u {
        return vec2(0u, count.x);
    }
    let grid = placed_instances.grid;
    let block = vec2<i32>(floor((point - grid.xy) / grid.z));
    if any(block < vec2(0i)) || any(block >= vec2<i32>(count.yz)) {
        return vec2(0u);
    }
    let slot = placed_instances.slots[u32(block.y) * count.y + u32(block.x)];
    return vec2(max(1u, slot) - 1u, slot);
}

fn placed_uv(point: vec2<f32>, shape: PlacedInstance) -> vec2<f32> {
    let delta = point - shape.centre_yaw.xy;
    let c = shape.centre_yaw.z;
    let s = shape.centre_yaw.w;
    let local = vec2(c * delta.x + s * delta.y, -s * delta.x + c * delta.y);
    return local / (shape.geometry.x * shape.form.x) + vec2(0.5);
}

struct PlacedSample {
    column: vec4<f32>,
    envelope: f32,
}

// The common source returns extinction, vertical optical depth, both height
// bounds and the life envelope. Erosion is relative to the body's extinction
// at its current envelope, so condensate can fade without crossing a cutoff.
fn placed_sample(point: vec3<f32>, footprint: f32, edge_footprint: f32) -> PlacedSample {
    let clear = PlacedSample(vec4(0.0), 0.0);
    if !placed_inside_bounds(point.xz) { return clear; }
    let optical = placed_instances.optical;
    let candidates = placed_candidates(point.xz);
    for (var index = candidates.x; index < candidates.y; index++) {
        let shape = placed_instances.items[index];
        let uv = placed_uv(point.xz, shape);
        let inset = 0.5 / f32(shape.footprint.x);
        if any(uv <= vec2(-inset)) || any(uv >= vec2(1.0 + inset)) { continue; }
        let column = placed_column(shape, uv);
        let scale = shape.form.x;
        let bottom = optical.y + column.y * scale;
        let top = optical.y + column.z * scale;
        if column.x == 0.0 || point.y <= bottom || point.y >= top { continue; }

        let height = (point.y - bottom) / (top - bottom);
        let vertical = placed_smooth(height / optical.z) * placed_smooth((1.0 - height) / optical.w);
        let integral = column.x * scale * shape.form.y * optical.x;
        let profile_integral = 1.0 - 0.5 * (optical.z + optical.w);
        let weights = placed_form_weights(shape.form.z);
        let column_density = integral * vertical / ((top - bottom) * profile_integral);
        var density = weights.x * column_density;
        if weights.y > 0.0 {
            let extent = shape.geometry.x;
            let local_y = (point.y - optical.y) / scale;
            let slab_y = (local_y - shape.geometry.y) / extent;
            let texel = extent * scale / f32(shape.footprint.x);
            let level = log2(max(1.0, footprint / texel));
            let address = vec3(uv.x, slab_y, uv.y);
            var voxel = placed_volume(shape, address, level);
            // The slab carries details smaller than the weather's billows.
            // Filter the boundary in metres and only remove density, so the
            // filter cannot fill a clear column or spread into a neighbour slab.
            if edge_footprint > max(texel, footprint) && voxel > 0.0 {
                voxel = min(voxel, placed_volume(shape, address, log2(edge_footprint / texel)));
            }
            density += weights.y * voxel * shape.form.y * optical.x;
        }
        return PlacedSample(vec4(density, integral, bottom, top), shape.form.y);
    }
    return clear;
}
