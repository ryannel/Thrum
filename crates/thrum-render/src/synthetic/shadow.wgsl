// Depth-only synthetic cube caster. Uses the same instance ABI and cube face
// construction as the forward diagnostic until a cooked mesh stream replaces it.
struct ShadowView { light_view_proj: mat4x4<f32> }
struct InstanceRecord {
    current: mat4x4<f32>,
    previous: mat4x4<f32>,
    normal0: vec4<f32>,
    normal1: vec4<f32>,
    normal2: vec4<f32>,
    ids: vec4<u32>,
}
@group(0) @binding(0) var<uniform> shadow_view: ShadowView;
@group(1) @binding(0) var<storage, read> instances: array<InstanceRecord>;
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
@group(1) @binding(2) var<uniform> serials: Serials;

@vertex fn vertex(@builtin(vertex_index) vertex_index: u32,
                  @builtin(instance_index) instance_index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>,4>(vec2(-1.0,-1.0),vec2(1.0,-1.0),vec2(-1.0,1.0),vec2(1.0,1.0));
    let normals = array<vec3<f32>,6>(vec3(0.0,0.0,-1.0),vec3(1.0,0.0,0.0),vec3(0.0,0.0,1.0),vec3(-1.0,0.0,0.0),vec3(0.0,1.0,0.0),vec3(0.0,-1.0,0.0));
    let tangents = array<vec3<f32>,6>(vec3(1.0,0.0,0.0),vec3(0.0,0.0,1.0),vec3(-1.0,0.0,0.0),vec3(0.0,0.0,-1.0),vec3(1.0,0.0,0.0),vec3(1.0,0.0,0.0));
    let face = vertex_index / 4u;
    let n = normals[face];
    let t = tangents[face];
    let p = positions[vertex_index % 4u];
    let local = n + t*p.x + cross(n,t)*p.y;
    let instance = instances[instance_index];
    if instance.ids.w == 0u { return vec4(2.0, 2.0, 2.0, 1.0); }
    var model = instance.current;
    model[3] = vec4(mix(instance.previous[3].xyz, instance.current[3].xyz, serials.presentation_alpha), instance.current[3].w);
    var clip = shadow_view.light_view_proj * model * vec4(local, 1.0);
    // Keep the current transform even when a caster leaves the prepared depth range.
    // Flatten its excess depth onto the nearest cascade plane.
    clip.z = clamp(clip.z, 0.0, clip.w);
    return clip;
}
