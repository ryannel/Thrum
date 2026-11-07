@group(1) @binding(12) var<uniform> scene_exposure: vec4<f32>;
fn scene_storage_scale() -> f32 { return scene_exposure.x; }

// ABI2 opaque clustered forward diagnostic: Lambert + GGX/Smith/Schlick.
// Checked finite reverse-Z camera; prepared lighting selects indirect resources.
struct ViewRecord {
    view_proj: mat4x4<f32>,
    previous_view_proj: mat4x4<f32>,
    camera_xyz_time: vec4<f32>,
    pre_exposure_inv_exposure_jitter: vec4<f32>,
}
struct ViewAuxRecord {
    inverse_view_proj: mat4x4<f32>,
    viewport_inv_viewport: vec4<f32>,
    near_far: vec4<f32>,
    time: vec4<f32>,
    flags: vec4<u32>,
}
struct DirectionalSource {
    direction_to_light: vec4<f32>,
    irradiance: vec4<f32>,
    half_vector_fresnel: vec4<f32>,
    ambient_irradiance: vec4<f32>,
    sky_diffuse: array<vec4<f32>,9>,
}
@group(0) @binding(0) var<uniform> view: ViewRecord;
@group(0) @binding(1) var<uniform> view_aux: ViewAuxRecord;
@group(1) @binding(0) var<uniform> light: DirectionalSource;
struct LocalSource {
    position_radius: vec4<f32>,
    candela: vec4<f32>,
}
struct ClusterHead {
    offset: u32,
    count: u32,
}
@group(1) @binding(1) var<storage, read> local_lights: array<LocalSource>;
@group(1) @binding(2) var<storage, read> cluster_heads: array<ClusterHead>;
@group(1) @binding(3) var<storage, read> cluster_indices: array<u32>;
struct ShadowCascades {
    light_view_proj: array<mat4x4<f32>, 2>,
    split_overlap: vec4<f32>,
    bias: vec4<f32>,
    penumbra: vec4<f32>,
    height_support: vec4<f32>,
    // Top-down reverse-Z caster heights in shadow layer 2.
    height_view_proj: mat4x4<f32>,
    // Depth = x*height + y; uv per metre; texel metres. Zero x disables it.
    horizon: vec4<f32>,
    // Excluded upstream range in near units; the near clip plane in far depth.
    near_partition: vec4<f32>,
}
@group(0) @binding(2) var<uniform> shadows: ShadowCascades;
@group(0) @binding(3) var shadow_maps: texture_depth_2d_array;
struct MaterialRecord {
    base: vec4<f32>,
    emissive_cutoff: vec4<f32>,
    surface: vec4<f32>,
    flags: vec4<u32>,
    texture_layers: vec4<u32>,
}
struct InstanceRecord {
    current: mat4x4<f32>,
    previous: mat4x4<f32>,
    normal0: vec4<f32>,
    normal1: vec4<f32>,
    normal2: vec4<f32>,
    ids: vec4<u32>,
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
struct HistoryRecord {
    current: mat4x4<f32>,
    identity: vec4<u32>,
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
@group(3) @binding(1) var<storage, read> history: array<HistoryRecord>;
@group(3) @binding(2) var<uniform> serials: Serials;
struct VertexOutput {
    // Invariant, so the depth prepass rasterizes the depth the forward pass writes.
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) material: u32,
    @location(2) normal: vec3<f32>,
    @location(3) world_position: vec3<f32>,
    @location(4) previous_clip: vec4<f32>,
    @location(5) @interpolate(flat) history_valid: u32,
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
    let positions = array<vec2<f32>,4>(vec2(-1.0,-1.0),vec2(1.0,-1.0),vec2(-1.0,1.0),vec2(1.0,1.0));
    let normals = array<vec3<f32>,6>(vec3(0.0,0.0,-1.0),vec3(1.0,0.0,0.0),vec3(0.0,0.0,1.0),vec3(-1.0,0.0,0.0),vec3(0.0,1.0,0.0),vec3(0.0,-1.0,0.0));
    let tangents = array<vec3<f32>,6>(vec3(1.0,0.0,0.0),vec3(0.0,0.0,1.0),vec3(-1.0,0.0,0.0),vec3(0.0,0.0,-1.0),vec3(1.0,0.0,0.0),vec3(1.0,0.0,0.0));
    let face = vertex_index / 4u;
    let n = normals[face];
    let t = tangents[face];
    let p = positions[vertex_index % 4u];
    let local = n + t*p.x + cross(n,t)*p.y;
    let instance = instances[slot];
    var output: VertexOutput;
    var model = instance.current;
    model[3] = vec4(mix(instance.previous[3].xyz, instance.current[3].xyz, serials.presentation_alpha), instance.current[3].w);
    let world = model * vec4(local,1.0);
    output.position = view.view_proj * world;
    if instance.ids.w == 0u { output.position = vec4(2.0, 2.0, 2.0, 1.0); }
    output.world_position = world.xyz;
    output.normal = mat3x3(instance.normal0.xyz,instance.normal1.xyz,instance.normal2.xyz) * n;
    output.uv = p*0.5+vec2(0.5);
    output.material = instance.ids.y;
    let prior = history[slot];
    let valid = (serials.previous_lo != 0u || serials.previous_hi != 0u)
        && prior.identity.x == instance.ids.x
        && prior.identity.y == instance.ids.w
        && prior.identity.z == serials.previous_lo
        && prior.identity.w == serials.previous_hi;
    output.history_valid = select(0u, 1u, valid);
    output.previous_clip = select(output.position,
        view.previous_view_proj * prior.current * vec4(local, 1.0), valid);
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
        u32(clamp(floor((world_position.x+1.0)*16.0),0.0,31.0)),
        u32(clamp(floor((world_position.y+1.0)*9.0),0.0,17.0)),
        u32(clamp(floor(world_position.z*16.0),0.0,15.0)));
    if view_aux.flags.x == 1u {
        let near = view_aux.near_far.x;
        let far = view_aux.near_far.y;
        let depth = near*far/(near+screen_position.z*(far-near));
        cluster = vec3<u32>(
            u32(clamp(floor(screen_position.x*view_aux.viewport_inv_viewport.z*32.0),0.0,31.0)),
            u32(clamp(floor((1.0-screen_position.y*view_aux.viewport_inv_viewport.w)*18.0),0.0,17.0)),
            u32(clamp(floor(log(depth/near)/log(far/near)*16.0),0.0,15.0)));
    }
    let head = cluster_heads[(cluster.z*18u+cluster.y)*32u+cluster.x];
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

const SUN_TAN_RADIUS: f32 = 0.004654244;
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
fn surface_tolerance(index:u32,gradient:vec2<f32>,delta:vec2<f32>) -> f32 {
    let uv_metres_span=select(shadows.penumbra.y,shadows.penumbra.x,index==0u)/SUN_TAN_RADIUS;
    let slope=min(length(gradient)*uv_metres_span,SURFACE_MAX_GRADIENT);
    let bend=min(SURFACE_ELEVATION_FRACTION/max(slope,1e-3),SURFACE_MAX_SLOPE);
    return bend*length(delta)*(1.0+slope*slope)/max(uv_metres_span,1e-20);
}

fn cascade_visibility(index: u32, receiver: ShadowReceiver) -> f32 {
    return cascade_window(index,receiver,vec3(0.0,2.0,2.0));
}

// The near map is fitted to its receivers only. A blocker whose finite-Sun
// support leaves that map is evaluated in the padded last cascade instead.
// Returns the spatial gap limit in near-map units and the absolute clip
// plane in last-map units. Two means that boundary needs no fallback.
fn near_gap_limit(receiver: ShadowReceiver) -> vec2<f32> {
    let clip = shadows.light_view_proj[0]*vec4(receiver.position,1.0);
    let p = clip.xyz/clip.w;
    let uv = vec2(p.x*0.5+0.5,0.5-p.y*0.5);
    let edge = min(min(uv.x,uv.y),min(1.0-uv.x,1.0-uv.y))-2.0/f32(textureDimensions(shadow_maps).x);
    let clip_gap = max(0.0,1.0-p.z);
    let excluded = shadows.near_partition.x;
    var search_gap = clip_gap+excluded;
    if shadows.height_support.y>1e-6 {
        let world_gap=max(0.0,shadows.height_support.x-receiver.position.y)/shadows.height_support.y;
        search_gap=min(search_gap,world_gap*shadows.height_support.z);
    }
    let support = edge/max(shadows.penumbra.x,1e-20);
    let spatial_limit = select(max(support,0.0),2.0,search_gap<=support);
    // The height estimate can be stale after a standalone instance upload.
    // It sizes the search but cannot revoke the last map's clip ownership.
    let far_clip = select(2.0,shadows.near_partition.y,excluded>0.0);
    return vec2(spatial_limit,far_clip);
}

fn near_visibility(receiver: ShadowReceiver) -> f32 {
    let limits = near_gap_limit(receiver);
    let near = cascade_window(0u,receiver,vec3(0.0,limits.x,2.0));
    if (limits.x>=2.0 && limits.y>=2.0) || near<=0.0 { return near; }
    // Depth units scale inversely with each cascade's depth span.
    let far_floor = select(limits.x*shadows.height_support.w/max(shadows.height_support.z,1e-20),
        2.0,limits.x>=2.0);
    return min(near,cascade_window(1u,receiver,vec3(far_floor,2.0,limits.y)));
}

// XY support bounds the tap's receiver-relative gap. The depth partition
// instead compares to the rasterizer's absolute clip plane.
fn blocker_in_window(depth: f32, gap: f32, window: vec3<f32>) -> bool {
    return gap<=window.y && (gap>window.x || depth>window.z);
}

fn cascade_window(index: u32, receiver: ShadowReceiver, window: vec3<f32>) -> f32 {
    let matrix = shadows.light_view_proj[index];
    let clip = matrix*vec4(receiver.position,1.0);
    let p = clip.xyz/clip.w;
    let uv = vec2(p.x*0.5+0.5,0.5-p.y*0.5);
    if any(uv < vec2(0.0)) || any(uv > vec2(1.0)) || p.z < 0.0 || p.z > 1.0 {
        return 1.0;
    }
    let normal_bias = 1.0-clamp(dot(receiver.normal,light.direction_to_light.xyz),0.0,1.0);
    let base = select(shadows.bias.y,shadows.bias.x,index == 0u);
    let slope = select(shadows.bias.w,shadows.bias.z,index == 0u);

    // The orthographic cascade maps a receiver triangle to a plane in light clip space.
    // Evaluate its depth at each texel centre so the four comparisons follow the surface.
    let dx = (matrix*vec4(receiver.dx,0.0)).xyz;
    let dy = (matrix*vec4(receiver.dy,0.0)).xyz;
    let plane = cross(dx,dy);
    let plane_extent = max(abs(plane.x),abs(plane.y));
    var depth_gradient = vec2(0.0);
    if abs(plane.z) > max(1e-20,plane_extent*1e-6) {
        depth_gradient = -plane.xy/plane.z*vec2(2.0,-2.0);
    }
    let extent = vec2<f32>(textureDimensions(shadow_maps));
    // A light texel spans more receiver depth at grazing incidence. Bound the
    // extra tolerance so a nearly edge-on triangle cannot erase its own shadow.
    let texel_depth = abs(depth_gradient)/extent;
    let footprint = 0.5*max(texel_depth.x,texel_depth.y);
    let reference = p.z + clamp(footprint,base,8.0*base) + slope*normal_bias;
    let scale = select(shadows.penumbra.y,shadows.penumbra.x,index==0u);
    var search_gap = max(0.0,1.0-p.z);
    if shadows.height_support.y>1e-6 {
        let world_gap=max(0.0,shadows.height_support.x-receiver.position.y)/shadows.height_support.y;
        let inverse_span=select(shadows.height_support.w,shadows.height_support.z,index==0u);
        search_gap=min(search_gap,world_gap*inverse_span);
    }
    let search_radius = search_gap*scale;
    var nearest_gap = 1.0;
    var blockers = 0.0;
    for(var i=0u;i<8u;i++) {
        let offset=select(SHADOW_DISK[i*4u],vec2(0.0),i==0u);
        let q=uv+offset*search_radius;
        if any(q<vec2(0.0)) || any(q>vec2(1.0)) { continue; }
        let texel=clamp(vec2<i32>(q*extent),vec2(0),vec2<i32>(extent)-1);
        let tap=(vec2<f32>(texel)+0.5)/extent;
        let depth=textureLoad(shadow_maps,texel,i32(index),0);
        let receiver_depth=reference+dot(depth_gradient,tap-uv);
        let plane_depth=p.z+dot(depth_gradient,tap-uv);
        let epsilon=select(shadows.penumbra.w,shadows.penumbra.z,index==0u)+4e-7;
        let same_plane=depth-plane_depth<=epsilon+surface_tolerance(index,depth_gradient,tap-uv);
        let gap=depth-plane_depth;
        if depth>p.z+epsilon && depth>receiver_depth && !same_plane && blocker_in_window(depth,gap,window) {
            nearest_gap=min(nearest_gap,max(0.0,depth-p.z));
            blockers+=1.0;
        }
    }
    if blockers==0.0 {
        return shadow_filter(index,uv,p.z,reference,depth_gradient,extent,uv,window);
    }
    // Eight search and sixteen disk taps match the 16/32 truth scores on the
    // solar crossing at about half the shadowed-fragment cost.
    // Prefer the nearest upstream blocker when depths overlap. Bound the
    // broad plane offset to half its gap; local texel bias remains separate.
    let physical_radius=nearest_gap*scale;
    let plane_radius=0.5*nearest_gap/max(length(depth_gradient),1e-12);
    let radius=min(physical_radius,plane_radius);
    if radius*extent.x<=0.5 {
        return shadow_filter(index,uv,p.z,reference,depth_gradient,extent,uv,window);
    }
    var visible=0.0;
    for(var i=0u;i<16u;i++) {
        let q=uv+SHADOW_DISK[i*2u+1u]*radius;
        visible+=shadow_filter(index,q,p.z,reference,depth_gradient,extent,uv,window);
    }
    return visible/16.0;
}

// Receiver-plane reconstruction follows each physical disk sample. The
// cascade bounds its extrapolation by the nearest blocker separation.
fn shadow_filter(index:u32,uv:vec2<f32>,receiver_depth:f32,reference:f32,
    gradient:vec2<f32>,extent:vec2<f32>,origin:vec2<f32>,window:vec3<f32>) -> f32 {
    if any(uv<vec2(0.0)) || any(uv>vec2(1.0)) { return 1.0; }
    let pixel = uv*extent-0.5;
    let first = floor(pixel);
    let blend = fract(pixel);
    var visible = 0.0;
    for (var i = 0u; i < 4u; i++) {
        let offset = vec2(f32(i & 1u),f32(i >> 1u));
        let tap = (clamp(first+offset,vec2(0.0),extent-1.0)+0.5)/extent;
        let weight = mix(1.0-blend,blend,offset);
        let tap_depth = max(0.0,reference+dot(gradient,tap-origin));
        let texel=vec2<i32>(clamp(first+offset,vec2(0.0),extent-1.0));
        let depth=textureLoad(shadow_maps,texel,i32(index),0);
        let plane_depth=receiver_depth+dot(gradient,tap-origin);
        let epsilon=select(shadows.penumbra.w,shadows.penumbra.z,index==0u)+4e-7;
        let same_plane=depth-plane_depth<=epsilon+surface_tolerance(index,gradient,tap-origin);
        let outside=!blocker_in_window(depth,depth-plane_depth,window);
        visible += weight.x*weight.y*select(0.0,1.0,tap_depth>=depth || same_plane || outside || depth<=receiver_depth+epsilon);
    }
    return visible;
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
    let extent = vec2<f32>(textureDimensions(shadow_maps));
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
        let depth = textureLoad(shadow_maps,texel_index,2,0);
        if depth > 0.0 {
            let height = (depth-horizon.y)/horizon.x;
            steepest = max(steepest,(height-receiver.position.y-0.5*texel*0.25)/distance);
        }
        distance *= ratio;
    }
    if steepest <= -1e8 { return 1.0; }
    return disk_above((sun_elevation-atan(steepest))/solar_radius);
}

