// Retain the exact transform of a submitted render sample in physical draw order.
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
