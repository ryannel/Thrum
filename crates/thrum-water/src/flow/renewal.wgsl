// Stagger the bounded renewals in world space to keep one loss of
// coherence from dimming a reach.
fn flow_renewal(point: vec2<f32>, seconds: f32, span: f32) -> vec4<f32> {
    let offset = dot(point, PHASE_GRADIENT) + PHASE_BEND_AMPLITUDE * sin(dot(point, PHASE_BEND));
    return cascade_renewal((fract(seconds / span) + fract(offset)) * span, span);
}

fn flow_phase_gradient(point: vec2<f32>) -> vec2<f32> {
    return PHASE_GRADIENT + (PHASE_BEND_AMPLITUDE * cos(dot(point, PHASE_BEND))) * PHASE_BEND;
}
