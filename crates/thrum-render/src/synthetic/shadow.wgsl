// Depth-only synthetic caster. Uses the same instance ABI and arena vertex
// fetch as the forward pass: the draw's index range and base vertex select the
// mesh. Legacy scenes draw the arena cube over physical rows; scenes that
// select meshes fetch each bucket's members from the bucket route's list.
struct ShadowView {
    light_view_proj: mat4x4<f32>,
    clip_upstream: vec4<u32>,
}
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
// `MeshVertex` in `mesh_arena.rs`: position and normal x, normal yz and uv,
// tangent. The caster reads only the position.
struct MeshVertex {
    a: vec4<f32>,
    b: vec4<f32>,
    c: vec4<f32>,
}
@group(1) @binding(3) var<storage, read> mesh_vertices: array<MeshVertex>;
@group(2) @binding(0) var<storage, read> members: array<u32>;

@vertex fn vertex(@builtin(vertex_index) vertex_index: u32,
                  @builtin(instance_index) instance_index: u32) -> @builtin(position) vec4<f32> {
    return caster_at(vertex_index, instance_index);
}
@vertex fn member_vertex(@builtin(vertex_index) vertex_index: u32,
                         @builtin(instance_index) instance_index: u32) -> @builtin(position) vec4<f32> {
    return caster_at(vertex_index, members[instance_index]);
}
fn caster_at(vertex_index: u32, slot: u32) -> vec4<f32> {
    let local = mesh_vertices[vertex_index].a.xyz;
    let instance = instances[slot];
    if instance.ids.w == 0u { return vec4(2.0, 2.0, 2.0, 1.0); }
    var model = instance.current;
    model[3] = vec4(mix(instance.previous[3].xyz, instance.current[3].xyz, serials.presentation_alpha), instance.current[3].w);
    var clip = shadow_view.light_view_proj * model * vec4(local, 1.0);
    // Clip intentionally excluded upstream geometry. Unexpected depth
    // outside the fitted support retains the existing clamp fallback.
    if shadow_view.clip_upstream.x==0u {
        clip.z = clamp(clip.z, 0.0, clip.w);
    } else {
        clip.z = max(clip.z, 0.0);
    }
    return clip;
}
