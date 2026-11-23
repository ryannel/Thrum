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

struct SunLookup {
    uv: vec2<f32>,
    rows: u32,
}
fn spectral_sun_lookup(h: f32, mu: f32, lookup: texture_2d<f32>) -> SunLookup {
    let size=plane_size(textureDimensions(lookup),TRANS_PLANES);
    return SunLookup(trans_uv(h,mu,size),size.y);
}
fn spectral_sun_plane(
    query: SunLookup, bank: u32, lookup: texture_2d<f32>, linear: sampler,
) -> vec4<f32> {
    return textureSampleLevel(lookup,linear,plane_uv(query.uv,query.rows,bank+1u,TRANS_PLANES),0.0);
}
fn spectral_sun(
    h: f32, mu: f32, bank: u32, lookup: texture_2d<f32>, linear: sampler,
) -> vec4<f32> {
    if blocked(h,mu) { return vec4(0.0); }
    return spectral_sun_plane(spectral_sun_lookup(h,mu,lookup),bank,lookup,linear);
}
fn spectral_sun_pair(
    h: f32, mu: f32, lookup: texture_2d<f32>, linear: sampler,
) -> array<vec4<f32>,2> {
    if blocked(h,mu) { return array(vec4(0.0),vec4(0.0)); }
    let query=spectral_sun_lookup(h,mu,lookup);
    return array(spectral_sun_plane(query,0u,lookup,linear),spectral_sun_plane(query,1u,lookup,linear));
}
fn spectral_lut(
    uv: vec2<f32>, bank: u32, lookup: texture_2d<f32>, linear: sampler,
) -> vec4<f32> {
    let rows=plane_size(textureDimensions(lookup),SPECTRAL_PLANES).y;
    return textureSampleLevel(lookup,linear,plane_uv(uv,rows,bank,SPECTRAL_PLANES),0.0);
}
