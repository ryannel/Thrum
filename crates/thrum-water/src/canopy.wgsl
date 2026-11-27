// A sparse column table keeps the 25 cm canopy resolution over a wide world.
fn canopy_load(atlas: texture_2d<f32>, index: u32) -> vec4<f32> {
    let width = textureDimensions(atlas).x;
    return textureLoad(atlas, vec2<i32>(vec2(index % width, index / width)), 0);
}

fn canopy_layers(atlas: texture_2d<f32>, cell: vec2<u32>, size: vec2<u32>, start: u32, code: f32) -> vec4<f32> {
    if code == 0.0 { return canopy_load(atlas, start + cell.y * size.x + cell.x); }
    let mask = u32(abs(code));
    let stride = select(2u, 1u, code < 0.0);
    let key = cell + vec2(1u, 0u);
    var slot = ((cell.x * 0x8da6b343u) ^ (cell.y * 0xd8163841u)) & mask;
    for (var checked = 0u; checked <= mask; checked += 1u) {
        let index = start + stride * slot;
        let entry = canopy_load(atlas, index);
        if entry.x == 0.0 { break; }
        if all(vec2<u32>(entry.xy) == key) {
            if stride == 1u { return vec4(entry.zw, 0.0, 1.0); }
            return canopy_load(atlas, index + 1u);
        }
        slot = (slot + 1u) & mask;
    }
    return vec4(0.0, 1.0, 0.0, 1.0);
}

fn canopy_column(atlas: texture_2d<f32>, cell: vec2<i32>, height: f32, size: vec2<i32>, start: u32, code: f32) -> f32 {
    if any(cell < vec2(0)) || any(cell >= size) { return 1.0; }
    let layers = canopy_layers(atlas, vec2<u32>(cell), vec2<u32>(size), start, code);
    return select(1.0, layers.y, height < layers.x) * select(1.0, layers.w, height < layers.z);
}

fn canopy_exposure(atlas: texture_2d<f32>, point: vec3<f32>, lattice: vec4<f32>,
                   size: vec2<i32>, start: u32, code: f32, origin: vec2<i32>) -> f32 {
    // Apply the coarse origin in integer cells. Compute overlap on the
    // fractional part so the edge average stays precise far from that origin.
    let p = (point.xz - lattice.xy) / lattice.z;
    let cell = floor(p);
    let base = vec2<i32>(cell) - origin;
    let local = p - cell;
    let half_width = 0.5 * lattice.w / lattice.z;
    if half_width == 0.0 { return canopy_column(atlas, base, point.y, size, start, code); }
    let lo = local - half_width;
    let hi = local + half_width;
    let first = vec2<i32>(floor(lo));
    let last = vec2<i32>(floor(hi));
    var sum = 0.0;
    for (var z = first.y; z <= last.y; z += 1) {
        for (var x = first.x; x <= last.x; x += 1) {
            let offset = vec2(x, z);
            let area = max(vec2(0.0), min(vec2<f32>(offset) + 1.0, hi) - max(vec2<f32>(offset), lo));
            if area.x * area.y > 0.0 {
                sum += area.x * area.y * canopy_column(atlas, base + offset, point.y, size, start, code);
            }
        }
    }
    return min(1.0, sum / (4.0 * half_width * half_width));
}
