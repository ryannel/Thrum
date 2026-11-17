@group(0) @binding(0) var<storage, read> heads: array<u32>;
@group(0) @binding(1) var<storage, read> indices: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<u32>;
@compute @workgroup_size(64)
fn copy_lists(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if i >= arrayLength(&output) { return; }
    let n = arrayLength(&heads);
    if i < n { output[i] = heads[i]; }
    else { output[i] = indices[i - n]; }
}
