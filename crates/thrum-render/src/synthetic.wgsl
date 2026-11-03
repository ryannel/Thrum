// ABI2 opaque directional forward diagnostic: Lambert + GGX/Smith/Schlick.
// Fixed orthographic projection, no normal maps, indirect light or shadows.
struct ViewRecord {
    view_proj: mat4x4<f32>,
    previous_view_proj: mat4x4<f32>,
    camera_xyz_time: vec4<f32>,
    pre_exposure_inv_exposure_jitter: vec4<f32>,
}
struct DirectionalSource {
    direction_to_light: vec4<f32>,
    irradiance: vec4<f32>,
    half_vector_fresnel: vec4<f32>,
}
@group(0) @binding(0) var<uniform> view: ViewRecord;
@group(1) @binding(0) var<uniform> light: DirectionalSource;
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
}
@vertex fn vertex(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>,6>(vec2(-1.0,-1.0),vec2(1.0,-1.0),vec2(-1.0,1.0),vec2(-1.0,1.0),vec2(1.0,-1.0),vec2(1.0,1.0));
    let normals = array<vec3<f32>,6>(vec3(0.0,0.0,-1.0),vec3(1.0,0.0,0.0),vec3(0.0,0.0,1.0),vec3(-1.0,0.0,0.0),vec3(0.0,1.0,0.0),vec3(0.0,-1.0,0.0));
    let tangents = array<vec3<f32>,6>(vec3(1.0,0.0,0.0),vec3(0.0,0.0,1.0),vec3(-1.0,0.0,0.0),vec3(0.0,0.0,-1.0),vec3(1.0,0.0,0.0),vec3(1.0,0.0,0.0));
    let face = vertex_index / 6u;
    let n = normals[face];
    let t = tangents[face];
    let p = positions[vertex_index % 6u];
    let local = n + t*p.x + cross(n,t)*p.y;
    let instance = instances[instance_index];
    var output: VertexOutput;
    output.position = view.view_proj * instance.current * vec4(local,1.0);
    output.normal = mat3x3(instance.normal0.xyz,instance.normal1.xyz,instance.normal2.xyz) * n;
    output.uv = p*0.5+vec2(0.5);
    output.material = instance.ids.y;
    return output;
}
fn direct_brdf(base: vec3<f32>, metalness: f32, roughness: f32, n: vec3<f32>) -> vec3<f32> {
    let l = light.direction_to_light.xyz;
    // Parallel receiver-to-camera rays for the fixed orthographic view.
    let v = vec3(0.0,0.0,-1.0);
    let ci = clamp(dot(n,l),0.0,1.0);
    let co = clamp(dot(n,v),0.0,1.0);
    if ci <= 0.0 || co <= 0.0 { return vec3(0.0); }
    // The fixed orthographic view makes H and (1-v.h)^5 source invariants;
    // preparation computes them once in f64, including near opposition.
    let h = light.half_vector_fresnel.xyz;
    let ch = clamp(dot(n,h),0.0,1.0);
    let alpha = max(0.045,roughness)*max(0.045,roughness);
    let a2 = alpha*alpha;
    let d_base = (1.0-ch*ch)+ch*ch*a2;
    let d = a2/(3.14159265359*d_base*d_base);
    let f0 = mix(vec3(0.04),base,metalness);
    let fresnel = f0+(vec3(1.0)-f0)*light.half_vector_fresnel.w;
    // Cancel Smith's ci*co and the outgoing cosine, then scale the remaining
    // ci/co pair. A grazing denominator cannot underflow into 0/0.
    let cosine_scale = max(ci,co);
    let ci_scaled = ci/cosine_scale;
    let co_scaled = co/cosine_scale;
    let smith_denominator = 2.0*(ci_scaled*sqrt(a2+(1.0-a2)*co*co)+co_scaled*sqrt(a2+(1.0-a2)*ci*ci));
    let specular_cosine = d*fresnel*ci_scaled/smith_denominator;
    let diffuse = (1.0-metalness)*(vec3(1.0)-fresnel)*base/3.14159265359;
    return (diffuse*ci+specular_cosine)*light.irradiance.rgb;
}
@fragment fn fragment(input:VertexOutput)->@location(0) vec4<f32> {
    let material=materials[input.material];
    let base=textureSample(base_textures,material_sampler,input.uv,i32(material.texture_layers.x));
    let orm=textureSample(orm_textures,material_sampler,input.uv,i32(material.texture_layers.z));
    let emissive=textureSample(emissive_textures,material_sampler,input.uv,i32(material.texture_layers.w));
    // AO affects indirect illumination only; this diagnostic has none.
    let colour = direct_brdf(material.base.rgb*base.rgb,material.surface.z*orm.b,material.surface.y*orm.g,normalize(input.normal))
        + material.emissive_cutoff.rgb*emissive.rgb;
    // Explicit bounded linear diagnostic target, before any V4 display transform.
    return vec4(clamp(colour,vec3(0.0),vec3(1.0)),1.0);
}
