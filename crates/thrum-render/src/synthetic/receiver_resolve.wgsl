// Current-frame receiver-mask diagnostic. The integer accumulation matches
// the frozen CPU oracle. Each group shares the 8×8 tile and its 1-pixel halo.
struct ResolveView {
    inverse_view_proj: mat4x4<f32>,
}

struct Receiver {
    normal: vec3<f32>,
    packed: u32,
    depth: f32,
    homogeneous_w: f32,
}

@group(0) @binding(0) var<uniform> view: ResolveView;
@group(0) @binding(1) var depth: texture_depth_2d;
@group(0) @binding(2) var guide: texture_2d<u32>;
@group(0) @binding(3) var resolved: texture_storage_2d<r32float, write>;
var<workgroup> tile: array<Receiver, 100>;

const FILTERABLE = 3u;
const QUANTIZED_ONE = 4095u;
const MIN_NORMAL_DOT = 0.996;
const MIN_PLANE_DISTANCE_METRES = 0.0004;
const MAX_PLANE_SLOPE = 0.015;

fn oct_normal(packed: u32) -> vec3<f32> {
    let oct = vec2<f32>(vec2<u32>(packed >> 12u, packed >> 20u) & vec2(255u)) * (2.0 / 255.0) - 1.0;
    let z = 1.0 - abs(oct.x) - abs(oct.y);
    let xy = select(
        (1.0 - abs(oct.yx)) * select(vec2(-1.0), vec2(1.0), oct >= vec2(0.0)),
        oct, z >= 0.0);
    return normalize(vec3(xy, z));
}

fn clip_position(pixel: vec2<i32>, size: vec2<i32>, z: f32) -> vec4<f32> {
    let screen = (vec2<f32>(pixel) + 0.5) / vec2<f32>(size);
    return vec4(screen.x * 2.0 - 1.0, 1.0 - screen.y * 2.0, z, 1.0);
}

fn receiver(pixel: vec2<i32>, size: vec2<i32>) -> Receiver {
    let invalid = Receiver(vec3(0.0), 0u, 0.0, 0.0);
    if any(pixel < vec2(0)) || any(pixel >= size) { return invalid; }
    let packed = textureLoad(guide, pixel, 0).x;
    if packed >> 28u != FILTERABLE { return invalid; }
    let z = textureLoad(depth, pixel, 0);
    if !(z > 0.0 && z <= 1.0) { return invalid; }
    let inv = view.inverse_view_proj;
    let w = dot(vec4(inv[0].w, inv[1].w, inv[2].w, inv[3].w), clip_position(pixel, size, z));
    if !(abs(w) > 1e-10) { return invalid; }
    return Receiver(oct_normal(packed), packed, z, w);
}

fn comparable(centre: Receiver, peer: Receiver, separation: vec3<f32>) -> bool {
    if dot(centre.normal, peer.normal) < MIN_NORMAL_DOT { return false; }
    let tolerance = max(MIN_PLANE_DISTANCE_METRES, MAX_PLANE_SLOPE * length(separation));
    return abs(dot(centre.normal, separation)) <= tolerance
        && abs(dot(peer.normal, separation)) <= tolerance;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(workgroup_id) group: vec3<u32>,
        @builtin(local_invocation_id) local: vec3<u32>,
        @builtin(local_invocation_index) lane: u32) {
    let size = vec2<i32>(textureDimensions(guide));
    let origin = vec2<i32>(group.xy * 8u) - 1;
    for (var index = lane; index < 100u; index += 64u) {
        let at = vec2<i32>(vec2(index % 10u, index / 10u));
        tile[index] = receiver(origin + at, size);
    }
    workgroupBarrier();

    let pixel = vec2<i32>(group.xy * 8u + local.xy);
    if any(pixel >= size) { return; }
    let index = i32((local.y + 1u) * 10u + local.x + 1u);
    let centre = tile[index];
    if centre.packed == 0u {
        textureStore(resolved, pixel, vec4(-1.0));
        return;
    }
    // For C = inv * centre_clip and D = inv * (peer_clip - centre_clip),
    // (C.xyz + D.xyz)/(C.w + D.w) - C.xyz/C.w
    // = (D.xyz - (C.xyz/C.w) * D.w) / peer.w.
    // Form D from pixel offsets and the depth difference to avoid
    // subtracting nearby positions already rounded to world-scale f32.
    let inv = view.inverse_view_proj;
    let centre_world = (inv * clip_position(pixel, size, centre.depth)).xyz / centre.homogeneous_w;
    if !all(abs(centre_world) <= vec3(3.402823e38)) {
        textureStore(resolved, pixel, vec4(-1.0));
        return;
    }
    let x_axis = (inv[0].xyz - centre_world * inv[0].w) * (2.0 / f32(size.x));
    let y_axis = (inv[1].xyz - centre_world * inv[1].w) * (-2.0 / f32(size.y));
    let z_axis = inv[2].xyz - centre_world * inv[2].w;
    var weighted = 4u * (centre.packed & QUANTIZED_ONE);
    var weights = 4u;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            if x == 0 && y == 0 { continue; }
            let peer = tile[index + y * 10 + x];
            if peer.packed == 0u { continue; }
            let separation = (x_axis * f32(x) + y_axis * f32(y)
                + z_axis * (peer.depth - centre.depth)) / peer.homogeneous_w;
            if !comparable(centre, peer, separation) { continue; }
            let weight = u32((2 - abs(x)) * (2 - abs(y)));
            weighted += weight * (peer.packed & QUANTIZED_ONE);
            weights += weight;
        }
    }
    let quantized = (weighted + weights / 2u) / weights;
    textureStore(resolved, pixel, vec4(f32(quantized) / f32(QUANTIZED_ONE)));
}
