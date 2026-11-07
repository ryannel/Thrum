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
}
@group(0) @binding(2) var<uniform> shadows: ShadowCascades;
@group(0) @binding(3) var shadow_maps: texture_depth_2d_array;
@group(0) @binding(4) var shadow_sampler: sampler_comparison;
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
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) material: u32,
    @location(2) normal: vec3<f32>,
    @location(3) world_position: vec3<f32>,
    @location(4) previous_clip: vec4<f32>,
    @location(5) @interpolate(flat) history_valid: u32,
}
@vertex fn vertex(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>,4>(vec2(-1.0,-1.0),vec2(1.0,-1.0),vec2(-1.0,1.0),vec2(1.0,1.0));
    let normals = array<vec3<f32>,6>(vec3(0.0,0.0,-1.0),vec3(1.0,0.0,0.0),vec3(0.0,0.0,1.0),vec3(-1.0,0.0,0.0),vec3(0.0,1.0,0.0),vec3(0.0,-1.0,0.0));
    let tangents = array<vec3<f32>,6>(vec3(1.0,0.0,0.0),vec3(0.0,0.0,1.0),vec3(-1.0,0.0,0.0),vec3(0.0,0.0,-1.0),vec3(1.0,0.0,0.0),vec3(1.0,0.0,0.0));
    let face = vertex_index / 4u;
    let n = normals[face];
    let t = tangents[face];
    let p = positions[vertex_index % 4u];
    let local = n + t*p.x + cross(n,t)*p.y;
    let instance = instances[instance_index];
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
    let prior = history[instance_index];
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

fn cascade_visibility(index: u32, receiver: ShadowReceiver) -> f32 {
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
    let pixel = uv*extent-0.5;
    let first = floor(pixel);
    let blend = fract(pixel);
    var visible = 0.0;
    for (var i = 0u; i < 4u; i++) {
        let offset = vec2(f32(i & 1u),f32(i >> 1u));
        let tap = (clamp(first+offset,vec2(0.0),extent-1.0)+0.5)/extent;
        let weight = mix(1.0-blend,blend,offset);
        let tap_depth = reference+dot(depth_gradient,tap-uv);
        visible += weight.x*weight.y*textureSampleCompareLevel(shadow_maps,shadow_sampler,tap,i32(index),tap_depth);
    }
    return visible;
}

fn sun_visibility(input: VertexOutput, receiver: ShadowReceiver) -> f32 {
    let near = view_aux.near_far.x;
    let far = view_aux.near_far.y;
    let depth = near*far/(near+input.position.z*(far-near));
    let split = shadows.split_overlap.x;
    let overlap = shadows.split_overlap.y;
    if depth < split-overlap { return cascade_visibility(0u,receiver); }
    if depth > split+overlap {
        let reach = shadows.split_overlap.z;
        if reach > 0.0 {
            if depth >= reach { return 1.0; }
            let visibility = cascade_visibility(1u,receiver);
            return mix(visibility,1.0,smoothstep(shadows.split_overlap.w,reach,depth));
        }
        return cascade_visibility(1u,receiver);
    }
    let first = cascade_visibility(0u,receiver);
    let second = cascade_visibility(1u,receiver);
    return mix(first,second,clamp((depth-split+overlap)/(2.0*overlap),0.0,1.0));
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
    // front-facing triangle. Reflect it across V's tangent plane once, so direct,
    // local and environment lighting share a valid normal. Preserve real raster
    // backfaces and the authored normal used above for geometric shadow bias.
    if front_facing { n -= 2.0*min(dot(n,v),0.0)*v; }
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
    return vec4(clamp(aerial*view.pre_exposure_inv_exposure_jitter.x,vec3(0.0),vec3(65504.0)),1.0);
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
