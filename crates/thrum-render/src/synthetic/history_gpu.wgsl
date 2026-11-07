// Retain the exact transform of a submitted render sample in physical draw order.
struct InstanceRecord {
    current: mat4x4<f32>,
    previous: mat4x4<f32>,
    normal0: vec4<f32>,
    normal1: vec4<f32>,
    normal2: vec4<f32>,
    ids: vec4<u32>,
}
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
@group(0) @binding(0) var<storage, read> instances: array<InstanceRecord>;
@group(0) @binding(1) var<storage, read_write> history: array<HistoryRecord>;
@group(0) @binding(2) var<uniform> serials: Serials;

@compute @workgroup_size(64)
fn publish(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let physical = invocation.x;
    if physical >= arrayLength(&instances) { return; }
    let instance = instances[physical];
    if instance.ids.w == 0u { return; }
    var model = instance.current;
    model[3] = vec4(mix(instance.previous[3].xyz, instance.current[3].xyz, serials.presentation_alpha), instance.current[3].w);
    history[physical].current = model;
    history[physical].identity = vec4(
        instance.ids.x, instance.ids.w, serials.current_lo, serials.current_hi);
}
