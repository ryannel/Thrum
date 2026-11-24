// The host transfers these words to and from staging to bracket the pixel copy.
@group(0) @binding(0) var<storage, read> input: u32;
@group(0) @binding(1) var<storage, read_write> sink: u32;
@compute @workgroup_size(1)
fn main() {
    sink = input;
}
