struct SurfVertexSample { current: vec4<f32>, previous: vec4<f32>, }
@group(3) @binding(13) var<storage, read> surf_cache: array<SurfVertexSample>;
fn surf_current(index: u32) -> vec4<f32> { return surf_cache[index].current; }
fn surf_previous(index: u32) -> vec4<f32> { return surf_cache[index].previous; }
