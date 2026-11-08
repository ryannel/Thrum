// Camera frustum compaction into one indexed indirect draw per bucket, and
// the two lists of occlusion. Frustum (phase 0) lists every slot in view.
// Prepass (phase 1) lists slots in view that were visible last frame; they
// draw depth only, and the pyramid is built from that depth. Final (phase 2)
// tests every slot in view against the pyramid, lists the ones it does not
// hide for the one ordered colour draw, and records them as the next
// last-visible bits on the other history side. The bits only choose
// occluders; the final list depends on them only through real depth.
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
    phase: u32,
}

const PHASE_FRUSTUM: u32 = 0u;
const PHASE_PREPASS: u32 = 1u;
const PHASE_FINAL: u32 = 2u;

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
// One last-visible bit per slot: the committed side, read by the prepass list.
@group(0) @binding(9) var<storage, read> bits: array<u32>;
// This frame's min depth pyramid, every mip; read by the final cull only.
@group(0) @binding(10) var pyramid: texture_2d<f32>;
// The other side, written by the final cull for the next frame.
@group(0) @binding(11) var<storage, read_write> next_bits: array<u32>;
// Member-indexed buckets (M3b, experimental): only the `*_members` entry
// points use these. Each bucket's segment of `members` starts at a multiple of
// WORKGROUP and holds physical slots, padded with MEMBER_PADDING, so every
// workgroup lies in one bucket. History bits are keyed by member index.
// WebGPU allows 8 storage buffers per stage and the flat path uses 8, so the
// member path reads its segments from a uniform and keeps its slot masks in
// `group_offsets` after the per-group counts (from `mask_base`), leaving
// `masks` unbound.
struct Segment {
    first_group: u32,
    groups: u32,
    mesh: u32,
    padding: u32,
}
const MAX_BUCKETS: u32 = 64u;
struct Buckets {
    count: u32,
    mask_base: u32,
    padding0: u32,
    padding1: u32,
    segments: array<Segment, MAX_BUCKETS>,
}
const MEMBER_PADDING: u32 = 0xffffffffu;
@group(0) @binding(12) var<storage, read> members: array<u32>;
@group(0) @binding(13) var<uniform> buckets: Buckets;
var<workgroup> group_mesh: u32;

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

struct Bounds {
    center: vec3<f32>,
    extent: vec3<f32>,
}

// Conservative world AABB of the slot's mesh at the interpolated translation
// the forward vertex shader uses.
fn world_bounds(slot: u32, mesh: MeshRecord) -> Bounds {
    let local_center = (mesh.bounds_min.xyz + mesh.bounds_max.xyz) * 0.5;
    let local_extent = (mesh.bounds_max.xyz - mesh.bounds_min.xyz) * 0.5;
    let basis = mat3x3(instances[slot].current[0].xyz, instances[slot].current[1].xyz,
        instances[slot].current[2].xyz);
    let translation = mix(instances[slot].previous[3].xyz, instances[slot].current[3].xyz,
        serials.presentation_alpha);
    let center = basis * local_center + translation;
    let extent = abs(basis[0]) * local_extent.x + abs(basis[1]) * local_extent.y
        + abs(basis[2]) * local_extent.z;
    return Bounds(center, extent);
}

fn in_view(slot: u32, mesh: MeshRecord) -> bool {
    if slot >= params.active_count {
        return false;
    }
    // Field loads, not a whole-record copy: this pass is bandwidth bound.
    if instances[slot].ids.w == 0u {
        return false;
    }
    let bounds = world_bounds(slot, mesh);
    let center = bounds.center;
    let extent = bounds.extent;
    let r0 = plane_row(0u);
    let r1 = plane_row(1u);
    let r2 = plane_row(2u);
    let r3 = plane_row(3u);
    return !(outside(r3 + r0, center, extent) || outside(r3 - r0, center, extent)
        || outside(r3 + r1, center, extent) || outside(r3 - r1, center, extent)
        || outside(r3 - r2, center, extent) || outside(r2, center, extent));
}

