fn scattering_step(
    densities: vec3<f32>, phase: vec2<f32>, dt: f32, bank: u32,
    direct: vec4<f32>, multiple: vec4<f32>,
) -> Segment {
    let rayleigh=RAYLEIGH_BETA[bank]*densities.x;
    let mie=aerosol.scattering[bank]*densities.y;
    let single=(rayleigh*phase.x+mie*phase.y)*direct;
    return integrate_segment(single+(rayleigh+mie)*multiple,extinction(densities,bank),dt);
}
