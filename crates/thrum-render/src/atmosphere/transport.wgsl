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
    let y=max(0.0,dot(rgb,SPECTRUM_LUMA));
    return max(vec3(0.0),mix(vec3(y),rgb,y/(y-low)));
}

fn medium_densities(h: f32) -> vec3<f32> {
    return vec3(exp(-h/8000.0),exp(-h/1200.0),max(0.0,1.0-abs(h-25000.0)/15000.0));
}
fn extinction(densities: vec3<f32>, bank: u32) -> vec4<f32> {
    return RAYLEIGH_BETA[bank]*densities.x+MIE_EXTINCTION*densities.y+OZONE_BETA[bank]*densities.z;
}
// Arbitrary RGB surface light has no known spectrum. Keep its transmission
// diagonal in the original three representative bands, with bounded T.
fn surface_extinction(densities: vec3<f32>) -> vec3<f32> {
    return vec3(5.802e-6,13.558e-6,33.100e-6)*densities.x
        + MIE_EXTINCTION*densities.y + vec3(0.650e-6,1.881e-6,0.085e-6)*densities.z;
}

struct Segment {
    radiance: vec4<f32>,
    transmission: vec4<f32>,
}
fn integrate_segment(source: vec4<f32>, ext: vec4<f32>, dt: f32) -> Segment {
    let transmission=exp(-ext*dt);
    let radiance=source*(vec4(1.0)-transmission)/max(ext,vec4(1e-12));
    return Segment(radiance,transmission);
}
struct SpectralRay {
    lo: Segment,
    hi: Segment,
}
fn empty_ray() -> SpectralRay {
    let initial=Segment(vec4(0.0),vec4(1.0));
    return SpectralRay(initial,initial);
}
fn extend_ray(ray: Segment, step: Segment) -> Segment {
    return Segment(ray.radiance+ray.transmission*step.radiance,ray.transmission*step.transmission);
}

fn spectral_sun(
    h: f32, mu: f32, bank: u32, lookup: texture_2d<f32>, linear: sampler,
) -> vec4<f32> {
    if blocked(h,mu) { return vec4(0.0); }
    let size=plane_size(textureDimensions(lookup),TRANS_PLANES);
    let uv=plane_uv(trans_uv(h,mu,size),size.y,bank+1u,TRANS_PLANES);
    return textureSampleLevel(lookup,linear,uv,0.0);
}
fn spectral_lut(
    uv: vec2<f32>, bank: u32, lookup: texture_2d<f32>, linear: sampler,
) -> vec4<f32> {
    let rows=plane_size(textureDimensions(lookup),SPECTRAL_PLANES).y;
    return textureSampleLevel(lookup,linear,plane_uv(uv,rows,bank,SPECTRAL_PLANES),0.0);
}
fn scattering_step(
    densities: vec3<f32>, phase: vec2<f32>, dt: f32, bank: u32,
    direct: vec4<f32>, multiple: vec4<f32>,
) -> Segment {
    let rayleigh=RAYLEIGH_BETA[bank]*densities.x;
    let mie=3.996e-6*densities.y;
    let single=(rayleigh*phase.x+mie*phase.y)*direct;
    return integrate_segment(single+(rayleigh+mie)*multiple,extinction(densities,bank),dt);
}
fn lit_step(
    ray: SpectralRay, densities: vec3<f32>, h: f32, mu: f32, phase: vec2<f32>, dt: f32,
    transmittance: texture_2d<f32>, multiple: texture_2d<f32>, linear: sampler,
) -> SpectralRay {
    let uv=vec2(mu*0.5+0.5,h/H);
    var direct_lo=vec4(0.0);
    var direct_hi=vec4(0.0);
    if !blocked(h,mu) {
        let size=plane_size(textureDimensions(transmittance),TRANS_PLANES);
        let sun_uv=trans_uv(h,mu,size);
        direct_lo=textureSampleLevel(transmittance,linear,plane_uv(sun_uv,size.y,1u,TRANS_PLANES),0.0);
        direct_hi=textureSampleLevel(transmittance,linear,plane_uv(sun_uv,size.y,2u,TRANS_PLANES),0.0);
    }
    let lo=scattering_step(densities,phase,dt,0u,direct_lo,
        spectral_lut(uv,0u,multiple,linear)/MULTIPLE_LUT_GAIN);
    let hi=scattering_step(densities,phase,dt,1u,direct_hi,
        spectral_lut(uv,1u,multiple,linear)/MULTIPLE_LUT_GAIN);
    return SpectralRay(extend_ray(ray.lo,lo),extend_ray(ray.hi,hi));
}