// The final classifier: the AABB's eight corners under the same view_proj,
// tested against the pyramid. Near-plane corners keep the slot visible.
// `magnitude` bounds the |clip x|, |clip y|, |clip z| and |clip w| terms for every position either
// path forms for this slot: the vertex shader's model * local, at the same
// interpolated translation, and these corners.
fn hidden_by_depth(slot: u32, mesh: MeshRecord) -> bool {
    let reach = max(abs(mesh.bounds_min.xyz), abs(mesh.bounds_max.xyz));
    let current = instances[slot].current;
    let spread = mat3x3(abs(current[0].xyz), abs(current[1].xyz), abs(current[2].xyz));
    let world = spread * reach + abs(instances[slot].previous[3].xyz) + abs(current[3].xyz);
    let rows = mat4x4(abs(plane_row(0u)), abs(plane_row(1u)), abs(plane_row(2u)), abs(plane_row(3u)));
    // Each component is |row| . (world, 1); transposed rows multiply as columns.
    let magnitude = vec4(world, 1.0) * rows;
    let bounds = world_bounds(slot, mesh);
    var corners: array<vec4<f32>, 8>;
    for (var index = 0u; index < 8u; index++) {
        let sign = vec3(select(-1.0, 1.0, (index & 1u) != 0u), select(-1.0, 1.0, (index & 2u) != 0u),
            select(-1.0, 1.0, (index & 4u) != 0u));
        corners[index] = view.view_proj * vec4(bounds.center + sign * bounds.extent, 1.0);
    }
    return occluded_by_pyramid(corners, magnitude);
}

fn drawn_last(index: u32) -> bool {
    return ((bits[index / 32u] >> (index % 32u)) & 1u) == 1u;
}

// Whether this phase lists `slot`, whose history bit is at `index`.
fn listed(slot: u32, index: u32, mesh: MeshRecord) -> bool {
    let framed = in_view(slot, mesh);
    if params.phase == PHASE_PREPASS {
        return framed && drawn_last(index);
    } else if params.phase == PHASE_FINAL {
        return framed && !hidden_by_depth(slot, mesh);
    }
    return framed;
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
    let word = count_word(listed(global.x, global.x, meshes[params.mesh]), local.x, group.x);
    if local.x < WORDS {
        masks[group.x * WORDS + local.x] = word;
        // The final list is exactly the next last-visible set.
        if params.phase == PHASE_FINAL {
            next_bits[group.x * WORDS + local.x] = word;
        }
    }
}

@compute @workgroup_size(256)
fn count_members(@builtin(global_invocation_id) global: vec3<u32>,
                 @builtin(local_invocation_id) local: vec3<u32>,
                 @builtin(workgroup_id) group: vec3<u32>) {
    // One invocation finds the workgroup's bucket; the search is per group.
    if local.x == 0u {
        group_mesh = buckets.segments[group_bucket(group.x)].mesh;
    }
    let mesh = meshes[workgroupUniformLoad(&group_mesh)];
    let slot = members[global.x];
    var draw = false;
    if slot != MEMBER_PADDING {
        draw = listed(slot, global.x, mesh);
    }
    let word = count_word(draw, local.x, group.x);
    if local.x < WORDS {
        group_offsets[buckets.mask_base + group.x * WORDS + local.x] = word;
        if params.phase == PHASE_FINAL {
            next_bits[group.x * WORDS + local.x] = word;
        }
    }
}

// The bucket whose segment holds workgroup `group`.
fn group_bucket(group: u32) -> u32 {
    let last = min(buckets.count, MAX_BUCKETS) - 1u;
    for (var bucket = 0u; bucket < last; bucket = bucket + 1u) {
        let segment = buckets.segments[bucket];
        if group < segment.first_group + segment.groups {
            return bucket;
        }
    }
    return last;
}

