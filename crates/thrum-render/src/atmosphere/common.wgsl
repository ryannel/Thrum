// Fixed Earth profile, distances in metres, local +Y up at the camera.
struct Atmosphere {
    inverse_view_projection: mat4x4<f32>,
    camera_altitude: vec4<f32>,
    sun_lux: vec4<f32>,
    colour_pre_exposure: vec4<f32>,
    viewport: vec4<f32>,
}
@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
const PI: f32 = 3.14159265359;
const R: f32 = 6371000.0;
const H: f32 = 100000.0;
fn altitude(p: vec3<f32>) -> f32 {
    let n = dot(p,p)+2.0*R*p.y;
    return max(0.0,n/(sqrt(max(0.0,R*R+n))+R));
}
fn extinction(h: f32) -> vec3<f32> {
    return vec3(5.802e-6,13.558e-6,33.100e-6)*exp(-h/8000.0)
        +vec3(4.4e-6)*exp(-h/1200.0)
        +vec3(0.650e-6,1.881e-6,0.085e-6)*max(0.0,1.0-abs(h-25000.0)/15000.0);
}
fn scattering(h: f32) -> vec3<f32> {
    return vec3(5.802e-6,13.558e-6,33.100e-6)*exp(-h/8000.0)
        +vec3(3.996e-6)*exp(-h/1200.0);
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
fn ground_distance(h: f32, mu: f32) -> f32 {
    if !blocked(h,mu) { return -1.0; }
    let b = (R+h)*mu;
    let c = h*(2.0*R+h);
    return c/max(0.001,-b+sqrt(max(0.0,b*b-c)));
}
fn trans_uv(h: f32, mu: f32) -> vec2<f32> {
    let cap = sqrt(H*(2.0*R+H));
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
fn safe_segment(source: vec3<f32>, ext: vec3<f32>, dt: f32) -> vec3<f32> {
    return source*(vec3(1.0)-exp(-ext*dt))/max(ext,vec3(1e-12));
}
// The sky LUT concentrates vertical samples at the horizon, independent of
// view rotation. Its horizontal coordinate is relative azimuth toward the Sun.
fn sky_angles(uv: vec2<f32>) -> vec2<f32> {
    let a = uv.y*2.0-1.0;
    return vec2((uv.x*2.0-1.0)*PI,sign(a)*a*a*PI*0.5);
}
fn sky_uv(direction: vec3<f32>, sun: vec3<f32>) -> vec2<f32> {
    var azimuth = 0.0;
    if dot(direction.xz,direction.xz)>1e-10 {
        azimuth = atan2(direction.z,direction.x)-atan2(sun.z,sun.x);
        azimuth = atan2(sin(azimuth),cos(azimuth));
    }
    let elevation = asin(clamp(direction.y,-1.0,1.0))/(PI*0.5);
    return vec2(azimuth/(2.0*PI)+0.5,0.5+0.5*sign(elevation)*sqrt(abs(elevation)));
}
