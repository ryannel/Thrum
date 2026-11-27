struct GroundWetness {
    pores: f32,
    surface: f32,
    cover: f32,
    class_index: u32,
    hollows_ground: bool,
}

fn wet_load(index: u32) -> vec4<f32> {
    let width = textureDimensions(wetness_field).x;
    return textureLoad(wetness_field, vec2<i32>(vec2(index % width, index / width)), 0);
}

fn wet_column(cell: vec2<i32>, y: f32, size: vec2<i32>, start: u32) -> f32 {
    if any(cell < vec2(0)) || any(cell >= size) { return 1.0; }
    let layers = wet_load(start + u32(cell.y * size.x + cell.x));
    return select(1.0, layers.y, y < layers.x) * select(1.0, layers.w, y < layers.z);
}

fn wet_exposure(point: vec3<f32>, header: vec4<f32>) -> f32 {
    let size = vec2<i32>(header.yz);
    if any(size == vec2(0)) || point.y >= header.x { return 1.0; }
    let canopy = wet_load(1u);
    let p = (point.xz - canopy.xy) / canopy.z;
    let bounds = wet_load(6u);
    if any(p < bounds.xy) || any(p >= bounds.zw) { return 1.0; }
    let half_width = 0.5 * canopy.w / canopy.z;
    let start = u32(header.w);
    if half_width == 0.0 { return wet_column(vec2<i32>(floor(p)), point.y, size, start); }
    let lo = p - half_width;
    let hi = p + half_width;
    let first = vec2<i32>(floor(lo));
    let last = vec2<i32>(floor(hi));
    var sum = 0.0;
    for (var z = first.y; z <= last.y; z += 1) {
        for (var x = first.x; x <= last.x; x += 1) {
            let cell = vec2(x, z);
            let area = max(vec2(0.0), min(vec2<f32>(cell) + 1.0, hi) - max(vec2<f32>(cell), lo));
            sum += area.x * area.y * wet_column(cell, point.y, size, start);
        }
    }
    return min(1.0, sum / (4.0 * half_width * half_width));
}

fn wet_class_start(class_index: u32) -> u32 { return 7u + 4u * class_index; }

fn wet_reach(first: vec4<f32>, quarter: vec2<f32>, exposure: f32) -> vec2<f32> {
    if exposure >= 0.5 { return mix(first.zw, first.xy, 2.0 * exposure - 1.0); }
    if exposure >= 0.25 { return mix(quarter, first.zw, 4.0 * exposure - 1.0); }
    return quarter * (4.0 * exposure);
}

fn relief_texel(p: vec2<i32>, size: vec2<i32>, address: u32) -> f32 {
    let texel = (p % size + size) % size;
    let index = u32(texel.y * size.x + texel.x);
    return wet_load(address + 2u + index / 4u)[index % 4u];
}

fn wet_pattern(point: vec2<f32>, address: u32) -> f32 {
    let tile = wet_load(address);
    let size = vec2<i32>(wet_load(address + 1u).xy);
    let uv = fract((point - tile.xy) / tile.zw) * vec2<f32>(size) - 0.5;
    let p = vec2<i32>(floor(uv));
    let t = fract(uv);
    let a = mix(relief_texel(p, size, address), relief_texel(p + vec2(1, 0), size, address), t.x);
    let b = mix(relief_texel(p + vec2(0, 1), size, address), relief_texel(p + vec2(1), size, address), t.x);
    return mix(a, b, t.y);
}

fn wet_band(above: f32, margin: f32) -> f32 {
    if above <= 0.0 { return 1.0; }
    return max(0.0, 1.0 - above / max(margin, 1e-20));
}

fn shore_corner(index: u32, start: u32) -> f32 {
    return wet_load(start + index / 4u)[index % 4u];
}

