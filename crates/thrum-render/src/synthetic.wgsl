// ABI2 opaque clustered forward diagnostic: Lambert + GGX/Smith/Schlick.
// Checked finite reverse-Z camera, no normal maps, indirect light or shadows.
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
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) material: u32,
    @location(2) normal: vec3<f32>,
    @location(3) world_position: vec3<f32>,
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
    let world = instance.current * vec4(local,1.0);
    output.position = view.view_proj * world;
    if instance.ids.w == 0u { output.position = vec4(2.0, 2.0, 2.0, 1.0); }
    output.world_position = world.xyz;
    output.normal = mat3x3(instance.normal0.xyz,instance.normal1.xyz,instance.normal2.xyz) * n;
    output.uv = p*0.5+vec2(0.5);
    output.material = instance.ids.y;
    return output;
}
fn direct_brdf(base: vec3<f32>, metalness: f32, roughness: f32, n: vec3<f32>,
               l: vec3<f32>, v: vec3<f32>, irradiance: vec3<f32>, h: vec3<f32>, fresnel_power: f32) -> vec3<f32> {
    let ci = clamp(dot(n,l),0.0,1.0);
    let co = clamp(dot(n,v),0.0,1.0);
    if ci <= 0.0 || co <= 0.0 { return vec3(0.0); }
    let ch = clamp(dot(n,h),0.0,1.0);
    let alpha = max(0.045,roughness)*max(0.045,roughness);
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
fn cascade_visibility(index: u32, world_position: vec3<f32>, n: vec3<f32>) -> f32 {
    let clip = shadows.light_view_proj[index] * vec4(world_position,1.0);
    let p = clip.xyz/clip.w;
    let uv = vec2(p.x*0.5+0.5,0.5-p.y*0.5);
    if any(uv < vec2(0.0)) || any(uv > vec2(1.0)) || p.z < 0.0 || p.z > 1.0 {
        return 1.0;
    }
    let normal_bias = 1.0-clamp(dot(n,light.direction_to_light.xyz),0.0,1.0);
    let base = select(shadows.bias.y,shadows.bias.x,index == 0u);
    let slope = select(shadows.bias.w,shadows.bias.z,index == 0u);
    let reference = p.z + base + slope*normal_bias;
    let texel = 1.0/vec2<f32>(textureDimensions(shadow_maps));
    var visible = 0.0;
    for (var i = 0u; i < 4u; i++) {
        let offset = vec2(f32(i & 1u)-0.5,f32(i >> 1u)-0.5)*texel;
        visible += textureSampleCompareLevel(shadow_maps,shadow_sampler,uv+offset,i32(index),reference);
    }
    return visible*0.25;
}

fn sun_visibility(input: VertexOutput, n: vec3<f32>) -> f32 {
    let near = view_aux.near_far.x;
    let far = view_aux.near_far.y;
    let depth = near*far/(near+input.position.z*(far-near));
    let split = shadows.split_overlap.x;
    let overlap = shadows.split_overlap.y;
    if depth < split-overlap { return cascade_visibility(0u,input.world_position,n); }
    if depth > split+overlap { return cascade_visibility(1u,input.world_position,n); }
    let first = cascade_visibility(0u,input.world_position,n);
    let second = cascade_visibility(1u,input.world_position,n);
    return mix(first,second,clamp((depth-split+overlap)/(2.0*overlap),0.0,1.0));
}

fn shade(input: VertexOutput, sun_factor: f32) -> vec4<f32> {
    let material=materials[input.material];
    let base=textureSample(base_textures,material_sampler,input.uv,i32(material.texture_layers.x));
    let orm=textureSample(orm_textures,material_sampler,input.uv,i32(material.texture_layers.z));
    let emissive=textureSample(emissive_textures,material_sampler,input.uv,i32(material.texture_layers.w));
    // AO affects the ambient source only; direct Sun and local lights retain
    // their independent visibility and incident irradiance.
    let surface_base = material.base.rgb*base.rgb;
    let metalness = material.surface.z*orm.b;
    let roughness = material.surface.y*orm.g;
    let n = normalize(input.normal);
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
    let colour = direct_brdf(surface_base,metalness,roughness,n,
            light.direction_to_light.xyz,v,light.irradiance.rgb*sun_factor,h,fresnel_power)
        + local_radiance(surface_base,metalness,roughness,n,v,input.world_position,input.position)
        + orm.r*(1.0-metalness)*surface_base*light.ambient_irradiance.rgb/3.14159265359
        + material.emissive_cutoff.rgb*emissive.rgb;
    // Keep scene radiance above one. Repair unsupported presentation arithmetic
    // before pre-exposure and bound only to the finite fp16 storage envelope.
    let finite = select(vec3(0.0), colour, colour >= vec3(0.0));
    let bounded = min(finite, vec3(3.402823e38));
    return vec4(clamp(bounded*view.pre_exposure_inv_exposure_jitter.x,vec3(0.0),vec3(65504.0)),1.0);
}
@fragment fn fragment(input:VertexOutput)->@location(0) vec4<f32> {
    return shade(input,1.0);
}
@fragment fn shadowed_fragment(input:VertexOutput)->@location(0) vec4<f32> {
    return shade(input,sun_visibility(input,normalize(input.normal)));
}
