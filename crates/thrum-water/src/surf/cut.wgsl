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
    face: f32,
}

fn cut_node(query: QueryPoint, rest: vec2<f32>, dt: f32) -> SurfPoint {
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
    let p = cut_node(query, query.position + offset, query.dt).displacement;
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
    let earlier = cell.y < hit.cell.y || (cell.y == hit.cell.y &&
        (cell.x < hit.cell.x || (cell.x == hit.cell.x && sign > hit.face)));
    if hit.valid && (height < hit.height || (height == hit.height && !earlier)) { return hit; }
    let lattice = select(vec2(t, t + c_weight), vec2(t + c_weight, c_weight), sign > 0.0);
    return CutHit(cell + surf.controls.z * lattice, cell, height, true, sign);
}

// The job table occupies the prefix. Every following record is one
// forward-evaluated coarse node, shared by stations on this sheet.
@group(2) @binding(0) var<storage, read_write> coarse_lattice: array<vec4<u32>>;

fn surf_displacement(point: QueryPoint, rest: vec2<f32>) -> vec3<f32> {
    return cut_node(point, rest, point.dt).displacement;
}

@compute @workgroup_size(QUERY_WIDTH)
fn sample_coarse(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_index) lane: u32) {
    let job = coarse_lattice[group.x];
    let index = job.y + lane;
    if index >= job.w { return; }
    let point = points[job.x];
    let grid = point.coarse_grid;
    let width = u32(grid.z);
    let coord = grid.xy + vec2(i32(index % width), i32(index / width));
    let rest = vec2<f32>(coord) * (surf.coarse.x * surf.controls.z);
    let displacement = surf_displacement(point, rest);
    coarse_lattice[settings.coarse_jobs.x + job.z + index] = bitcast<vec4<u32>>(vec4(displacement, 0.0));
}

fn coarse_corner(point: QueryPoint, coord: vec2<i32>) -> vec2<f32> {
    let grid = point.coarse_grid;
    if grid.z == 0 {
        let rest = vec2<f32>(coord) * (surf.coarse.x * surf.controls.z);
        return surf_displacement(point, rest).xz;
    }
    let relative = coord - grid.xy;
    let index = grid.w + relative.y * grid.z + relative.x;
    return bitcast<vec4<f32>>(coarse_lattice[settings.coarse_jobs.x + u32(index)]).xz;
}

fn keeps_cell(point: QueryPoint, coord: vec2<i32>) -> bool {
    let cell = surf.coarse.x * surf.controls.z;
    let a = coarse_corner(point, coord);
    let b = coarse_corner(point, coord + vec2(1, 0));
    let c = coarse_corner(point, coord + vec2(1, 1));
    let d = coarse_corner(point, coord + vec2(0, 1));
    let spread = sqrt(max(max(max(dot(a-b, a-b), dot(a-c, a-c)), max(dot(a-d, a-d), dot(b-c, b-c))),
        max(dot(b-d, b-d), dot(c-d, c-d))));
    let margin = surf.coarse.y + surf.coarse.z * spread;
    let pb = b + vec2(cell, 0.0);
    let pc = c + vec2(cell);
    let pd = d + vec2(0.0, cell);
    let lo = min(min(a, pb), min(pc, pd)) - margin;
    let hi = max(max(a, pb), max(pc, pd)) + margin;
    let column = point.position - vec2<f32>(coord) * cell;
    return all(column >= lo) && all(column <= hi);
}

fn coarse_bounds(fine: vec4<i32>) -> vec4<i32> {
    return vec4<i32>(vec2<i32>(floor(vec2<f32>(fine.xy) / surf.coarse.x)),
        vec2<i32>(ceil(vec2<f32>(fine.zw) / surf.coarse.x)));
}

fn fine_record(point: QueryPoint, index: u32) -> u32 {
    let width = u32(surf.coarse.x) + 1u;
    return settings.coarse_jobs.x + settings.coarse_jobs.y +
        (bitcast<u32>(point.surf_stage.w) - 1u) + index * width * width;
}

var<workgroup> fine_owner: atomic<u32>;
var<workgroup> fine_selected: u32;

