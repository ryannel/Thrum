// One invocation owns one cell. CPU supplies conservative projected source
// bounds; a source outside the cell cannot affect any receiver in that cell.
struct Bounds {
    lo: vec4<u32>,
    hi: vec4<u32>,
}
struct Head {
    offset: u32,
    count: u32,
}
@group(0) @binding(0) var<storage, read> source_bounds: array<Bounds>;
@group(0) @binding(1) var<storage, read_write> heads: array<Head>;
@group(0) @binding(2) var<storage, read_write> indices: array<u32>;

@compute @workgroup_size(64)
fn build(@builtin(global_invocation_id) id: vec3<u32>) {
    let cell = id.x;
    if cell >= 32u * 18u * 16u { return; }
    let xyz = vec3<u32>(cell % 32u, (cell / 32u) % 18u, cell / (32u * 18u));
    let stride = arrayLength(&source_bounds);
    let offset = cell * stride;
    var count = 0u;
    for (var source = 0u; source < stride; source++) {
        let bounds = source_bounds[source];
        if all(xyz >= bounds.lo.xyz) && all(xyz <= bounds.hi.xyz) {
            indices[offset + count] = source;
            count += 1u;
        }
    }
    for (var unused = count; unused < stride; unused++) {
        indices[offset + unused] = 0u;
    }
    heads[cell] = Head(offset, count);
}