fn mapped_wet_shore(point: vec3<f32>, field: vec4<f32>) -> vec4<f32> {
    let start = u32(field.y);
    let header = wet_load(start);
    if point.y > header.w { return vec4(0.0); }
    let lattice = wet_load(4u);
    let last = vec2(lattice.w, field.x) - 1.0;
    let grid = (point.xz - lattice.xy) / lattice.z;
    if any(grid < vec2(0.0)) || any(grid > last) { return vec4(0.0); }
    let first = min(floor(grid), last - 1.0);
    let uv = grid - first;
    let weights = vec4((1.0-uv.x)*(1.0-uv.y), uv.x*(1.0-uv.y), (1.0-uv.x)*uv.y, uv.x*uv.y);
    let stride = u32(lattice.w);
    let lower = u32(first.y) * stride + u32(first.x);
    let corners = lower + vec4(0u, 1u, stride, stride + 1u);
    let heights = vec4(shore_corner(corners.x, u32(field.z)), shore_corner(corners.y, u32(field.z)),
        shore_corner(corners.z, u32(field.z)), shore_corner(corners.w, u32(field.z)));
    let bed = dot(heights, weights);
    // As in the film, the depth must belong to the bed.
    if abs(point.y - bed) > 0.06 { return vec4(0.0); }
    let masks_at = u32(header.z);
    let masks = vec4<u32>(vec4(shore_corner(corners.x, masks_at), shore_corner(corners.y, masks_at),
        shore_corner(corners.z, masks_at), shore_corner(corners.w, masks_at)));
    let candidates = masks.x | masks.y | masks.z | masks.w;
    // Resolve ownership with the same rank and corner reach as map_sample.
    var best = vec4(0.0);
    for (var id = 0u; id < u32(header.x); id++) {
        let bit = 1u << id;
        if (candidates & bit) == 0u { continue; }
        let row = wet_load(start + 1u + id);
        if bed > row.x + header.y { continue; }
        let both_margin = bed >= max(row.x, best.x);
        let wins = select(row.w > best.w || (row.w == best.w && row.x > best.x),
            row.x > best.x, both_margin);
        if best.w > 0.0 && !wins { continue; }
        for (var k = 0u; k < 4u; k++) {
            if weights[k] > 0.0 && (masks[k] & bit) != 0u &&
                bed_reaches(heights, bed, uv, k, row.x, row.w == 2.0 && bed < row.x) {
                best = row;
                break;
            }
        }
    }
    return best;
}

fn wet_shore(point: vec3<f32>) -> vec4<f32> {
    let shore = wet_load(5u);
    if shore.w == 0.0 { return shore; }
    if shore.w == 2.0 { return mapped_wet_shore(point, shore); }
    let bounds = wet_load(4u);
    return select(vec4(0.0), shore, all(point.xz >= bounds.xy) && all(point.xz <= bounds.zw));
}

// The film already has the owning map row. Apply the exposed sea treatment
// to that row alone, retaining the existing WATERS sheen on inland shores.
fn wet_sea_film(point: vec3<f32>, sea: bool) -> bool {
    if !sea { return false; }
    let shore = wet_load(5u);
    if shore.w == 2.0 { return true; }
    return wet_shore(point).w > 0.0;
}

fn wet_shore_edge(shore: vec4<f32>) -> f32 {
    return shore.x + shore.y;
}