@compute @workgroup_size(QUERY_WIDTH)
fn sample_fine(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_index) lane: u32) {
    let job = coarse_lattice[group.x / QUERY_WIDTH];
    let index = job.y + group.x % QUERY_WIDTH;
    let sheet = points[job.x];
    let width = u32(sheet.coarse_grid.z);
    let local = vec2(i32(index % width), i32(index / width));
    let coarse = sheet.coarse_grid.xy + local;
    if lane == 0u { atomicStore(&fine_owner, 0xffffffffu); }
    workgroupBarrier();
    if bitcast<u32>(sheet.surf_stage.w) != 0u &&
        index + width < job.w && u32(local.x) + 1u < width {
        for (var i = lane; i < settings.count; i += QUERY_WIDTH) {
            let point = points[i];
            let bounds = coarse_bounds(point.fine_bounds);
            if point.coarse_grid.z != 0 && point.coarse_grid.w == sheet.coarse_grid.w &&
                all(coarse >= bounds.xy) && all(coarse < bounds.zw) && keeps_cell(point, coarse) {
                atomicMin(&fine_owner, i);
            }
        }
    }
    workgroupBarrier();
    if lane == 0u { fine_selected = atomicLoad(&fine_owner); }
    let owner = workgroupUniformLoad(&fine_selected);
    if owner == 0xffffffffu { return; }
    let step = u32(surf.coarse.x);
    let side = step + 1u;
    let offset = fine_record(sheet, index);
    for (var i = lane; i < side * side; i += QUERY_WIDTH) {
        let coord = coarse * i32(step) + vec2(i32(i % side), i32(i / side));
        let rest = vec2<f32>(coord) * surf.controls.z;
        let p = surf_displacement(sheet, rest);
        coarse_lattice[offset + i] = vec4(bitcast<vec3<u32>>(p), owner);
    }
}

struct CutResult { hit: CutHit, fine_nodes: u32, coarse_nodes: u32, }

fn fine_node(point: QueryPoint, coord: vec2<i32>) -> CutNode {
    let rest = vec2<f32>(coord) * surf.controls.z;
    let p = surf_displacement(point, rest);
    return CutNode((rest - point.position) + p.xz, p.y);
}

// One group cooperatively evaluates each retained fine cell. The common
// eight-cell profile has 81 nodes and 64 fine cells. A lane samples up to
// two nodes, and then cuts one fine cell using its four shared corners.
const FINE_GRID_SIZE = 9u;
const KEPT_CAPACITY = 256u;
var<workgroup> kept: array<vec2<i32>, KEPT_CAPACITY>;
var<workgroup> kept_count: atomic<u32>;
var<workgroup> kept_final: u32;
var<workgroup> fine_lattice: array<CutNode, FINE_GRID_SIZE * FINE_GRID_SIZE>;

fn find_kept(
    point: QueryPoint, lane: u32, start: u32, end: u32, lo: vec2<i32>, width: u32,
) -> u32 {
    var evaluations = 0u;
    for (var i = start + lane; i < end; i += QUERY_WIDTH) {
        let coord = lo + vec2(i32(i % width), i32(i / width));
        if keeps_cell(point, coord) {
            let index = atomicAdd(&kept_count, 1u);
            if index < KEPT_CAPACITY { kept[index] = coord; }
        }
        if point.coarse_grid.z == 0 { evaluations += 4u; }
    }
    return evaluations;
}

fn shared_fine_tile(
    point: QueryPoint, coarse: vec2<i32>, lo: vec2<i32>, hi: vec2<i32>,
    station: u32, lane: u32, result: CutResult,
) -> CutResult {
    let size = vec2<u32>(max(hi - lo, vec2(0))) + 1u;
    let area = select(0u, size.x * size.y, all(hi > lo));
    var nodes = result.fine_nodes;
    let cached = bitcast<u32>(point.surf_stage.w) != 0u;
    let step = i32(surf.coarse.x);
    let side = u32(step) + 1u;
    var offset = 0u;
    if cached {
        let relative = coarse - point.coarse_grid.xy;
        offset = fine_record(point, u32(relative.y * point.coarse_grid.z + relative.x));
        if lane == 0u && coarse_lattice[offset].w == station {
            nodes += side * side;
        }
    }
    for (var i = lane; i < area; i += QUERY_WIDTH) {
        let coord = lo + vec2(i32(i % size.x), i32(i / size.x));
        if cached {
            let rel = vec2<u32>(coord - coarse * step);
            let p = bitcast<vec3<f32>>(coarse_lattice[offset + rel.y * side + rel.x].xyz);
            let rest = vec2<f32>(coord) * surf.controls.z;
            fine_lattice[i] = CutNode((rest - point.position) + p.xz, p.y);
        } else {
            fine_lattice[i] = fine_node(point, coord);
            nodes += 1u;
        }
    }
    workgroupBarrier();
    let width = size.x - 1u;
    var hit = result.hit;
    if area != 0u && lane < width * (size.y - 1u) {
        let x = lane % width;
        let z = lane / width;
        let i = z * size.x + x;
        let cell = vec2<f32>(lo + vec2(i32(x), i32(z))) * surf.controls.z - point.position;
        let a = fine_lattice[i];
        let b = fine_lattice[i + 1u];
        let c = fine_lattice[i + size.x + 1u];
        let d = fine_lattice[i + size.x];
        hit = triangle(a, b, c, cell, 1.0, hit);
        hit = triangle(a, c, d, cell, -1.0, hit);
    }
    workgroupBarrier();
    return CutResult(hit, nodes, result.coarse_nodes);
}

