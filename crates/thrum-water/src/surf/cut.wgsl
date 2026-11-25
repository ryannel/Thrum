override single_sheet: bool = false;

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
    if !single_sheet && (query.flags & ONE_SHEET) == 0u && map.size_count_sea.z > 1u {
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
@group(2) @binding(1) var<storage, read_write> selected_count: atomic<u32>;

fn surf_displacement(point: QueryPoint, rest: vec2<f32>) -> vec3<f32> {
    return cut_node(point, rest, point.dt).displacement;
}

@compute @workgroup_size(QUERY_WIDTH)
fn sample_coarse(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_index) lane: u32) {
    let job = coarse_lattice[group.x];
    let index = job.y + lane;
    if index >= job.w { return; }
    let at = settings.coarse_jobs.x + job.z + index;
    let mask = coarse_lattice[at].w;
    if (mask & 2u) == 0u { return; }
    let point = points[job.x];
    let grid = point.coarse_grid;
    let width = u32(grid.z);
    let coord = grid.xy + vec2(i32(index % width), i32(index / width));
    let rest = vec2<f32>(coord) * (surf.coarse.x * surf.controls.z);
    let displacement = surf_displacement(point, rest);
    coarse_lattice[at] = vec4(bitcast<vec3<u32>>(displacement), mask);
}

// Eight adjacent lanes accumulate independent components of one node.
const NODES_PER_GROUP: u32 = QUERY_WIDTH / WAVE_LANES;
var<workgroup> wave_lanes: array<SurfMoments, QUERY_WIDTH>;
var<workgroup> swash_lanes: array<SurfMoments, QUERY_WIDTH>;

fn merge_moments(a: SurfMoments, b: SurfMoments) -> SurfMoments {
    return SurfMoments(a.field + b.field, a.rate + b.rate,
        a.re_vector + b.re_vector, a.im_vector + b.im_vector,
        a.horizontal + b.horizontal, a.variance + b.variance);
}

fn parallel_displacement(point: QueryPoint, rest: vec2<f32>, lane: u32, enabled: bool) -> vec3<f32> {
    let wave = lane % WAVE_LANES;
    let row = MapRow(point.level_current_bed.xyz, point.gains);
    let chop = settings.lengths_chop.w;
    var column: SurfColumn;
    var ordinary: SurfMoments;
    var swash: SurfMoments;
    if enabled {
        column = surf_column(rest, point.body, row, point.surf_stage.x, settings.heights, settings.components);
        if column.valid {
            ordinary = surf_column_moments(column, rest, row, settings.origin, point.dt,
                chop, settings.components, true, wave, WAVE_LANES);
            if column.inner > 0.0 {
                swash = surf_column_moments(column, rest, row, settings.origin, point.dt,
                    chop, settings.components, false, wave, WAVE_LANES);
            }
        }
    }
    wave_lanes[lane] = ordinary;
    swash_lanes[lane] = swash;
    workgroupBarrier();
    // Tree reduction within each node. Every lane reaches each barrier.
    for (var step = WAVE_LANES / 2u; step != 0u; step /= 2u) {
        if wave < step {
            wave_lanes[lane] = merge_moments(wave_lanes[lane], wave_lanes[lane + step]);
            swash_lanes[lane] = merge_moments(swash_lanes[lane], swash_lanes[lane + step]);
        }
        workgroupBarrier();
    }
    var result = vec3(0.0);
    if wave == 0u && column.valid {
        result = surf_column_result(column, rest, row, chop, wave_lanes[lane], swash_lanes[lane]).displacement;
    }
    workgroupBarrier();
    return result;
}

@compute @workgroup_size(QUERY_WIDTH)
fn sample_coarse_wide(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_index) lane: u32) {
    let job = coarse_lattice[group.x];
    let index = job.y + group.y * NODES_PER_GROUP + lane / WAVE_LANES;
    let at = settings.coarse_jobs.x + job.z + index;
    var mask = 0u;
    if index < job.w { mask = coarse_lattice[at].w; }
    let point = points[job.x];
    let grid = point.coarse_grid;
    let width = u32(grid.z);
    let coord = grid.xy + vec2(i32(index % width), i32(index / width));
    let rest = vec2<f32>(coord) * (surf.coarse.x * surf.controls.z);
    let valid = index < job.w && (mask & 2u) != 0u;
    let displacement = parallel_displacement(point, rest, lane, valid);
    if valid && lane % WAVE_LANES == 0u {
        coarse_lattice[at] = vec4(bitcast<vec3<u32>>(displacement), mask);
    }
}

fn coarse_corner(point: QueryPoint, coord: vec2<i32>, cached: bool) -> vec2<f32> {
    let grid = point.coarse_grid;
    if !cached {
        let rest = vec2<f32>(coord) * (surf.coarse.x * surf.controls.z);
        return surf_displacement(point, rest).xz;
    }
    let relative = coord - grid.xy;
    let index = grid.w + relative.y * grid.z + relative.x;
    return bitcast<vec4<f32>>(coarse_lattice[settings.coarse_jobs.x + u32(index)]).xz;
}

