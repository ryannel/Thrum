// Diagnostic shader, ABI revision 2. No PBR, clustered lighting or shadow claim.
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
}
@vertex fn vertex(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>,6>(vec2(-1.0,-1.0),vec2(1.0,-1.0),vec2(-1.0,1.0),vec2(-1.0,1.0),vec2(1.0,-1.0),vec2(1.0,1.0));
    let instance=instances[instance_index];
    let p=positions[vertex_index];
    var output: VertexOutput;
    output.position=instance.current*vec4(p,0.5,1.0);
    output.uv=p*0.5+vec2(0.5);
    output.material=instance.ids.y;
    return output;
}
@fragment fn fragment(input:VertexOutput)->@location(0) vec4<f32> {
    let material=materials[input.material];
    let base=textureSample(base_textures,material_sampler,input.uv,i32(material.texture_layers.x));
    let orm=textureSample(orm_textures,material_sampler,input.uv,i32(material.texture_layers.z));
    let emissive=textureSample(emissive_textures,material_sampler,input.uv,i32(material.texture_layers.w));
    // Multiplicative base/default and emission semantics, unlit diagnostic only.
    // ORM is read so the typed data-map binding participates in device validation.
    let ao=1.0+material.surface.x*(orm.r-1.0);
    return vec4(material.base.rgb*base.rgb*ao+material.emissive_cutoff.rgb*emissive.rgb,material.base.a*base.a);
}
