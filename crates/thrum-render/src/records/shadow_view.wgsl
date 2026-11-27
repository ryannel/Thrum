struct ShadowView {
    light_view_proj: mat4x4<f32>,
    minimum_cast_width: vec4<f32>,
}

fn shadow_cast_width(instance: InstanceRecord) -> f32 {
    return instance.normal[0].w;
}