fn landing_box(point: QueryPoint, coord: vec2<i32>, cached: bool) -> vec4<f32> {
    let cell = surf.coarse.x * surf.controls.z;
    let offsets = array(vec2(0, 0), vec2(1, 0), vec2(1, 1), vec2(0, 1));
    var corners: array<vec2<f32>, 4>;
    for (var i = 0u; i < 4u; i++) {
        corners[i] = coarse_corner(point, coord + offsets[i], cached);
    }
    let a = corners[0];
    let b = corners[1];
    let c = corners[2];
    let d = corners[3];
    let spread = sqrt(max(max(max(dot(a-b, a-b), dot(a-c, a-c)), max(dot(a-d, a-d), dot(b-c, b-c))),
        max(dot(b-d, b-d), dot(c-d, c-d))));
    let margin = surf.coarse.y + surf.coarse.z * spread;
    let pb = b + vec2(cell, 0.0);
    let pc = c + vec2(cell);
    let pd = d + vec2(0.0, cell);
    let lo = min(min(a, pb), min(pc, pd)) - margin;
    let hi = max(max(a, pb), max(pc, pd)) + margin;
    return vec4(lo, hi);
}

fn contains_column(box: vec4<f32>, point: vec2<f32>, coord: vec2<i32>) -> bool {
    let column = point - vec2<f32>(coord) * (surf.coarse.x * surf.controls.z);
    return all(column >= box.xy) && all(column <= box.zw);
}

fn coarse_bounds(fine: vec4<i32>) -> vec4<i32> {
    return vec4<i32>(vec2<i32>(floor(vec2<f32>(fine.xy) / surf.coarse.x)),
        vec2<i32>(ceil(vec2<f32>(fine.zw) / surf.coarse.x)));
}

fn coarse_index(point: QueryPoint, coord: vec2<i32>) -> u32 {
    let relative = coord - point.coarse_grid.xy;
    return u32(relative.y * point.coarse_grid.z + relative.x);
}

fn owners_offset() -> u32 {
    return settings.coarse_jobs.x + settings.coarse_jobs.y;
}

fn jobs_offset() -> u32 {
    return owners_offset() + settings.coarse_jobs.y;
}

fn fine_record(point: QueryPoint, index: u32) -> u32 {
    let width = u32(surf.coarse.x) + 1u;
    return jobs_offset() + settings.coarse_jobs.y +
        (bitcast<u32>(point.surf_stage.w) - 1u) + index * width * width;
}

@compute @workgroup_size(QUERY_WIDTH)
fn select_fine(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_index) lane: u32) {
    let job = coarse_lattice[group.x];
    let index = job.y + lane;
    let sheet = points[job.x];
    let width = u32(sheet.coarse_grid.z);
    let local = vec2(i32(index % width), i32(index / width));
    if bitcast<u32>(sheet.surf_stage.w) == 0u ||
        index + width >= job.w || u32(local.x) + 1u >= width { return; }
    let coarse = sheet.coarse_grid.xy + local;
    var owner = 0xffffffffu;
    if (coarse_lattice[settings.coarse_jobs.x + job.z + index].w & 1u) == 0u {
        coarse_lattice[owners_offset() + job.z + index] = vec4(owner, 0u, 0u, 0u);
        return;
    }
    let box = landing_box(sheet, coarse, true);
    for (var i = 0u; i < settings.count; i++) {
        let point = points[i];
        let bounds = coarse_bounds(point.fine_bounds);
        if bitcast<u32>(point.surf_stage.w) == bitcast<u32>(sheet.surf_stage.w) &&
            all(coarse >= bounds.xy) && all(coarse < bounds.zw) &&
            contains_column(box, point.position, coarse) {
            owner = i;
            break;
        }
    }
    coarse_lattice[owners_offset() + job.z + index] = vec4(owner, 0u, 0u, 0u);
    if owner != 0xffffffffu {
        let selected = atomicAdd(&selected_count, 1u);
        coarse_lattice[jobs_offset() + selected] = vec4(job.x, index, 0u, 0u);
    }
}

@compute @workgroup_size(QUERY_WIDTH)
fn sample_fine(@builtin(workgroup_id) group: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>,
    @builtin(local_invocation_index) lane: u32) {
    let count = atomicLoad(&selected_count);
    let step = u32(surf.coarse.x);
    let side = step + 1u;
    for (var selected = group.x; selected < count; selected += groups.x) {
        let job = coarse_lattice[jobs_offset() + selected];
        let sheet = points[job.x];
        let width = u32(sheet.coarse_grid.z);
        let index = job.y;
        let coarse = sheet.coarse_grid.xy + vec2(i32(index % width), i32(index / width));
        let offset = fine_record(sheet, index);
        for (var i = lane; i < side * side; i += QUERY_WIDTH) {
            let coord = coarse * i32(step) + vec2(i32(i % side), i32(i / side));
            let rest = vec2<f32>(coord) * surf.controls.z;
            let p = surf_displacement(sheet, rest);
            coarse_lattice[offset + i] = vec4(bitcast<vec3<u32>>(p), 0u);
        }
    }
}

