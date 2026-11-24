@group(0) @binding(0) var<uniform> frame: SoftSunFrame;
@group(0) @binding(1) var depth: texture_depth_2d;
@group(0) @binding(2) var<uniform> sun: DirectionalLight;
@group(0) @binding(3) var rays_out: texture_storage_2d<rgba8uint, write>;
@group(0) @binding(4) var rays: texture_2d<u32>;
@group(0) @binding(5) var tiles_out: texture_storage_2d<rgba8uint, write>;
@group(0) @binding(6) var tiles: texture_2d<u32>;
@group(0) @binding(7) var motion: texture_2d<f32>;
@group(0) @binding(8) var previous: texture_2d<u32>;
@group(0) @binding(9) var samples_out: texture_storage_2d<rgba32uint, write>;
@group(0) @binding(10) var samples: texture_2d<u32>;
@group(0) @binding(11) var values_out: texture_storage_2d<rgba16float, write>;

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

fn sun_disc(pixel: vec2<u32>) -> vec3<f32> {
    let direction = sun.direction_to_light.xyz;
    let tangent = normalize(cross(direction, select(vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0), abs(direction.y) > 0.9)));
    let bitangent = cross(direction, tangent);
    let seed = hash(pixel.y) ^ hash(pixel.x + 0x9e3779b9u);
    let offset = vec2<f32>(vec2(hash(seed) >> 8u, hash(seed ^ 0xa511e9b3u) >> 8u)) / 16777216.0;
    let uv = fract(offset + f32(frame.phase % 4096u) * vec2(0.754877666, 0.569840296));
    let theta = 6.2831853 * uv.y;
    let radius = tan(frame.radius) * sqrt(uv.x);
    return normalize(direction + radius * (cos(theta) * tangent + sin(theta) * bitangent));
}

// Depth interpolation loses submillimetre correspondence against the TLAS
// on distant, shallow terrain. Locate the surface inside the depth footprint
// before applying the shared lift, without increasing the caster separation.
fn align_depth(point: vec3<f32>, normal: vec3<f32>, footprint: f32) -> vec3<f32> {
    let high = point + normal * footprint;
    let hit = closest_shadow_hit(high, -normal, 2.0 * footprint);
    if hit.kind == RAY_QUERY_INTERSECTION_NONE { return point; }
    return high - normal * hit.t;
}

@compute @workgroup_size(SOFT_SUN_TILE, SOFT_SUN_TILE)
fn trace_sun(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= frame.extent) { return; }
    let pixel = vec2<i32>(id.xy);
    let z = textureLoad(depth, pixel, 0);
    if z <= 0.0 {
        textureStore(rays_out, pixel, vec4(0u));
        return;
    }
    let point = world_at(pixel, z);
    let tangent = plane_tangent(pixel, vec2(1, 0), z, point);
    let bitangent = plane_tangent(pixel, vec2(0, 1), z, point);
    let normal = cross(bitangent, tangent);
    let normal_length = length(normal);
    if normal_length == 0.0 {
        textureStore(rays_out, pixel, vec4(0u));
        return;
    }
    let outward = normal / normal_length;
    let direction = sun_disc(id.xy);
    let cosine = dot(outward, direction);
    // A sample outside the limit has no history and no influence on neighbours.
    if cosine < -frame.dive_cosine {
        textureStore(rays_out, pixel, vec4(0u, 0u, 0u, 2u));
        return;
    }
    let distance = length(point);
    let footprint = distance * 0.00016;
    let align = footprint > 0.001;
    var world = point + frame.camera.xyz;
    if align { world = align_depth(world, outward, footprint); }
    let origin = shadow_ray_origin_at_distance(world, outward, distance);
    var visible: vec2<u32>;
    if cosine >= 0.0 {
        visible = vec2(select(1u, 0u, shadow_ray_blocked(origin, direction, 1e30)));
    } else {
        let inside = shadow_ray_origin_at_distance(world, -outward, distance);
        let crossing = closest_shadow_hit(inside, direction, 1e30);
        if crossing.kind == RAY_QUERY_INTERSECTION_NONE {
            visible = vec2(1u);
        } else if shadow_same_receiver(origin, inside, outward, crossing) {
            visible = vec2(select(1u, 0u, shadow_ray_blocked_from(inside, direction, shadow_after(crossing), 1e30)), 0u);
        } else {
            visible = vec2(0u);
        }
    }
    textureStore(rays_out, pixel, vec4(visible, 1u, select(0u, 1u, cosine < 0.0) | select(0u, 4u, align)));
}

// Each tile preserves the union of light and shadow samples for each channel.
var<workgroup> tile_rays: array<u32, SOFT_SUN_TILE * SOFT_SUN_TILE>;
@compute @workgroup_size(SOFT_SUN_TILE, SOFT_SUN_TILE)
fn reduce_sun(@builtin(global_invocation_id) id: vec3<u32>,
    @builtin(local_invocation_index) index: u32, @builtin(workgroup_id) tile: vec3<u32>) {
    var bits = 0u;
    if all(id.xy < frame.extent) {
        let ray = textureLoad(rays, vec2<i32>(id.xy), 0);
        if ray.z != 0u {
            bits = (1u << ray.x) | (4u << ray.y);
        }
    }
    tile_rays[index] = bits;
    workgroupBarrier();
    for (var stride = SOFT_SUN_TILE * SOFT_SUN_TILE / 2u; stride > 0u; stride >>= 1u) {
        if index < stride {
            tile_rays[index] |= tile_rays[index + stride];
        }
        workgroupBarrier();
    }
    if index == 0u {
        textureStore(tiles_out, vec2<i32>(tile.xy), vec4(tile_rays[0], 0u, 0u, 0u));
    }
}

