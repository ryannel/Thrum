@group(1) @binding(12) var<uniform> scene_exposure: vec4<f32>;
fn scene_storage_scale() -> f32 { return scene_exposure.x; }

// ABI2 opaque clustered forward diagnostic: Lambert + GGX/Smith/Schlick.
// Checked finite reverse-Z camera; prepared lighting selects indirect resources.
@group(0) @binding(0) var<uniform> view: ViewRecord;
@group(0) @binding(1) var<uniform> view_aux: ViewAuxRecord;
@group(1) @binding(0) var<uniform> light: DirectionalLight;
@group(1) @binding(18) var<uniform> environment_light: EnvironmentLight;
struct LocalSource {
    position_radius: vec4<f32>,
    candela: vec4<f32>,
}
@group(1) @binding(1) var<storage, read> local_lights: array<LocalSource>;
@group(1) @binding(2) var<storage, read> cluster_heads: array<ClusterHead>;
@group(1) @binding(3) var<storage, read> cluster_indices: array<u32>;
@group(0) @binding(2) var<uniform> shadows: ShadowCascades;
@group(0) @binding(3) var shadow_maps: texture_depth_2d_array;
struct MaterialRecord {
    base: vec4<f32>,
    emissive_cutoff: vec4<f32>,
    surface: vec4<f32>,
    flags: vec4<u32>,
    texture_layers: vec4<u32>,
}
@group(2) @binding(0) var<storage, read> materials: array<MaterialRecord>;
@group(2) @binding(1) var base_textures: texture_2d_array<f32>;
@group(2) @binding(2) var normal_textures: texture_2d_array<f32>;
@group(2) @binding(3) var orm_textures: texture_2d_array<f32>;
@group(2) @binding(4) var emissive_textures: texture_2d_array<f32>;
@group(2) @binding(5) var material_sampler: sampler;
@group(3) @binding(0) var<storage, read> instances: array<InstanceRecord>;
// Camera-visible physical slots, read only by `compacted_vertex`.
@group(0) @binding(5) var<storage, read> visible: array<u32>;
@group(3) @binding(1) var<storage, read> history: array<HistoryRecord>;
@group(3) @binding(2) var<uniform> serials: Serials;
// One mesh-arena vertex (`mesh_arena::MeshVertex`, 48 bytes): position and
// normal.x, normal.yz and uv, then tangent and bitangent sign.
@group(3) @binding(3) var<storage, read> mesh_vertices: array<MeshVertex>;
struct VertexOutput {
    // Invariant, so the depth prepass rasterizes the depth the forward pass writes.
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) material: u32,
    @location(2) normal: vec3<f32>,
    @location(3) world_position: vec3<f32>,
    @location(4) clip_delta: vec4<f32>,
    @location(5) @interpolate(flat) history_valid: u32,
    @location(6) current_clip: vec4<f32>,
}
// Direct draws read the physical slot; compacted draws fetch it from the
// camera-visible list. History always uses the physical slot.
@vertex fn vertex(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    return vertex_at(vertex_index, instance_index);
}
@vertex fn compacted_vertex(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    return vertex_at(vertex_index, visible[instance_index]);
}
fn vertex_at(vertex_index: u32, slot: u32) -> VertexOutput {
    // The draw's index range and base vertex select the mesh in the arena.
    let vertex = mesh_vertices[vertex_index];
    let local = vertex.a.xyz;
    let n = vec3(vertex.a.w, vertex.b.xy);
    let instance = instances[slot];
    var output: VertexOutput;
    var model = instance.current;
    model[3] = vec4(mix(instance.previous[3].xyz, instance.current[3].xyz, serials.presentation_alpha), instance.current[3].w);
    let world = model * vec4(local,1.0);
    output.position = view.view_proj * world;
    output.current_clip = view.unjittered_view_proj * world;
    if instance.ids.w == 0u { output.position = vec4(2.0, 2.0, 2.0, 1.0); }
    output.world_position = world.xyz;
    output.normal = mat3x3(instance.normal[0].xyz,instance.normal[1].xyz,instance.normal[2].xyz) * n;
    output.uv = vertex.b.zw;
    output.material = instance.ids.y;
    let prior = history[slot];
    let valid = (serials.previous_lo != 0u || serials.previous_hi != 0u)
        && prior.identity.x == instance.ids.x
        && prior.identity.y == instance.ids.w
        && prior.identity.z == serials.previous_lo
        && prior.identity.w == serials.previous_hi;
    output.history_valid = select(0u, 1u, valid);
    // Form the difference before projection/interpolation. Identical submitted
    // camera and pose give exact zero without subtracting two rounded clips.
    let camera_delta = view.previous_view_proj - view.unjittered_view_proj;
    let world_delta = (prior.current - model) * vec4(local, 1.0);
    output.clip_delta = select(vec4(0.0),
        camera_delta * world + view.previous_view_proj * world_delta, valid);
    return output;
}
fn direct_brdf(base: vec3<f32>, metalness: f32, roughness: f32, n: vec3<f32>,
               l: vec3<f32>, v: vec3<f32>, irradiance: vec3<f32>, h: vec3<f32>, fresnel_power: f32) -> vec3<f32> {
    let ci = clamp(dot(n,l),0.0,1.0);
    let co = clamp(dot(n,v),0.0,1.0);
    if ci <= 0.0 || co <= 0.0 { return vec3(0.0); }
    let ch = clamp(dot(n,h),0.0,1.0);
    let alpha = ggx_alpha(roughness);
    let a2 = alpha*alpha;
    let d_base = (1.0-ch*ch)+ch*ch*a2;
    let d = a2/(3.14159265359*d_base*d_base);
    let f0 = mix(vec3(0.04),base,metalness);
    let fresnel = f0+(vec3(1.0)-f0)*fresnel_power;
    // Cancel Smith's ci*co and the outgoing cosine, then scale the remaining
    // ci/co pair. A grazing denominator cannot underflow into 0/0.
    let cosine_scale = max(ci,co);
    let ci_scaled = ci/cosine_scale;
    let co_scaled = co/cosine_scale;
    let smith_denominator = 2.0*(ci_scaled*sqrt(a2+(1.0-a2)*co*co)+co_scaled*sqrt(a2+(1.0-a2)*ci*ci));
    let specular_cosine = d*fresnel*ci_scaled/smith_denominator;
    let diffuse = (1.0-metalness)*(vec3(1.0)-fresnel)*base/3.14159265359;
    return (diffuse*ci+specular_cosine)*irradiance;
}
fn local_radiance(base: vec3<f32>, metalness: f32, roughness: f32,
                  n: vec3<f32>, v: vec3<f32>, world_position: vec3<f32>, screen_position: vec4<f32>) -> vec3<f32> {
    if light.direction_to_light.w == 0.0 { return vec3(0.0); }
    var cluster = vec3<u32>(
        u32(clamp(floor((world_position.x+1.0)* (0.5 * f32(CLUSTER_X))),0.0,f32(CLUSTER_X - 1u))),
        u32(clamp(floor((world_position.y+1.0)* (0.5 * f32(CLUSTER_Y))),0.0,f32(CLUSTER_Y - 1u))),
        u32(clamp(floor(world_position.z*f32(CLUSTER_Z)),0.0,f32(CLUSTER_Z - 1u))));
    if view_aux.flags.x == 1u {
        let near = view_aux.near_far.x;
        let far = view_aux.near_far.y;
        let depth = near*far/(near+screen_position.z*(far-near));
        cluster = vec3<u32>(
            u32(clamp(floor(screen_position.x*view_aux.viewport_inv_viewport.z*f32(CLUSTER_X)),0.0,f32(CLUSTER_X - 1u))),
            u32(clamp(floor((1.0-screen_position.y*view_aux.viewport_inv_viewport.w)*f32(CLUSTER_Y)),0.0,f32(CLUSTER_Y - 1u))),
            u32(clamp(floor(log(depth/near)/log(far/near)*f32(CLUSTER_Z)),0.0,f32(CLUSTER_Z - 1u))));
    }
    let head = cluster_heads[(cluster.z*CLUSTER_Y+cluster.y)*CLUSTER_X+cluster.x];
    var sum = vec3(0.0);
    for (var i = 0u; i < head.count; i++) {
        let source = local_lights[cluster_indices[head.offset+i]];
        let displacement = source.position_radius.xyz-world_position;
        let distance_squared = dot(displacement,displacement);
        let radius = source.position_radius.w;
        if distance_squared >= radius*radius { continue; }
        let distance = sqrt(distance_squared);
        var l = vec3(0.0,0.0,-1.0);
        if distance_squared > 0.000000000001 { l = displacement/distance; }
        let hv = l+v;
        var h = v;
        if dot(hv,hv)>0.000001 { h = normalize(hv); }
        let fresnel_power = pow(clamp(1.0-dot(v,h),0.0,1.0),5.0);
        let radial = distance/radius;
        let fade = max(0.0,1.0-radial*radial*radial*radial);
        let irradiance = source.candela.rgb*(fade*fade/max(distance_squared,0.000001));
        sum += direct_brdf(base,metalness,roughness,n,l,v,irradiance,h,fresnel_power);
    }
    return sum;
}
struct ShadowReceiver {
    position: vec3<f32>,
    dx: vec3<f32>,
    dy: vec3<f32>,
    normal: vec3<f32>,
}

