// Shared fixed-Earth geometry and transmittance chart. Distances are metres.
const PI: f32 = 3.14159265359;
const R: f32 = 6371000.0;
const H: f32 = 100000.0;
fn altitude(p: vec3<f32>) -> f32 {
    let n = dot(p,p)+2.0*R*p.y;
    return max(0.0,n/(sqrt(max(0.0,R*R+n))+R));
}
fn blocked(h: f32, mu: f32) -> bool {
    return mu < -sqrt(max(0.0,h*(2.0*R+h)))/(R+h);
}
fn top_distance(h: f32, mu: f32) -> f32 {
    let b = (R+h)*mu;
    let c = (H-h)*(2.0*R+H+h);
    let q = sqrt(max(0.0,b*b+c));
    if mu>=0.0 { return c/max(0.001,q+b); }
    return q-b;
}
fn trans_uv(height: f32, mu: f32, size: vec2<u32>) -> vec2<f32> {
    let cap = sqrt(H*(2.0*R+H));
    // Below the first physical producer row, both coordinates must describe
    // that same row. Clamping only v previously kept true-height u, sampling
    // a different ray. The near-ground GPU/CPU regression bounds the remaining
    // first-row altitude approximation without changing the producer grid.
    let first_rho=cap*0.5/f32(size.y);
    let first_h=first_rho*first_rho/(sqrt(R*R+first_rho*first_rho)+R);
    let h=max(height,first_h);
    let rho = sqrt(max(0.0,h*(2.0*R+h)));
    let d = top_distance(h,mu);
    return clamp(vec2((d-(H-h))/max(0.001,rho+cap-(H-h)),rho/cap),vec2(0.0),vec2(1.0));
}
fn trans_params(uv: vec2<f32>) -> vec2<f32> {
    let cap = sqrt(H*(2.0*R+H));
    let rho = cap*uv.y;
    let h = rho*rho/(sqrt(R*R+rho*rho)+R);
    let d = (H-h)+uv.x*(rho+cap-(H-h));
    let mu = ((H-h)*(2.0*R+H+h)-d*d)/max(0.001,2.0*(R+h)*d);
    return vec2(h,clamp(mu,-1.0,1.0));
}

// Vertical LUT planes share one binding. Clamp within each plane before
// offsetting so filtering never mixes two wavelength groups.
const TRANS_PLANES: u32 = 3u;
const SPECTRAL_PLANES: u32 = 2u;
fn plane_size(size: vec2<u32>, planes: u32) -> vec2<u32> {
    return vec2(size.x,size.y/planes);
}
fn plane_uv(uv: vec2<f32>, height: u32, plane: u32, planes: u32) -> vec2<f32> {
    let edge=0.5/f32(height);
    return vec2(uv.x,(clamp(uv.y,edge,1.0-edge)+f32(plane))/f32(planes));
}
fn sample_sun_transmittance(h: f32, mu: f32, lookup: texture_2d<f32>, linear: sampler) -> vec3<f32> {
    if blocked(h,mu) { return vec3(0.0); }
    let size=plane_size(textureDimensions(lookup),TRANS_PLANES);
    return textureSampleLevel(lookup,linear,plane_uv(trans_uv(h,mu,size),size.y,0u,TRANS_PLANES),0.0).rgb;
}
