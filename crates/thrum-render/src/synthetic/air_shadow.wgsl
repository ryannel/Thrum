// Sun shadow heights for the air. For each column x of the top-down caster
// height map, H(x) = max_i (h_i − d_i·tan e) over casters at distance d_i toward
// the Sun's azimuth: air at x below H is hidden from the Sun's centre. G holds
// the maximising caster's distance for the penumbra. Steps grow geometrically
// from two height texels to the map edge, as the receiver horizon march does.
struct ShadowCascades {
    light_view_proj: array<mat4x4<f32>, 2>,
    split_overlap: vec4<f32>,
    bias: vec4<f32>,
    penumbra: vec4<f32>,
    height_support: vec4<f32>,
    height_view_proj: mat4x4<f32>,
    horizon: vec4<f32>,
}
struct DirectionalSource {
    direction_to_light: vec4<f32>,
}
@group(0) @binding(0) var<uniform> shadows: ShadowCascades;
@group(0) @binding(1) var shadow_maps: texture_depth_2d_array;
@group(0) @binding(2) var<uniform> light: DirectionalSource;

// Must match `air_shadow::SIZE`; the pass draws one fullscreen triangle into it.
const SIZE: f32 = 512.0;

const STEPS: u32 = 96u;
const NONE: f32 = -1e30;

@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return vec4(positions[index], 0.0, 1.0);
}

// A render pass, like the opaque pass that samples the same maps: its read of the
// height layer follows the caster passes through ordinary render-pass ordering.
@fragment fn build(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let horizon = shadows.horizon;
    let l = light.direction_to_light.xyz;
    let flat = length(l.xz);
    if horizon.x <= 0.0 || flat < 1e-4 {
        return vec4(NONE, 0.0, 0.0, 0.0);
    }
    // Invert the affine top-down mapping clip.xy = A·(x,z) + c for this texel centre.
    let m = shadows.height_view_proj;
    let uv = position.xy / SIZE;
    let clip = vec2(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0) - vec2(m[3][0], m[3][1]);
    let a = vec2(m[0][0], m[0][1]);
    let b = vec2(m[2][0], m[2][1]);
    let det = a.x * b.y - b.x * a.y;
    let origin = vec2((clip.x * b.y - b.x * clip.y) / det, (a.x * clip.y - clip.x * a.y) / det);
    let azimuth = l.xz / flat;
    let slope = l.y / flat;
    let texel = horizon.w;
    let start = 2.0 * texel;
    let reach = 1.0 / max(horizon.z, 1e-20);
    let ratio = pow(reach / start, 1.0 / f32(STEPS - 1u));
    let extent = vec2<f32>(textureDimensions(shadow_maps));
    let ceiling = shadows.height_support.x;
    var best = NONE;
    var best_distance = 0.0;
    var distance = start;
    for (var i = 0u; i < STEPS; i++) {
        // Farther casters fall by slope·d; none can beat the best once the
        // tallest caster cannot.
        if slope >= 0.0 && ceiling - distance * slope <= best { break; }
        let q = origin + azimuth * distance;
        let c = m * vec4(q.x, 0.0, q.y, 1.0);
        let sample_uv = vec2(c.x * 0.5 + 0.5, 0.5 - c.y * 0.5);
        if any(sample_uv < vec2(0.0)) || any(sample_uv > vec2(1.0)) { break; }
        let index = clamp(vec2<i32>(sample_uv * extent), vec2(0), vec2<i32>(extent) - 1);
        let depth = textureLoad(shadow_maps, index, 2, 0);
        if depth > 0.0 {
            let candidate = (depth - horizon.y) / horizon.x - distance * slope;
            if candidate > best {
                best = candidate;
                best_distance = distance;
            }
        }
        distance *= ratio;
    }
    return vec4(best, best_distance, 0.0, 0.0);
}
