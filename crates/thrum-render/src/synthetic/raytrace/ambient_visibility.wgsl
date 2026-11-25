@group(0) @binding(0) var<uniform> frame: AmbientFrame;
@group(0) @binding(1) var depth: texture_depth_2d;
@group(0) @binding(2) var motion: texture_2d<f32>;
@group(0) @binding(3) var previous_samples: texture_2d<u32>;
@group(0) @binding(4) var samples_out: texture_storage_2d<rgba32uint, write>;
@group(0) @binding(5) var current_samples: texture_2d<u32>;
@group(0) @binding(6) var filtered_out: texture_storage_2d<rg32float, write>;

fn world_at(pixel: vec2<i32>, z: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + 0.5) / vec2<f32>(frame.extent);
    let h = frame.inverse * vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), z, 1.0);
    return h.xyz / h.w;
}

fn plane_tangent(pixel: vec2<i32>, axis: vec2<i32>, z: f32, point: vec3<f32>) -> vec3<f32> {
    let bounds = vec2<i32>(frame.extent) - 1;
    let lo = max(pixel - axis, vec2(0));
    let hi = min(pixel + axis, bounds);
    let left = textureLoad(depth, lo, 0);
    let right = textureLoad(depth, hi, 0);
    if right > 0.0 && any(hi != pixel) &&
        (left <= 0.0 || all(lo == pixel) || abs(right - z) < abs(left - z)) {
        return world_at(hi, right) - point;
    }
    if left > 0.0 && any(lo != pixel) { return point - world_at(lo, left); }
    return world_at(pixel + axis, z) - point;
}

fn hash(seed: u32) -> u32 {
    var x = seed;
    x = (x ^ (x >> 16u)) * 0x7feb352du;
    x = (x ^ (x >> 15u)) * 0x846ca68bu;
    return x ^ (x >> 16u);
}

fn sample_direction(normal: vec3<f32>, pixel: vec2<u32>) -> vec3<f32> {
    let seed = hash(pixel.y) ^ hash(pixel.x + 0x9e3779b9u);
    let offset = vec2<f32>(vec2(hash(seed) >> 8u, hash(seed ^ 0xa511e9b3u) >> 8u)) / 16777216.0;
    let uv = fract(offset + f32(frame.phase % frame.settle) * vec2(0.754877666, 0.569840296));
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
    rayQueryInitialize(&query, scene_structure, RayDesc(flags, 2u, 0.0, frame.radius, origin, direction));
    while rayQueryProceed(&query) {}
    return select(0u, 1u, rayQueryGetCommittedIntersection(&query).kind == RAY_QUERY_INTERSECTION_NONE);
}

fn sample_pixel(pixel: vec2<i32>) -> vec2<i32> {
    return min(pixel * i32(frame.stride) + i32(frame.stride / 2u), vec2<i32>(frame.extent) - 1);
}

@compute @workgroup_size(AMBIENT_TILE, AMBIENT_TILE)
fn trace_ambient(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= ambient_extent(frame)) { return; }
    let texel = vec2<i32>(id.xy);
    let pixel = sample_pixel(texel);
    let z = textureLoad(depth, pixel, 0);
    if z <= 0.0 {
        textureStore(samples_out, texel, vec4(0u));
        return;
    }

    let point = world_at(pixel, z);
    let tangent = plane_tangent(pixel, vec2(1, 0), z, point);
    let bitangent = plane_tangent(pixel, vec2(0, 1), z, point);
    let plane = cross(bitangent, tangent);
    if dot(plane, plane) < 1e-24 {
        textureStore(samples_out, texel, vec4(0u));
        return;
    }
    let normal = normalize(select(plane, -plane, dot(plane, point) > 0.0));
    let world = point + frame.camera.xyz;
    let direction = sample_direction(normal, id.xy);
    let distance = length(point);
    // Raster depth reconstruction and the TLAS use different arithmetic.
    // Leave room for their distance-dependent error before casting at grazing angles.
    let lift = min(distance * 0.0001, frame.radius * 0.025);
    let origin = shadow_ray_origin_at_distance(world, normal, distance) + normal * lift;
    let vote = ray_visible(origin, direction);

    var prior = vec4(0u);
    if frame.valid != 0u {
        let shift = textureLoad(motion, pixel, 0).xy + frame.jitter_delta;
        let old_pixel = vec2<i32>(floor((vec2<f32>(pixel) + 0.5 + shift) / f32(frame.stride)));
        let clip = frame.previous * vec4(world, 1.0);
        if clip.w > 0.0 && ambient_in_bounds(old_pixel, ambient_extent(frame)) {
            let old = textureLoad(previous_samples, old_pixel, 0);
            if ambient_depth_weight(bitcast<f32>(old.z), clip.z / clip.w) > 0.25 &&
                dot(unpack4x8snorm(old.w).xyz, normal) > 0.85 {
                prior = old;
            }
        }
    }

    // Each vote expires on its own after exactly one settle window.
    let bits = ((prior.x << 1u) | vote) & ((1u << frame.settle) - 1u);
    textureStore(samples_out, texel, vec4(bits, min(prior.y + 1u, frame.settle),
        bitcast<u32>(z), pack4x8snorm(vec4(normal, 0.0))));
}

@compute @workgroup_size(AMBIENT_TILE, AMBIENT_TILE)
fn filter_ambient(@builtin(global_invocation_id) id: vec3<u32>) {
    let extent = ambient_extent(frame);
    if any(id.xy >= extent) { return; }
    let texel = vec2<i32>(id.xy);
    let centre = textureLoad(current_samples, texel, 0);
    let z = bitcast<f32>(centre.z);
    if centre.y == 0u {
        textureStore(filtered_out, texel, vec4(1.0, 0.0, 0.0, 0.0));
        return;
    }

    let point = world_at(sample_pixel(texel), z);
    let normal = unpack4x8snorm(centre.w).xyz;
    let footprint = max(length(point - world_at(sample_pixel(texel) + vec2(1, 0), z)), 1e-4);
    var occlusion = 0.0;
    var total = 0.0;
    for (var y = -2; y <= 2; y++) {
        for (var x = -2; x <= 2; x++) {
            let at = texel + vec2(x, y);
            if !ambient_in_bounds(at, extent) { continue; }
            let sample = textureLoad(current_samples, at, 0);
            if sample.y == 0u { continue; }
            let aligned = smoothstep(0.8, 0.98, dot(normal, unpack4x8snorm(sample.w).xyz));
            let relative = world_at(sample_pixel(at), bitcast<f32>(sample.z)) - point;
            let plane = 1.0 - smoothstep(0.5, 2.0, abs(dot(relative, normal)) / footprint);
            let weight = aligned * plane * exp(-0.33 * f32(x * x + y * y));
            occlusion += weight * f32(sample.y - countOneBits(sample.x)) / f32(sample.y);
            total += weight;
        }
    }
    textureStore(filtered_out, texel, vec4(select(1.0, 1.0 - occlusion / total, total > 0.0), z, 0.0, 0.0));
}