@compute @workgroup_size(SOFT_SUN_TILE, SOFT_SUN_TILE)
fn accumulate_sun(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= frame.extent) { return; }
    let pixel = vec2<i32>(id.xy);
    let ray = textureLoad(rays, pixel, 0);
    if ray.z == 0u {
        textureStore(samples_out, pixel, vec4(0u));
        return;
    }

    let z = textureLoad(depth, pixel, 0);
    var prior = vec4(0u);
    if frame.valid != 0u {
        let shift = textureLoad(motion, pixel, 0).xy;
        if all(abs(shift) < vec2<f32>(frame.extent)) {
            let old_pixel = vec2<i32>(floor(vec2<f32>(pixel) + 0.5 - shift + frame.jitter_delta));
            if soft_within(old_pixel, frame.extent) {
                let old = textureLoad(previous, old_pixel, 0);
                let clip = frame.previous * vec4(world_at(pixel, z) + frame.camera.xyz, 1.0);
                if clip.w > 0.0 && soft_depth_matches(bitcast<f32>(old.z), clip.z / clip.w) {
                    prior = old;
                }
            }
        }
    }

    var neighbourhood = 0u;
    let tile = pixel / i32(SOFT_SUN_TILE);
    let tile_extent = (frame.extent + SOFT_SUN_TILE - 1u) / SOFT_SUN_TILE;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let at = tile + vec2(x, y);
            if soft_within(at, tile_extent) { neighbourhood |= textureLoad(tiles, at, 0).x; }
        }
    }

    // The retained samples are binary, so their second moment equals their
    // first. A bounded shift register discards a sample after exactly settle
    // submitted frames. Each channel resets independently when the rays agree.
    let mask = (1u << frame.settle) - 1u;
    var counts = min(vec2(prior.w & 31u, (prior.w >> 5u) & 31u) + 1u, vec2(frame.settle));
    var bits = ((prior.xy << vec2(1u)) | ray.xy) & vec2(mask);
    var settled = 0u;
    for (var channel = 0u; channel < 2u; channel++) {
        let votes = (neighbourhood >> (2u * channel)) & 3u;
        if votes != 3u {
            settled |= 1u << channel;
            bits[channel] = select(0u, mask, ray[channel] != 0u);
            counts[channel] = frame.settle;
        }
    }
    let flags = counts.x | (counts.y << 5u) | (settled << 10u);
    textureStore(samples_out, pixel, vec4(bits, bitcast<u32>(z), flags));
}

fn sample_mean(sample: vec4<u32>) -> vec2<f32> {
    return vec2<f32>(countOneBits(sample.xy)) / vec2<f32>(vec2(sample.w & 31u, (sample.w >> 5u) & 31u));
}

@compute @workgroup_size(SOFT_SUN_TILE, SOFT_SUN_TILE)
fn filter_sun(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= frame.extent) { return; }
    let pixel = vec2<i32>(id.xy);
    let own = textureLoad(samples, pixel, 0);
    if own.w == 0u {
        textureStore(values_out, pixel, vec4(0.0));
        return;
    }
    let centre = sample_mean(own);
    let settled = vec2<bool>((own.w & 1024u) != 0u, (own.w & 2048u) != 0u);
    if all(settled) {
        textureStore(values_out, pixel, vec4(centre, 1.0, 1.0));
        return;
    }
    let z = bitcast<f32>(own.z);
    let point = world_at(pixel, z);
    let plane = normalize(cross(plane_tangent(pixel, vec2(0, 1), z, point),
        plane_tangent(pixel, vec2(1, 0), z, point)));
    let pixel_world = length(world_at(pixel + vec2(1, 0), z) - point);
    let variance = centre * (vec2(1.0) - centre);
    let step_size = select(1, 2, max(variance.x, variance.y) > 0.12);
    var total = centre * 4.0;
    var weight = 4.0;
    for (var y = -2; y <= 2; y++) {
        for (var x = -2; x <= 2; x++) {
            if (x == 0 && y == 0) || abs(x) + abs(y) == 4 { continue; }
            let at = pixel + vec2(x, y) * step_size;
            if !soft_within(at, frame.extent) { continue; }
            let neighbour = textureLoad(samples, at, 0);
            if neighbour.w == 0u { continue; }
            let neighbour_point = world_at(at, bitcast<f32>(neighbour.z));
            let plane_distance = abs(dot(plane, neighbour_point - point));
            let tap = exp2(-4.0 * plane_distance / max(pixel_world, 1e-5)) /
                (1.0 + f32(x * x + y * y));
            total += tap * sample_mean(neighbour);
            weight += tap;
        }
    }
    textureStore(values_out, pixel,
        vec4(select(total / weight, centre, settled), select(vec2(0.0), vec2(1.0), settled)));
}