// Fixed equal-area disk samples. No per-frame stochastic rotation.
const SHADOW_DISK = array<vec2<f32>,32>(
    vec2<f32>(0.125000000,0.000000000),
    vec2<f32>(-0.159645045,0.146247939),
    vec2<f32>(0.024436233,-0.278438271),
    vec2<f32>(0.201222239,0.262458779),
    vec2<f32>(-0.369267557,-0.065318231),
    vec2<f32>(0.349802466,-0.222515696),
    vec2<f32>(-0.117002079,0.435241902),
    vec2<f32>(-0.223135654,-0.429634123),
    vec2<f32>(0.484115115,0.176798064),
    vec2<f32>(-0.503641109,0.207895728),
    vec2<f32>(0.242788294,-0.518824483),
    vec2<f32>(0.179414374,0.572001296),
    vec2<f32>(-0.540757006,-0.313379738),
    vec2<f32>(0.634369523,-0.139464360),
    vec2<f32>(-0.387145845,0.550675126),
    vec2<f32>(-0.089439655,-0.690199644),
    vec2<f32>(0.549071757,0.462758258),
    vec2<f32>(-0.738878471,0.030554945),
    vec2<f32>(0.538955125,-0.536332334),
    vec2<f32>(-0.036058186,0.779791515),
    vec2<f32>(-0.512817532,-0.614526793),
    vec2<f32>(0.812359593,0.109301834),
    vec2<f32>(-0.688310641,0.478908616),
    vec2<f32>(0.188086057,-0.836061382),
    vec2<f32>(0.435033266,0.759191055),
    vec2<f32>(-0.850448410,-0.271316239),
    vec2<f32>(0.826102401,-0.381680263),
    vec2<f32>(-0.357888202,0.855155562),
    vec2<f32>(-0.319407334,-0.888033758),
    vec2<f32>(0.849908635,0.446688159),
    vec2<f32>(-0.944034646,0.248844503),
    vec2<f32>(0.536595808,-0.834529771)
);

