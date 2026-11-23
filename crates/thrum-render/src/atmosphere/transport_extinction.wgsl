fn extinction(densities: vec3<f32>, bank: u32) -> vec4<f32> {
    return RAYLEIGH_BETA[bank]*densities.x+MIE_EXTINCTION*densities.y+OZONE_BETA[bank]*densities.z;
}
// Arbitrary RGB surface light has no known spectrum. Keep its transmission
// diagonal in the original three representative bands, with bounded T.
fn surface_extinction(densities: vec3<f32>) -> vec3<f32> {
    return vec3(5.802e-6,13.558e-6,33.100e-6)*densities.x
        + MIE_EXTINCTION*densities.y + vec3(0.650e-6,1.881e-6,0.085e-6)*densities.z;
}