var<workgroup> fine_count: u32;

@compute @workgroup_size(QUERY_WIDTH)
fn sample_fine_wide(@builtin(workgroup_id) group: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>,
    @builtin(local_invocation_index) lane: u32) {
    if lane == 0u { fine_count = atomicLoad(&selected_count); }
    let count = workgroupUniformLoad(&fine_count);
    let step = u32(surf.coarse.x);
    let side = step + 1u;
    let i = group.y * NODES_PER_GROUP + lane / WAVE_LANES;
    for (var selected = group.x; selected < count; selected += groups.x) {
        let job = coarse_lattice[jobs_offset() + selected];
        let sheet = points[job.x];
        let width = u32(sheet.coarse_grid.z);
        let index = job.y;
        let coarse = sheet.coarse_grid.xy + vec2(i32(index % width), i32(index / width));
        let coord = coarse * i32(step) + vec2(i32(i % side), i32(i / side));
        let rest = vec2<f32>(coord) * surf.controls.z;
        let displacement = parallel_displacement(sheet, rest, lane, i < side * side);
        if i < side * side && lane % WAVE_LANES == 0u {
            coarse_lattice[fine_record(sheet, index) + i] = vec4(bitcast<vec3<u32>>(displacement), 0u);
        }
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
        if point.coarse_grid.z != 0 {
            let index = u32(point.coarse_grid.w) + coarse_index(point, coord);
            if (coarse_lattice[settings.coarse_jobs.x + index].w & 1u) == 0u { continue; }
        }
        if bitcast<u32>(point.surf_stage.w) != 0u {
            let owner = coarse_lattice[owners_offset() + u32(point.coarse_grid.w) + coarse_index(point, coord)].x;
            if owner == 0xffffffffu { continue; }
        }
        if contains_column(landing_box(point, coord, point.coarse_grid.z != 0), point.position, coord) {
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
        let index = coarse_index(point, coarse);
        offset = fine_record(point, index);
        let owner = coarse_lattice[owners_offset() + u32(point.coarse_grid.w) + index].x;
        if lane == 0u && owner == station {
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

fn surf_query(point: QueryPoint, cut: CutResult) -> QueryWave {
    let hit = cut.hit;
    var rest = hit.rest;
    var height = hit.height;
    var finished = false;
    var slope = vec2(0.0);
    var velocity = vec3(0.0);
    // Keep one forward-sheet evaluation site for the centre, spatial
    // stencil and time samples. Expanding a separate evaluator at every
    // stencil offset makes the Metal compiler repeat the entire shore model.
    for (var iteration = 0u; iteration <= 256u; iteration++) {
        let h = select(max(surf.controls.z * 1e-3, 0.001), 0.001, finished);
        let offsets = array(vec3(0.0), vec3(h, 0.0, 0.0), vec3(-h, 0.0, 0.0),
            vec3(0.0, h, 0.0), vec3(0.0, -h, 0.0), vec3(0.0, 0.0, h), vec3(0.0, 0.0, -h));
        var samples: array<vec3<f32>, 7>;
        var count = select(1u, 7u, finished);
        for (var j = 0u; j < count; j++) {
            let offset = offsets[j];
            samples[j] = cut_node(point, point.position + (rest + offset.xy), point.dt + offset.z).displacement;
            if j == 0u && hit.valid && !finished &&
                length(rest + samples[0].xz) > surf.controls.w {
                count = 5u;
            }
        }
        if finished {
            if !hit.valid { height = samples[0].y; }
            velocity = (samples[5] - samples[6]) / (2.0 * h);
        } else if !hit.valid {
            let next = -samples[0].xz;
            let error = length(next - rest);
            rest = next;
            finished = error <= 0.25 * surf.controls.w || iteration == 255u;
            continue;
        } else if count == 1u {
            height = samples[0].y;
            finished = true;
            continue;
        }
        let x = (samples[1] - samples[2]) / (2.0 * h) + vec3(1.0, 0.0, 0.0);
        let z = (samples[3] - samples[4]) / (2.0 * h) + vec3(0.0, 0.0, 1.0);
        let det = x.x * z.z - x.z * z.x;
        if finished {
            slope = vec2(x.y * z.z - z.y * x.z, z.y * x.x - x.y * z.x)
                / select(det, select(-1e-6, 1e-6, det >= 0.0), abs(det) < 1e-6);
            break;
        }
        if abs(det) < 1e-6 {
            rest = hit.rest;
            finished = true;
            continue;
        }
        let next = rest - mat2x2(vec2(z.z, -x.z), vec2(-z.x, x.x)) * (rest + samples[0].xz) / det;
        if any(next < hit.cell) || any(next > hit.cell + surf.controls.z) {
            rest = hit.rest;
            finished = true;
        } else {
            rest = next;
            finished = iteration == 15u;
        }
    }
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
        if (point.flags & 1u) == 0u {
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