// A continuous surface inside the filter footprint occludes only if it rises
// relative to the receiver plane faster than the Sun's elevation above it,
// tan(e)=1/g for receiver depth slope g. Accept 90% of that, capped.
const SURFACE_ELEVATION_FRACTION: f32 = 0.9;
const SURFACE_MAX_SLOPE: f32 = 0.25;
const SURFACE_MAX_GRADIENT: f32 = 30.0;

// A planar receiver predicts each neighboring texel's depth. Continuous terrain
// bends away from that plane; at grazing light a height deviation h appears as
// h/(n.l) along the ray. Accept bounded bending that grows with the tap's world
// offset, so a concave join near the receiver is not its own blocker.
fn surface_tolerance_per_uv(gradient: vec2<f32>, uv_metres_span: f32) -> f32 {
    let slope = min(length(gradient) * uv_metres_span, SURFACE_MAX_GRADIENT);
    let bend = min(SURFACE_ELEVATION_FRACTION / max(slope, 1e-3), SURFACE_MAX_SLOPE);
    return bend * (1.0 + slope * slope) / max(uv_metres_span, 1e-20);
}

const SUN_SEARCH_TEXELS: f32 = 8.0;
const SUN_MIN_FILTER_RADIUS: f32 = 1.25;
const NO_BLOCKER: f32 = 1e30;