var<workgroup> coarse_area: u32;

fn cut_cells(point: QueryPoint, station: u32, lane: u32) -> CutResult {
    let step = i32(surf.coarse.x);
    let bounds = coarse_bounds(point.fine_bounds);
    let lo = bounds.xy;
    let size = vec2<u32>(bounds.zw - lo);
    if lane == 0u { coarse_area = size.x * size.y; }
    let area = workgroupUniformLoad(&coarse_area);
    var part: CutResult;
    var chunk = area;
    var start = 0u;
    while start < area {
        if lane == 0u { atomicStore(&kept_count, 0u); }
        workgroupBarrier();
        let end = min(start + chunk, area);
        part.coarse_nodes += find_kept(point, lane, start, end, lo, size.x);
        workgroupBarrier();
        if lane == 0u { kept_final = atomicLoad(&kept_count); }
        let count = workgroupUniformLoad(&kept_final);
        if count > KEPT_CAPACITY {
            // Retry very broad sheets in bounded chunks. Each coarse cell
            // in a chunk can contribute at most one retained coordinate.
            chunk = KEPT_CAPACITY;
            continue;
        }
        for (var i = 0u; i < count; i++) {
            let coarse = kept[i];
            let base = coarse * step;
            for (var z = 0; z < step; z += 8) {
                for (var x = 0; x < step; x += 8) {
                    let tile = base + vec2(x, z);
                    part = shared_fine_tile(point, coarse, max(tile, point.fine_bounds.xy),
                        min(min(tile + 8, base + step), point.fine_bounds.zw), station, lane, part);
                }
            }
        }
        start = end;
    }
    return part;
}

fn sheet_jacobian(query: QueryPoint, rest: vec2<f32>, h: f32) -> mat2x3<f32> {
    let x = vec2(h, 0.0);
    let z = x.yx;
    let dx = (cut_node(query, query.position + (rest + x), query.dt).displacement -
        cut_node(query, query.position + (rest - x), query.dt).displacement) / (2.0 * h);
    let dz = (cut_node(query, query.position + (rest + z), query.dt).displacement -
        cut_node(query, query.position + (rest - z), query.dt).displacement) / (2.0 * h);
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
            let next = -cut_node(point, point.position + rest, point.dt).displacement.xz;
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
    let velocity = (cut_node(point, point.position + rest, point.dt + h).displacement -
        cut_node(point, point.position + rest, point.dt - h).displacement) / (2.0 * h);
    return QueryWave(Wave(vec3(0.0, height, 0.0), slope, vec3(0.0), velocity), cut.fine_nodes, cut.coarse_nodes);
}


var<workgroup> cut_parts: array<CutResult, QUERY_WIDTH>;

@compute @workgroup_size(QUERY_WIDTH)
fn sample_query(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_index) lane: u32) {
    let point = points[group.x];
    let part = cut_cells(point, group.x, lane);
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
                cut.fine_nodes += next.fine_nodes;
                cut.coarse_nodes += next.coarse_nodes;
                let earlier = next.hit.cell.y < cut.hit.cell.y ||
                    (next.hit.cell.y == cut.hit.cell.y && (next.hit.cell.x < cut.hit.cell.x ||
                        (next.hit.cell.x == cut.hit.cell.x && next.hit.face > cut.hit.face)));
                // Match the sequential row-major tie break at equal height.
                if next.hit.valid && (!cut.hit.valid || next.hit.height > cut.hit.height ||
                    (next.hit.height == cut.hit.height && earlier)) {
                    cut.hit = next.hit;
                }
            }
            if point.coarse_grid.z != 0 && group.x == bitcast<u32>(point.surf_stage.y) {
                cut.coarse_nodes += bitcast<u32>(point.surf_stage.z);
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
