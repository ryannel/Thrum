// Spectral data stays positive through transport. Signed weights appear only
// at the RGB boundary. Desaturate out-of-gamut light at constant luminance.
fn project_spectrum(lo: vec4<f32>, hi: vec4<f32>) -> vec3<f32> {
    let rgb=vec3(
        dot(lo,RGB_RED[0])+dot(hi,RGB_RED[1]),
        dot(lo,RGB_GREEN[0])+dot(hi,RGB_GREEN[1]),
        dot(lo,RGB_BLUE[0])+dot(hi,RGB_BLUE[1]),
    );
    let low=min(rgb.r,min(rgb.g,rgb.b));
    if low>=0.0 { return rgb; }
    let y=max(0.0,dot(rgb,REC709_LUMA));
    return max(vec3(0.0),mix(vec3(y),rgb,y/(y-low)));
}

fn medium_densities(h: f32) -> vec3<f32> {
    return vec3(exp(-h/RAYLEIGH_SCALE_HEIGHT),exp(-h/MIE_SCALE_HEIGHT),max(0.0,1.0-abs(h-OZONE_CENTRE)/OZONE_HALF_WIDTH));
}
