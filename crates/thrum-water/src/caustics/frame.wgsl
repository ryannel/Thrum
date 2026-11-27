struct CausticDrawFrame {
    window: vec4<f32>,
    sun: vec4<f32>,
}
@group(1) @binding(0) var<uniform> caustic_draw: CausticDrawFrame;
