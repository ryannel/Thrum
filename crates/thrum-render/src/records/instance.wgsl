struct InstanceRecord {
    current: mat4x4<f32>,
    previous: mat4x4<f32>,
    normal: array<vec4<f32>, 3>,
    ids: vec4<u32>,
}
