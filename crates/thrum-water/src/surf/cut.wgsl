fn surf_component(i: u32) -> Component { return components[i]; }

struct CutNode {
    horizontal: vec2<f32>,
    height: f32,
}
struct CutHit {
    rest: vec2<f32>,
    cell: vec2<f32>,
    height: f32,
    valid: bool,
}

fn cut_node(query: QueryPoint, offset: vec2<f32>, dt: f32) -> SurfPoint {
    let rest = query.position + offset;
    var body = query.body;
    var row = MapRow(query.level_current_bed.xyz, query.gains);
    var stage = query.surf_stage.x;
    if map.size_count_sea.z > 1u {
        let owner = map_sample(rest);
        if owner.body != NO_BODY && owner.body != body {
            body = owner.body;
            row = owner.row;
            stage = map.bodies[body].level.y;
        }
    }
    return surf_forward(rest, body, row, stage, settings.origin, dt,
        settings.lengths_chop.w, settings.heights, settings.components);
}

fn node(query: QueryPoint, offset: vec2<f32>) -> CutNode {
    let p = cut_node(query, offset, query.dt).displacement;
    return CutNode(offset + p.xz, p.y);
}

fn triangle(a: CutNode, b: CutNode, c: CutNode, cell: vec2<f32>, sign: f32, hit: CutHit) -> CutHit {
    let u = a.horizontal - c.horizontal;
    let v = b.horizontal - c.horizontal;
    let p = -c.horizontal;
    let det = u.x * v.y - u.y * v.x;
    if abs(det) < 1e-12 { return hit; }
    let s = (p.x * v.y - p.y * v.x) / det;
    let t = (u.x * p.y - u.y * p.x) / det;
    if min(s, t) < -1e-6 || s + t > 1.000001 { return hit; }
    let c_weight = 1.0 - s - t;
    let height = s * a.height + t * b.height + c_weight * c.height;
    if hit.valid && height <= hit.height { return hit; }
    let lattice = select(vec2(t, t + c_weight), vec2(t + c_weight, c_weight), sign > 0.0);
    return CutHit(cell + surf.controls.z * lattice, cell, height, true);
}

fn ceiling(depth: f32) -> vec2<f32> {
    var amplitude = 0.0;
    for (var k = 0u; k < settings.components; k++) {
        let c = components[k];
        amplitude += c.amplitude * max(1.0, surf_shoaling(1.0 / c.inverse_k, depth));
    }
    let throwing = 2.0 * amplitude / (surf.controls.y * depth) > surf.bounds.y;
    return vec2(amplitude * (settings.lengths_chop.w + select(0.0, surf.bounds.x, throwing)), f32(throwing));
}

fn reach_depth(point: vec2<f32>, reach: f32) -> f32 {
    let origin = map.origin_spacing_margin.xy;
    let spacing = map.origin_spacing_margin.z;
    var size = vec2<i32>(map.size_count_sea.xy);
    let a = vec2<i32>(floor((point - vec2(reach) - origin) / spacing));
    let b = vec2<i32>(ceil((point + vec2(reach) - origin) / spacing));
    if any(b < vec2(0)) || any(a >= size) { return surf.controls.x; }
    var lo = clamp(a, vec2(0), size - 1);
    var hi = clamp(b, vec2(0), size - 1);
    var y = 0;
    // Cover the complete rectangle with at most four blocks. Extra
    // extrema enlarge the reach safely where a block crosses its boundary.
    while any(hi - lo > vec2(1)) {
        y += size.y;
        size = (size + vec2(1)) / 2;
        lo /= 2;
        hi /= 2;
    }
    var highest = -1e30;
    for (var z = lo.y; z <= hi.y; z++) {
        for (var x = lo.x; x <= hi.x; x++) {
            highest = max(highest, textureLoad(bed_pyramid, vec2(x, y + z), 0).y);
        }
    }
    return max(surf.controls.x, surf.bounds.z - highest);
}

struct CutResult { hit: CutHit, nodes: u32, }

fn cut_rows(query: QueryPoint, reach: f32, lane: u32) -> CutResult {
    let cell = surf.controls.z;
    let base = floor((query.position - reach) / cell);
    let end = ceil((query.position + reach) / cell) + 1.0;
    let origin = base * cell - query.position;
    let extent = vec2<u32>(end - base);
    var hit: CutHit;
    var nodes = 0u;
    for (var z = lane; z < extent.y; z += QUERY_WIDTH) {
        let start = origin + vec2(0.0, f32(z) * cell);
        var a = node(query, start);
        var d = node(query, start + vec2(0.0, cell));
        for (var x = 0u; x < extent.x; x++) {
            let p = start + vec2(f32(x) * cell, 0.0);
            let b = node(query, p + vec2(cell, 0.0));
            let c = node(query, p + vec2(cell, cell));
            hit = triangle(a, b, c, p, 1.0, hit);
            hit = triangle(a, c, d, p, -1.0, hit);
            a = b;
            d = c;
        }
        nodes += 2u * (extent.x + 1u);
    }
    return CutResult(hit, nodes);
}