// Scan one workgroup's flags into its count, and return the 32-slot mask word
// of invocations below WORDS (zero above); every invocation must call it.
fn count_word(draw: bool, local: u32, group: u32) -> u32 {
    scan[local] = select(0u, 1u, draw);
    scan_workgroup(local);
    if local == WORKGROUP - 1u {
        group_offsets[group] = scan[local];
    }
    // Recover each slot's flag from the inclusive scan and pack 32 per word.
    var word = 0u;
    if local < WORDS {
        let base = local * 32u;
        var before = 0u;
        if base > 0u {
            before = scan[base - 1u];
        }
        for (var bit = 0u; bit < 32u; bit = bit + 1u) {
            let at = scan[base + bit];
            word = word | ((at - before) << bit);
            before = at;
        }
    }
    return word;
}

// One workgroup turns per-group counts into exclusive offsets and writes the
// bucket's indirect arguments, including the GPU-written instance count.
@compute @workgroup_size(256)
fn offsets(@builtin(local_invocation_id) local: vec3<u32>) {
    let total = scan_offsets(local.x, 0u, params.groups, 0u);
    if local.x == WORKGROUP - 1u {
        write_args(0u, meshes[params.mesh], total);
    }
}

// One workgroup per bucket: offsets restart at the bucket's segment start,
// and each bucket writes its own indirect arguments.
@compute @workgroup_size(256)
fn offsets_members(@builtin(local_invocation_id) local: vec3<u32>,
                   @builtin(workgroup_id) bucket: vec3<u32>) {
    let segment = buckets.segments[bucket.x];
    let total = scan_offsets(local.x, segment.first_group, segment.groups,
        segment.first_group * WORKGROUP);
    if local.x == WORKGROUP - 1u {
        write_args(bucket.x, meshes[segment.mesh], total);
    }
}

// Turn the counts of `groups` workgroups from `first` into exclusive offsets
// starting at `base`, and return the total; every invocation must call it.
fn scan_offsets(local: u32, first: u32, groups: u32, base: u32) -> u32 {
    let per = (groups + WORKGROUP - 1u) / WORKGROUP;
    let begin = first + min(local * per, groups);
    let end = min(begin + per, first + groups);
    var sum = 0u;
    for (var index = begin; index < end; index = index + 1u) {
        sum = sum + group_offsets[index];
    }
    scan[local] = sum;
    scan_workgroup(local);
    var running = base + scan[local] - sum;
    for (var index = begin; index < end; index = index + 1u) {
        let group_count = group_offsets[index];
        group_offsets[index] = running;
        running = running + group_count;
    }
    return scan[WORKGROUP - 1u];
}

fn write_args(record: u32, mesh: MeshRecord, instances: u32) {
    let at = record * 5u;
    args[at] = mesh.index_count;
    args[at + 1u] = instances;
    args[at + 2u] = mesh.first_index;
    args[at + 3u] = bitcast<u32>(mesh.base_vertex);
    args[at + 4u] = 0u;
}

@compute @workgroup_size(256)
fn scatter(@builtin(global_invocation_id) global: vec3<u32>,
           @builtin(local_invocation_id) local: vec3<u32>,
           @builtin(workgroup_id) group: vec3<u32>) {
    let word_index = local.x / 32u;
    let lane = local.x % 32u;
    let word = masks[group.x * WORDS + word_index];
    if ((word >> lane) & 1u) == 0u {
        return;
    }
    var before = countOneBits(word & ((1u << lane) - 1u));
    for (var index = 0u; index < word_index; index = index + 1u) {
        before = before + countOneBits(masks[group.x * WORDS + index]);
    }
    visible[group_offsets[group.x] + before] = global.x;
}

@compute @workgroup_size(256)
fn scatter_members(@builtin(global_invocation_id) global: vec3<u32>,
                   @builtin(local_invocation_id) local: vec3<u32>,
                   @builtin(workgroup_id) group: vec3<u32>) {
    let first = buckets.mask_base + group.x * WORDS;
    let word_index = local.x / 32u;
    let lane = local.x % 32u;
    let word = group_offsets[first + word_index];
    if ((word >> lane) & 1u) == 0u {
        return;
    }
    var before = countOneBits(word & ((1u << lane) - 1u));
    for (var index = 0u; index < word_index; index = index + 1u) {
        before = before + countOneBits(group_offsets[first + index]);
    }
    // A listed member is never padding: its flag came from a real slot.
    visible[group_offsets[group.x] + before] = members[global.x];
}
