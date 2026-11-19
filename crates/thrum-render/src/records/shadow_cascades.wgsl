struct ShadowCascades {
    light_view_proj: array<mat4x4<f32>, 2>,
    split_overlap: vec4<f32>,
    bias: vec4<f32>,
    penumbra: vec4<f32>,
    height_support: vec4<f32>,
    // Top-down reverse-Z caster heights in shadow layer 2.
    height_view_proj: mat4x4<f32>,
    // Depth = x*height + y; uv per metre; texel metres. Zero x disables it.
    horizon: vec4<f32>,
    // Excluded upstream range in near units; the near clip plane in far depth.
    near_partition: vec4<f32>,
}
