// Camera frustum compaction into one indexed indirect draw per bucket.
// Three ordered dispatches: per-workgroup counts and slot masks, one-workgroup
// scan, and a scatter that reads only the masks (instances are tested once).
// The visible list preserves instance order, so draw order and depth ties
// match the direct draw. The test uses the same jittered view_proj and the
// same interpolated translation as the forward vertex shader.
struct ViewRecord {
    view_proj: mat4x4<f32>,
    previous_view_proj: mat4x4<f32>,
    camera_xyz_time: vec4<f32>,
    pre_exposure_inv_exposure_jitter: vec4<f32>,
}
struct InstanceRecord {
    current: mat4x4<f32>,
    previous: mat4x4<f32>,
    normal0: vec4<f32>,
    normal1: vec4<f32>,
    normal2: vec4<f32>,
    ids: vec4<u32>,
}
struct Serials {
    previous_lo: u32,
    previous_hi: u32,
    current_lo: u32,
    current_hi: u32,
    presentation_alpha: f32,
    padding0: u32,
    padding1: u32,
    padding2: u32,
}
struct MeshRecord {
    first_index: u32,
    index_count: u32,
    base_vertex: i32,
    padding: u32,
    bounds_min: vec4<f32>,
    bounds_max: vec4<f32>,
}
struct Params {
    active_count: u32,
    groups: u32,
    mesh: u32,
    padding: u32,
}

const WORKGROUP: u32 = 256u;
const WORDS: u32 = WORKGROUP / 32u;

@group(0) @binding(0) var<uniform> view: ViewRecord;
@group(0) @binding(1) var<storage, read> instances: array<InstanceRecord>;
@group(0) @binding(2) var<uniform> serials: Serials;
@group(0) @binding(3) var<storage, read> meshes: array<MeshRecord>;
@group(0) @binding(4) var<uniform> params: Params;
@group(0) @binding(5) var<storage, read_write> group_offsets: array<u32>;
@group(0) @binding(6) var<storage, read_write> visible: array<u32>;
@group(0) @binding(7) var<storage, read_write> args: array<u32>;
@group(0) @binding(8) var<storage, read_write> masks: array<u32>;

var<workgroup> scan: array<u32, WORKGROUP>;

fn plane_row(row: u32) -> vec4<f32> {
    let m = view.view_proj;
    return vec4(m[0][row], m[1][row], m[2][row], m[3][row]);
}

// Conservative world AABB against the six reverse-Z clip planes. A box is
// rejected only when it lies wholly behind one plane.
fn outside(plane: vec4<f32>, center: vec3<f32>, extent: vec3<f32>) -> bool {
    let distance = dot(plane.xyz, center) + plane.w;
    let radius = dot(abs(plane.xyz), extent);
    return distance + radius * 1.0001 + 1.0e-5 * abs(plane.w) < 0.0;
}

fn in_view(slot: u32) -> bool {
    if slot >= params.active_count {
        return false;
    }
    // Field loads, not a whole-record copy: this pass is bandwidth bound.
    if instances[slot].ids.w == 0u {
        return false;
    }
    let mesh = meshes[params.mesh];
    let local_center = (mesh.bounds_min.xyz + mesh.bounds_max.xyz) * 0.5;
    let local_extent = (mesh.bounds_max.xyz - mesh.bounds_min.xyz) * 0.5;
    let basis = mat3x3(instances[slot].current[0].xyz, instances[slot].current[1].xyz,
        instances[slot].current[2].xyz);
    let translation = mix(instances[slot].previous[3].xyz, instances[slot].current[3].xyz,
        serials.presentation_alpha);
    let center = basis * local_center + translation;
    let extent = abs(basis[0]) * local_extent.x + abs(basis[1]) * local_extent.y
        + abs(basis[2]) * local_extent.z;
    let r0 = plane_row(0u);
    let r1 = plane_row(1u);
    let r2 = plane_row(2u);
    let r3 = plane_row(3u);
    return !(outside(r3 + r0, center, extent) || outside(r3 - r0, center, extent)
        || outside(r3 + r1, center, extent) || outside(r3 - r1, center, extent)
        || outside(r3 - r2, center, extent) || outside(r2, center, extent));
}

// Inclusive Hillis-Steele scan of `scan`; every invocation must call it.
fn scan_workgroup(local: u32) {
    for (var stride = 1u; stride < WORKGROUP; stride = stride * 2u) {
        workgroupBarrier();
        var add = 0u;
        if local >= stride {
            add = scan[local - stride];
        }
        workgroupBarrier();
        scan[local] = scan[local] + add;
    }
    workgroupBarrier();
}

@compute @workgroup_size(256)
fn count(@builtin(global_invocation_id) global: vec3<u32>,
         @builtin(local_invocation_id) local: vec3<u32>,
         @builtin(workgroup_id) group: vec3<u32>) {
    scan[local.x] = select(0u, 1u, in_view(global.x));
    scan_workgroup(local.x);
    if local.x == WORKGROUP - 1u {
        group_offsets[group.x] = scan[local.x];
    }
    // Recover each slot's flag from the inclusive scan and pack 32 per word.
    if local.x < WORDS {
        let base = local.x * 32u;
        var before = 0u;
        if base > 0u {
            before = scan[base - 1u];
        }
        var word = 0u;
        for (var bit = 0u; bit < 32u; bit = bit + 1u) {
            let at = scan[base + bit];
            word = word | ((at - before) << bit);
            before = at;
        }
        masks[group.x * WORDS + local.x] = word;
    }
}

// One workgroup turns per-group counts into exclusive offsets and writes the
// bucket's indirect arguments, including the GPU-written instance count.
@compute @workgroup_size(256)
fn offsets(@builtin(local_invocation_id) local: vec3<u32>) {
    let per = (params.groups + WORKGROUP - 1u) / WORKGROUP;
    let begin = min(local.x * per, params.groups);
    let end = min(begin + per, params.groups);
    var sum = 0u;
    for (var index = begin; index < end; index = index + 1u) {
        sum = sum + group_offsets[index];
    }
    scan[local.x] = sum;
    scan_workgroup(local.x);
    var running = scan[local.x] - sum;
    for (var index = begin; index < end; index = index + 1u) {
        let group_count = group_offsets[index];
        group_offsets[index] = running;
        running = running + group_count;
    }
    if local.x == WORKGROUP - 1u {
        let mesh = meshes[params.mesh];
        args[0] = mesh.index_count;
        args[1] = scan[local.x];
        args[2] = mesh.first_index;
        args[3] = bitcast<u32>(mesh.base_vertex);
        args[4] = 0u;
    }
}

@compute @workgroup_size(256)
fn scatter(@builtin(global_invocation_id) global: vec3<u32>,
           @builtin(local_invocation_id) local: vec3<u32>,
           @builtin(workgroup_id) group: vec3<u32>) {
    let first = group.x * WORDS;
    let word_index = local.x / 32u;
    let lane = local.x % 32u;
    let word = masks[first + word_index];
    if ((word >> lane) & 1u) == 0u {
        return;
    }
    var position = group_offsets[group.x] + countOneBits(word & ((1u << lane) - 1u));
    for (var index = 0u; index < word_index; index = index + 1u) {
        position = position + countOneBits(masks[first + index]);
    }
    visible[position] = global.x;
}