fn sun_visibility(input: VertexOutput, receiver: ShadowReceiver) -> f32 {
    let near = view_aux.near_far.x;
    let far = view_aux.near_far.y;
    let depth = near*far/(near+input.position.z*(far-near));
    let split = shadows.split_overlap.x;
    let overlap = shadows.split_overlap.y;
    if depth < split-overlap { return low_sun_terrain(near_visibility(receiver),receiver); }
    if depth > split+overlap {
        let reach = shadows.split_overlap.z;
        if reach > 0.0 {
            if depth >= reach { return horizon_visibility(receiver); }
            let cascade = cascade_visibility(1u,receiver);
            let fade_start = shadows.split_overlap.w;
            if depth <= fade_start { return low_sun_terrain(cascade,receiver); }
            let horizon = horizon_visibility(receiver);
            let visibility = select(cascade,min(cascade,horizon),light.direction_to_light.y<LOW_SUN_SINE);
            return mix(visibility,horizon,smoothstep(fade_start,reach,depth));
        }
        return low_sun_terrain(cascade_visibility(1u,receiver),receiver);
    }
    let first = near_visibility(receiver);
    let second = cascade_visibility(1u,receiver);
    return low_sun_terrain(mix(first,second,clamp((depth-split+overlap)/(2.0*overlap),0.0,1.0)),receiver);
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

fn ambient_irradiance(n: vec3<f32>) -> vec3<f32> {
    if light.ambient_irradiance.w==0.0 { return light.ambient_irradiance.rgb; }
    // Signed SH coefficients are retained. Only the final reconstructed
    // irradiance is repaired; approximation risk is recorded by normal probes.
    return max(evaluate_sky_diffuse(light.sky_diffuse,n),vec3(0.0));
}

fn shade(input: VertexOutput, sun_factor: f32, front_facing: bool) -> vec4<f32> {
    let material=materials[input.material];
    let base=textureSample(base_textures,material_sampler,input.uv,i32(material.texture_layers.x));
    let orm=textureSample(orm_textures,material_sampler,input.uv,i32(material.texture_layers.z));
    let emissive=textureSample(emissive_textures,material_sampler,input.uv,i32(material.texture_layers.w));
    // AO affects the ambient source only; direct Sun and local lights retain
    // their independent visibility and incident irradiance.
    let surface_base = material.base.rgb*base.rgb;
    let metalness = material.surface.z*orm.b;
    let roughness = material.surface.y*orm.g;
    var n = normalize(input.normal);
    var v = vec3(0.0,0.0,-1.0);
    var h = light.half_vector_fresnel.xyz;
    var fresnel_power = light.half_vector_fresnel.w;
    if view_aux.flags.x == 1u {
        v = normalize(view.camera_xyz_time.xyz-input.world_position);
        let hv = light.direction_to_light.xyz+v;
        h = v;
        if dot(hv,hv)>0.000001 { h = normalize(hv); }
        fresnel_power = pow(clamp(1.0-dot(v,h),0.0,1.0),5.0);
    }
    // Smooth interpolation can put a shading normal behind V on a geometrically
    // front-facing triangle. Lift it just onto V's visible side, so direct,
    // local and environment lighting share a valid normal. Reflecting it instead
    // flips grazing slopes that face the Sun away from a camera looking into it.
    // Preserve real raster backfaces and the authored normal used above for
    // geometric shadow bias.
    if front_facing {
        let behind = dot(n,v)-0.01;
        if behind < 0.0 { n = normalize(n-behind*v); }
    }
    let colour = direct_brdf(surface_base,metalness,roughness,n,
            light.direction_to_light.xyz,v,sun_irradiance(input.world_position)*sun_factor,h,fresnel_power)
        + local_radiance(surface_base,metalness,roughness,n,v,input.world_position,input.position)
        + indirect_radiance(surface_base,metalness,roughness,n,v,orm.r)
        + material.emissive_cutoff.rgb*emissive.rgb;
    // Keep scene radiance above one. Repair unsupported presentation arithmetic
    // before pre-exposure and bound only to the finite fp16 storage envelope.
    let finite = select(vec3(0.0), colour, colour >= vec3(0.0));
    let bounded = min(finite, vec3(3.402823e38));
    let aerial = aerial_colour(bounded, input.world_position, input.position);
    return vec4(clamp(aerial*scene_storage_scale(),vec3(0.0),vec3(65504.0)),1.0);
}
struct FragmentOutput {
    @location(0) colour: vec4<f32>,
    @location(1) motion: vec2<f32>,
}
fn motion_vector(input: VertexOutput) -> vec2<f32> {
    if input.history_valid == 0u || input.previous_clip.w <= 0.0 {
        return vec2(0.0);
    }
    let previous_ndc = input.previous_clip.xy / input.previous_clip.w;
    let extent = view_aux.viewport_inv_viewport.xy;
    let previous_pixel = vec2(
        (previous_ndc.x * 0.5 + 0.5) * extent.x,
        (0.5 - previous_ndc.y * 0.5) * extent.y);
    return clamp(previous_pixel - input.position.xy, vec2(-65504.0), vec2(65504.0));
}
@fragment fn fragment(input:VertexOutput, @builtin(front_facing) front_facing: bool)->FragmentOutput {
    return FragmentOutput(shade(input,1.0,front_facing), motion_vector(input));
}
@fragment fn shadowed_fragment(input:VertexOutput, @builtin(front_facing) front_facing: bool)->FragmentOutput {
    let receiver = ShadowReceiver(input.world_position,dpdx(input.world_position),
        dpdy(input.world_position),normalize(input.normal));
    return FragmentOutput(shade(input,sun_visibility(input,receiver),front_facing), motion_vector(input));
}