fn sheet_jacobian(query: QueryPoint, rest: vec2<f32>, h: f32) -> mat2x3<f32> {
    let x = vec2(h, 0.0);
    let z = x.yx;
    let dx = (cut_node(query, rest + x, query.dt).displacement -
        cut_node(query, rest - x, query.dt).displacement) / (2.0 * h);
    let dz = (cut_node(query, rest + z, query.dt).displacement -
        cut_node(query, rest - z, query.dt).displacement) / (2.0 * h);
    return mat2x3(dx + vec3(1.0, 0.0, 0.0), dz + vec3(0.0, 0.0, 1.0));
}

fn surf_query(point: QueryPoint, cut: CutResult) -> QueryWave {
    let hit = cut.hit;
    var rest = hit.rest;
    var height = hit.height;
    if hit.valid {
        for (var i = 0u; i < 16u; i++) {
            let v = node(point, rest);
            if length(v.horizontal) <= surf.controls.w {
                height = v.height;
                break;
            }
            let derivative = sheet_jacobian(point, rest, max(surf.controls.z * 1e-3, 0.001));
            let x = derivative[0].xz;
            let z = derivative[1].xz;
            let det = x.x * z.y - x.y * z.x;
            if abs(det) < 1e-6 { rest = hit.rest; break; }
            let next = rest - mat2x2(vec2(z.y, -x.y), vec2(-z.x, x.x)) * v.horizontal / det;
            if any(next < hit.cell) || any(next > hit.cell + surf.controls.z) {
                rest = hit.rest;
                break;
            }
            rest = next;
        }
    } else {
        for (var i = 0u; i < 256u; i++) {
            let next = -cut_node(point, rest, point.dt).displacement.xz;
            let error = length(next - rest);
            rest = next;
            if error < 0.00001 { break; }
        }
        height = node(point, rest).height;
    }
    let h = 0.001;
    let derivative = sheet_jacobian(point, rest, h);
    let x = derivative[0];
    let z = derivative[1];
    let det = x.x * z.z - x.z * z.x;
    let slope = vec2(x.y * z.z - z.y * x.z, z.y * x.x - x.y * z.x)
        / select(det, select(-1e-6, 1e-6, det >= 0.0), abs(det) < 1e-6);
    let velocity = (cut_node(point, rest, point.dt + h).displacement -
        cut_node(point, rest, point.dt - h).displacement) / (2.0 * h);
    return QueryWave(Wave(vec3(0.0, height, 0.0), slope, vec3(0.0), velocity), cut.nodes);
}


var<workgroup> cut_reach: f32;
var<workgroup> cut_enabled: bool;
var<workgroup> cut_parts: array<CutResult, QUERY_WIDTH>;

fn needs_cut(point: QueryPoint) -> vec2<f32> {
    var reach = ceiling(surf.controls.x).x;
    for (var i = 0u; i < 2u; i++) {
        reach = min(reach, ceiling(reach_depth(point.position, reach)).x);
    }
    return vec2(reach, ceiling(reach_depth(point.position, reach)).y);
}

@compute @workgroup_size(QUERY_WIDTH)
fn sample_query(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_index) lane: u32) {
    let point = points[group.x];
    if lane == 0u {
        cut_enabled = false;
        if point.body != 0xffffffffu && point.has_bed != 0u {
            let reach = needs_cut(point);
            cut_reach = reach.x;
            cut_enabled = reach.y > 0.0;
        }
    }
    workgroupBarrier();
    var part: CutResult;
    if cut_enabled { part = cut_rows(point, cut_reach, lane); }
    cut_parts[lane] = part;
    workgroupBarrier();
    if lane != 0u { return; }
    var gains = query_gains(point);
    var result: QueryWave;
    if point.body != 0xffffffffu {
        if point.has_bed == 0u {
            result = plain_query(point, gains.x);
        } else {
            var cut: CutResult;
            for (var i = 0u; i < QUERY_WIDTH; i++) {
                let next = cut_parts[i];
                cut.nodes += next.nodes;
                let earlier = next.hit.cell.y < cut.hit.cell.y ||
                    (next.hit.cell.y == cut.hit.cell.y && next.hit.cell.x < cut.hit.cell.x);
                // Match the sequential row-major tie break at equal height.
                if next.hit.valid && (!cut.hit.valid || next.hit.height > cut.hit.height ||
                    (next.hit.height == cut.hit.height && earlier)) {
                    cut.hit = next.hit;
                }
            }
            result = surf_query(point, cut);
            if surf.bodies[point.body].count > 0u && point.gains.x * settings.heights.x > 0.0 {
                let depth = point.level_current_bed.x - point.level_current_bed.w;
                let swash = min(settings.swash_margin.x * length(settings.heights * point.gains),
                    settings.swash_margin.y);
                if swash > 0.0 && depth < 4.0 * map.origin_spacing_margin.w {
                    let stages = surf_stages(surf.bodies[point.body], point.surf_stage.x);
                    let contour = shore_contour(map_cell(point.position), stages);
                    gains = vec4(gains.x, gains.yzw * contour_short_weight(
                        depth, swash, contour.z, depth + result.wave.displacement.y));
                }
            }
        }
    }
    write_answer(group.x, point, gains, result);
}