fn ground_wetness_footprint(point: vec3<f32>, normal: vec3<f32>, footprint: f32) -> GroundWetness {
    let header = wet_load(0u);
    let lattice = wet_load(2u);
    var cell = vec4(0.0, -1.0, 0.0, 0.0);
    var ground = false;
    if lattice.z > 0.0 {
        let at = floor((point.xz - lattice.xy) / lattice.z);
        let field = wet_load(3u);
        let size = vec2(lattice.w, field.y);
        if all(at >= vec2(0.0)) && all(at < size) {
            let start = u32(field.x);
            let candidate = wet_load(start + u32(at.y) * u32(size.x) + u32(at.x));
            // At a slope the authored triangle can differ from its cell centre.
            // Keep props, including the roof, out of the ground's water account.
            if abs(point.y - candidate.x) < 0.028 + 0.5 * lattice.z && normal.y > 0.0 {
                ground = true;
                cell = candidate;
            }
        }
    }
    let class_index = u32(cell.z);
    let first = wet_class_start(class_index);
    let class_data = wet_load(first);
    let appearance = wet_load(first + 1u);
    let low = wet_load(first + 3u);
    let up = clamp(normal.y, -1.0, 1.0);
    let sky = select(0.25 * (1.0 + up), 0.25 + 0.75 * up, up >= 0.0);
    let store = wet_reach(wet_load(first + 2u), low.xy, wet_exposure(point, header) * sky);
    let slope = length(normal.xz) / max(up, 1e-20);
    let hold = class_data.x * max(0.0, 1.0 - slope / max(class_data.y, 1e-20));
    let surface = select(0.0, max(0.0, store.x - (class_data.x - hold)), up > 0.0);
    let share = clamp(surface / max(class_data.x, 1e-20), 0.0, 1.0);
    var cover = share;
    if appearance.z > 0.0 {
        let pattern = wet_pattern(point.xz, u32(appearance.z));
        // At a grazing view the footprint may cross the whole height ramp.
        // Fade the resolved boundary into the average cover over that footprint.
        let unresolved = smoothstep(0.013, 0.084, footprint);
        let half_edge = 0.11 * unresolved;
        let detail = smoothstep(-half_edge, 0.065 + half_edge, share - pattern);
        cover = select(select(0.0, 1.0, share > pattern),
            mix(detail, share, unresolved), footprint > 0.0);
        cover = select(0.0, cover, surface > 0.0);
    }
    if appearance.w > class_data.w {
        cover *= 1.0 - smoothstep(class_data.w, appearance.w, point.y);
    }
    var pores = store.y;
    if !ground && up > 0.05 {
        let shore = wet_shore(point);
        if shore.w > 0.0 {
            pores = max(pores, wet_band(point.y - wet_shore_edge(shore), shore.z));
        }
    }
    let open = GroundWetness(pores, surface, cover, class_index, ground);
    if cell.y < 0.0 { return open; }
    let hollow = wet_load(39u + u32(cell.y));
    if hollow.x > cell.w && point.y < hollow.x {
        return GroundWetness(1.0, 0.0, 0.0, class_index, ground);
    }
    let margin = select(0.0, wet_band(point.y - hollow.x, hollow.z), hollow.z > 0.0);
    if hollow.y > cell.w && point.y < hollow.y {
        return GroundWetness(max(hollow.w, margin), 0.0, 0.0, class_index, ground);
    }
    return GroundWetness(max(open.pores, margin), surface, cover, class_index, ground);
}

fn ground_wetness(point: vec3<f32>, normal: vec3<f32>) -> GroundWetness {
    return ground_wetness_footprint(point, normal, 0.0);
}

fn wet_material(point: vec3<f32>, normal: vec3<f32>, base: vec3<f32>, roughness: f32,
                metalness: f32) -> vec4<f32> {
    let wet = ground_wetness(point, normal);
    if wet.pores <= 0.0 && (wet.hollows_ground || wet.cover <= 0.0) {
        return vec4(base, roughness);
    }
    let index = wet_class_start(wet.class_index);
    let class_data = wet_load(index);
    let appearance = wet_load(index + 1u);
    let porosity = select(clamp((roughness - 0.35) / 0.65, 0.0, 1.0), appearance.x, appearance.x >= 0.0);
    let weight = wet.pores * porosity * (1.0 - metalness);
    let share = clamp(class_data.z, 0.0, 0.99);
    let dark = 0.98 * (1.0 - share) * base / max(vec3(0.01), vec3(1.0) - share * base);
    let film = select(wet.cover * (1.0 - metalness), 0.0, wet.hollows_ground);
    let gloss = max(0.5 * weight, film);
    return vec4(mix(base, dark, weight), mix(roughness, min(roughness, appearance.y), gloss));
}
