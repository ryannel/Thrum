@group(2) @binding(0) var<uniform> ambient_frame: AmbientFrame;
@group(2) @binding(2) var ambient_motion: texture_2d<f32>;
@group(2) @binding(3) var ambient_previous_samples: texture_2d<u32>;
@group(2) @binding(4) var ambient_samples_out: texture_storage_2d<rgba32uint, write>;

fn sample_direction(normal: vec3<f32>, pixel: vec2<u32>) -> vec3<f32> {
    let seed = hash(pixel.y) ^ hash(pixel.x + 0x9e3779b9u);
    let phase = ambient_frame.phase % ambient_frame.settle;
    let sequence = seed ^ hash(phase + 0x63d83595u);
    let jitter = vec2<f32>(vec2(hash(sequence) >> 8u, hash(sequence ^ 0xa511e9b3u) >> 8u)) / 16777216.0;
    // Each 4×4 receiver neighbourhood covers the hemisphere even during
    // warmup. Each pixel visits all 16 strata in one default settle window.
    let stratum = (pixel + vec2(phase, phase / 4u)) % vec2(4u);
    let uv = (vec2<f32>(stratum) + jitter) / 4.0;
    let tangent = normalize(cross(normal, select(vec3(0.0, 1.0, 0.0),
        vec3(1.0, 0.0, 0.0), abs(normal.y) > 0.9)));
    let bitangent = cross(normal, tangent);
    let theta = 6.28318530718 * uv.y;
    return sqrt(uv.x) * (cos(theta) * tangent + sin(theta) * bitangent)
        + sqrt(1.0 - uv.x) * normal;
}

fn ray_visible(origin: vec3<f32>, direction: vec3<f32>) -> u32 {
    var query: ray_query;
    let flags = RAY_FLAG_FORCE_OPAQUE | RAY_FLAG_TERMINATE_ON_FIRST_HIT;
    rayQueryInitialize(&query, scene_structure, RayDesc(flags, 2u, 0.0, ambient_frame.radius, origin, direction));
    while rayQueryProceed(&query) {}
    return select(0u, 1u, rayQueryGetCommittedIntersection(&query).kind == RAY_QUERY_INTERSECTION_NONE);
}

// Snap the view-depth receiver back onto its triangle before applying the
// shared origin rule. Search from behind the raster depth so a close sheet
// in front of the receiver cannot replace the first surface.
fn ambient_receiver_world(point: vec3<f32>, distance: f32) -> vec3<f32> {
    let camera = ambient_frame.camera.xyz;
    let band = distance * 0.00016;
    if band <= 0.001 { return camera + point; }
    let direction = point / distance;
    let behind = camera + direction * (distance + band);
    let hit = closest_masked_hit_from(behind, -direction, 0.0, 2.0 * band, 3u);
    if hit.kind == RAY_QUERY_INTERSECTION_NONE { return camera + point; }
    return behind - direction * hit.t;
}

fn trace_ambient_pixel(texel: vec2<i32>, pixel: vec2<i32>, receiver: VisibilityReceiver) {
    let point = receiver.point;
    let plane = receiver.normal;
    let z = receiver.z;
    if dot(plane, plane) < 1e-24 {
        textureStore(ambient_samples_out, texel, vec4(0u));
        return;
    }
    let normal = normalize(select(plane, -plane, dot(plane, point) > 0.0));
    let distance = length(point);
    let world = ambient_receiver_world(point, distance);
    let direction = sample_direction(normal, vec2<u32>(texel));
    let origin = shadow_ray_origin_at_distance(world, normal, distance);
    let vote = ray_visible(origin, direction);

    var prior = vec4(0u);
    if ambient_frame.valid != 0u {
        let shift = textureLoad(ambient_motion, pixel, 0).xy + ambient_frame.jitter_delta;
        let old_pixel = vec2<i32>(floor((vec2<f32>(pixel) + 0.5 + shift) / f32(ambient_frame.stride)));
        let clip = ambient_frame.previous * vec4(point + ambient_frame.camera.xyz, 1.0);
        if clip.w > 0.0 && ambient_in_bounds(old_pixel, ambient_extent(ambient_frame)) {
            let old = textureLoad(ambient_previous_samples, old_pixel, 0);
            if ambient_depth_weight(bitcast<f32>(old.z), clip.z / clip.w) > 0.25 &&
                dot(unpack4x8snorm(old.w).xyz, normal) > 0.85 {
                prior = old;
            }
        }
    }

    // Each vote expires on its own after exactly one settle window.
    let bits = ((prior.x << 1u) | vote) & ((1u << ambient_frame.settle) - 1u);
    textureStore(ambient_samples_out, texel, vec4(bits, min(prior.y + 1u, ambient_frame.settle),
        bitcast<u32>(z), pack4x8snorm(vec4(normal, 0.0))));
}