struct CascadeReceiver {
    uv: vec2<f32>,
    depth: f32,
    gradient: vec2<f32>,
    bias: f32,
    epsilon: f32,
    metres_to_depth: f32,
    texel_metres: f32,
    tolerance_per_uv: f32,
};

fn cascade_receiver(index: u32, receiver: ShadowReceiver, size: f32) -> CascadeReceiver {
    let matrix = shadows.light_view_proj[index];
    let p = (matrix * vec4(receiver.position, 1.0)).xyz;
    let uv = vec2(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    let dx = (matrix * vec4(receiver.dx, 0.0)).xyz;
    let dy = (matrix * vec4(receiver.dy, 0.0)).xyz;
    let plane = cross(dx, dy);
    var gradient = vec2(0.0);
    if abs(plane.z) > max(1e-20, max(abs(plane.x), abs(plane.y)) * 1e-6) {
        gradient = -plane.xy / plane.z * vec2(2.0, -2.0);
    }
    let metrics = shadows.cascade_metrics[index];
    let footprint = max(abs(gradient.x), abs(gradient.y)) * (0.25 / size);
    let normal_bias = 1.0 - clamp(dot(receiver.normal, light.direction_to_light.xyz), 0.0, 1.0);
    let bias = clamp(footprint, metrics.w, 8.0 * metrics.w) + 0.25 * metrics.w * normal_bias;
    let epsilon = max(0.06 * metrics.y * metrics.z, 4e-7);
    let tolerance = surface_tolerance_per_uv(gradient, 1.0 / (size * metrics.y * metrics.z));
    return CascadeReceiver(uv, p.z, gradient, bias, epsilon, metrics.z, metrics.y, tolerance);
}

fn caster_gap(index: u32, receiver: CascadeReceiver, texel: vec2<i32>, size: f32,
    phase: vec2<f32>) -> f32 {
    let uv = (vec2<f32>(texel) + 0.5 + phase) / size;
    let delta = uv - receiver.uv;
    let depth = textureLoad(shadow_maps, texel, i32(index), 0);
    let gap = depth - receiver.depth - dot(receiver.gradient, delta);
    let tolerance = receiver.epsilon + receiver.tolerance_per_uv * length(delta);
    if depth <= 0.0 || depth <= receiver.depth + receiver.epsilon || gap <= max(receiver.bias, tolerance) {
        return 0.0;
    }
    return gap / receiver.metres_to_depth;
}

fn nearest_blocker(index: u32, receiver: CascadeReceiver, size: f32, radius: f32, lower: f32,
    upper: f32) -> vec2<f32> {
    var nearest = NO_BLOCKER;
    var centre_gap = 0.0;
    for (var tap = 0u; tap < 9u; tap++) {
        let offset = select(SHADOW_DISK[(tap * 4u + 1u) % 32u], vec2(0.0), tap == 0u);
        let pixel = receiver.uv * size + offset * radius;
        if any(pixel < vec2(0.0)) || any(pixel >= vec2(size)) { continue; }
        let gap = caster_gap(index, receiver, vec2<i32>(pixel), size, vec2(0.0));
        if tap == 0u { centre_gap = gap; }
        if gap > lower && gap <= upper { nearest = min(nearest, gap); }
    }
    return vec2(nearest, centre_gap);
}

// For the minimum footprint each map texel is integrated once. Fractional
// footprint coverage keeps the ramp continuous across the snapped raster.
fn contact_filter(layer: u32, receiver: CascadeReceiver, size: f32, lower: f32, upper: f32,
    phase: vec2<f32>) -> f32 {
    let centre = receiver.uv * size - phase;
    let lo = centre - SUN_MIN_FILTER_RADIUS;
    let hi = centre + SUN_MIN_FILTER_RADIUS;
    let first = floor(lo);
    var lit = 0.0;
    for (var y = 0u; y < 4u; y++) {
        for (var x = 0u; x < 4u; x++) {
            let sample = first + vec2(f32(x), f32(y));
            let weight = max(vec2(0.0), min(hi, sample + 1.0) - max(lo, sample));
            let texel = vec2<i32>(clamp(sample, vec2(0.0), vec2(size - 1.0)));
            let gap = caster_gap(layer, receiver, texel, size, phase);
            lit += weight.x * weight.y * select(0.0, 1.0, gap <= lower || gap > upper);
        }
    }
    let width = 2.0 * SUN_MIN_FILTER_RADIUS;
    return lit / (width * width);
}

fn filter_outline(layer: u32, receiver: CascadeReceiver, size: f32, radius: f32,
    lower: f32, upper: f32, phase: vec2<f32>) -> f32 {
    if radius <= SUN_MIN_FILTER_RADIUS {
        return contact_filter(layer, receiver, size, lower, upper, phase);
    }
    var visible = 0.0;
    // Nine bilinear taps spread the minimum footprint over neighbouring texels.
    for (var tap = 0u; tap < 9u; tap++) {
        let offset = select(SHADOW_DISK[(tap * 4u + 1u) % 32u], vec2(0.0), tap == 0u);
        let pixel = receiver.uv * size + offset * radius - phase - 0.5;
        let first = floor(pixel);
        let blend = fract(pixel);
        for (var corner = 0u; corner < 4u; corner++) {
            let step = vec2(f32(corner & 1u), f32(corner >> 1u));
            let texel = vec2<i32>(clamp(first + step, vec2(0.0), vec2(size - 1.0)));
            let weights = mix(1.0 - blend, blend, step);
            let gap = caster_gap(layer, receiver, texel, size, phase);
            visible += weights.x * weights.y * select(0.0, 1.0, gap <= lower || gap > upper);
        }
    }
    return visible / 9.0;
}

// The first layer owns the projection and blocker search. Integrate each
// shifted outline in that same coordinate system before averaging coverage.
const SUN_OUTLINE_PHASES = array(
    vec2(SUN_OUTLINE_X1, SUN_OUTLINE_Y1),
    vec2(SUN_OUTLINE_X2, SUN_OUTLINE_Y2),
    vec2(SUN_OUTLINE_X3, SUN_OUTLINE_Y3),
);

fn filtered_shadow(index: u32, receiver: CascadeReceiver, size: f32, radius: f32,
    lower: f32, upper: f32) -> f32 {
    var visible = filter_outline(index, receiver, size, radius, lower, upper, vec2(0.0));
    if shadows.cascade_metrics[index].x > SUN_OUTLINE_RADIUS { return visible; }
    let first = u32(shadows.profile.w) + 3u * index;
    for (var sample = 0u; sample < 3u; sample++) {
        visible += filter_outline(first + sample, receiver, size, radius, lower, upper,
            SUN_OUTLINE_PHASES[sample]);
    }
    return visible * 0.25;
}

fn penumbra_layer(first: u32, world_radius: f32) -> u32 {
    let last = u32(shadows.profile.x) - 1u;
    var layer = first;
    while layer < last && shadows.cascade_metrics[layer].y * SUN_SEARCH_TEXELS < world_radius {
        layer += 1u;
    }
    return layer;
}

fn cascade_visibility(index: u32, receiver: ShadowReceiver) -> f32 {
    let size = f32(textureDimensions(shadow_maps).x);
    let local = cascade_receiver(index, receiver, size);
    if any(local.uv < vec2(0.0)) || any(local.uv > vec2(1.0)) || local.depth < 0.0 || local.depth > 1.0 {
        return 1.0;
    }
    let maximum_gap = max(0.0, 1.0 - local.depth) / local.metres_to_depth;
    var gap_bound = maximum_gap;
    if shadows.height_support.y > 1e-6 {
        gap_bound = min(maximum_gap, max(0.0, shadows.height_support.x - receiver.position.y)
            / shadows.height_support.y);
    }
    let search_layer = penumbra_layer(index, gap_bound * SUN_TAN_RADIUS);
    let local_radius = min(SUN_SEARCH_TEXELS, gap_bound * SUN_TAN_RADIUS / local.texel_metres);
    let split = select(NO_BLOCKER, SUN_SEARCH_TEXELS * local.texel_metres / SUN_TAN_RADIUS,
        search_layer != index);
    let near = nearest_blocker(index, local, size, local_radius, 0.0, split);
    let near_radius = clamp(near.x * SUN_TAN_RADIUS / local.texel_metres, SUN_MIN_FILTER_RADIUS,
        SUN_SEARCH_TEXELS);
    let near_filtered = filtered_shadow(index, local, size,
        select(near_radius, SUN_MIN_FILTER_RADIUS, near.x == NO_BLOCKER), 0.0, split);
    if near_filtered <= 0.0 || search_layer == index { return near_filtered; }
    let remote = cascade_receiver(search_layer, receiver, size);
    let far = nearest_blocker(search_layer, remote, size,
        min(SUN_SEARCH_TEXELS, gap_bound * SUN_TAN_RADIUS / remote.texel_metres), split, NO_BLOCKER);
    let nearest = min(far.x, select(NO_BLOCKER, near.y, near.y > split));
    if nearest == NO_BLOCKER { return near_filtered; }
    let world_radius = nearest * SUN_TAN_RADIUS;
    let filter_layer = penumbra_layer(index, world_radius);
    let filtered = cascade_receiver(filter_layer, receiver, size);
    let radius = clamp(world_radius / filtered.texel_metres, SUN_MIN_FILTER_RADIUS, SUN_SEARCH_TEXELS);
    let far_filtered = filtered_shadow(filter_layer, filtered, size, radius, split, NO_BLOCKER);
    if far.x == NO_BLOCKER && near.y > split && far_filtered >= 1.0 {
        // A small caster omitted from the broader search keeps its soft core
        // where the fine map saw it in the receiver's own texel.
        return min(near_filtered, filtered_shadow(index, local, size, SUN_MIN_FILTER_RADIUS, split, NO_BLOCKER));
    }
    return min(near_filtered, far_filtered);
}

const HORIZON_STEPS: u32 = 96u;

// Fraction of the solar disk above a straight horizon whose elevation is
// offset from the disk centre by t solar radii (t>0: centre above horizon).
fn disk_above(t: f32) -> f32 {
    let c = clamp(t,-1.0,1.0);
    return 0.5+(c*sqrt(max(0.0,1.0-c*c))+asin(c))/3.14159265;
}

// Terrain-scale Sun visibility beyond the cascades: march the highest caster
// surface toward the Sun's azimuth and compare the steepest horizon with the
// solar disk. Steps grow geometrically from two texels to the map edge.
fn horizon_visibility(receiver: ShadowReceiver) -> f32 {
    let horizon = shadows.horizon;
    let l = light.direction_to_light.xyz;
    let flat = length(l.xz);
    if horizon.x <= 0.0 || flat < 1e-4 { return 1.0; }
    let azimuth = l.xz/flat;
    let sun_elevation = atan2(l.y,flat);
    let solar_radius = atan(SUN_TAN_RADIUS);
    // Horizons below the lower limb leave the disk whole; above the upper limb
    // they hide it. Both bounds end the march early without changing the result.
    let lower_limb = tan(sun_elevation-solar_radius);
    let upper_limb = tan(min(sun_elevation+solar_radius,1.5));
    let rise = shadows.height_support.x-receiver.position.y;
    let texel = horizon.w;
    let start = 2.0*texel;
    if lower_limb >= 0.0 && rise <= lower_limb*start { return 1.0; }
    let reach = 1.0/max(horizon.z,1e-20);
    let ratio = pow(reach/start,1.0/f32(HORIZON_STEPS-1u));
    // Nearest-texel heights can stand half a texel's rise above the true
    // surface on a slope; allow that much before a sample counts as horizon.
    let extent = vec2<f32>(f32(SHADOW_HEIGHT_SIZE));
    var steepest = -1e9;
    var distance = start;
    for (var i = 0u; i < HORIZON_STEPS; i++) {
        // A nonnegative slope threshold rises with distance. A negative one falls,
        // so even lower terrain can still rise above it farther along the ray.
        let threshold = max(lower_limb,steepest);
        if steepest >= upper_limb || (threshold >= 0.0 && rise <= threshold*distance) { break; }
        let q = receiver.position.xz+azimuth*distance;
        let clip = shadows.height_view_proj*vec4(q.x,0.0,q.y,1.0);
        let uv = vec2(clip.x*0.5+0.5,0.5-clip.y*0.5);
        if any(uv<vec2(0.0)) || any(uv>vec2(1.0)) { break; }
        let texel_index = clamp(vec2<i32>(uv*extent),vec2(0),vec2<i32>(extent)-1);
        let depth = textureLoad(shadow_maps,texel_index,i32(SHADOW_HEIGHT_LAYER),0);
        if depth > 0.0 {
            let height = (depth-horizon.y)/horizon.x;
            steepest = max(steepest,(height-receiver.position.y-0.5*texel*0.25)/distance);
        }
        distance *= ratio;
    }
    if steepest <= -1e8 { return 1.0; }
    return disk_above((sun_elevation-atan(steepest))/solar_radius);
}

fn mapped_sun_visibility(eye: vec3<f32>, receiver: ShadowReceiver) -> f32 {
    let choice = sun_cascade_choice(shadows.profile, shadows.cascade_metrics[0].x, distance(receiver.position, eye));
    if choice.fade >= 1.0 { return horizon_visibility(receiver); }
    var visibility = cascade_visibility(choice.layer, receiver);
    if choice.blend > 0.0 {
        visibility = mix(visibility, cascade_visibility(choice.layer + 1u, receiver), choice.blend);
    }
    if choice.fade > 0.0 {
        let horizon = horizon_visibility(receiver);
        visibility = select(visibility, min(visibility, horizon), light.direction_to_light.y < LOW_SUN_SINE);
        return mix(visibility, horizon, choice.fade);
    }
    return low_sun_terrain(visibility, receiver);
}

fn sun_visibility(_input: VertexOutput, receiver: ShadowReceiver) -> f32 {
    return mapped_sun_visibility(view.camera_xyz_time.xyz, receiver);
}

// Within a few degrees of the horizon, rays that clip a distant crest cross a
// wedge of terrain whose faces are nearly edge-on to the light; a depth map
// cannot hold that wedge. The height-map horizon can, so it bounds terrain
// occlusion there. It only darkens.
const LOW_SUN_SINE: f32 = 0.1;
fn low_sun_terrain(visibility: f32, receiver: ShadowReceiver) -> f32 {
    if visibility <= 0.0 || light.direction_to_light.y >= LOW_SUN_SINE { return visibility; }
    return min(visibility,horizon_visibility(receiver));
}

fn ambient_source(n: vec3<f32>) -> vec3<f32> {
    if environment_light.horizontal_irradiance.w==0.0 { return light.ambient_irradiance.rgb; }
    // Signed SH coefficients are retained. Only the final reconstructed
    // irradiance is repaired; approximation risk is recorded by normal probes.
    return max(evaluate_sky_diffuse(environment_light.sky_diffuse,n),vec3(0.0));
}

fn ambient_irradiance(n: vec3<f32>, world_position: vec3<f32>, pixel_position: vec4<f32>) -> vec3<f32> {
    return ambient_source(n);
}

fn shade(input: VertexOutput, receiver: ShadowReceiver, front_facing: bool, shadowed: bool) -> vec4<f32> {
    let material=materials[input.material];
    let base=textureSampleBias(base_textures,material_sampler,input.uv,i32(material.texture_layers.x),view.mip_bias);
    let orm=textureSampleBias(orm_textures,material_sampler,input.uv,i32(material.texture_layers.z),view.mip_bias);
    let emissive=textureSampleBias(emissive_textures,material_sampler,input.uv,i32(material.texture_layers.w),view.mip_bias);
    // Keep implicit-derivative texture operations before the divergent Sun searches.
