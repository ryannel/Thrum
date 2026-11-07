// Fixed Earth profile, distances in metres, local +Y up at the camera.
struct Atmosphere {
    inverse_view_projection: mat4x4<f32>,
    camera_altitude: vec4<f32>,
    sun_lux: vec4<f32>,
    colour_pre_exposure: vec4<f32>,
    sky_mapping: vec4<f32>, // Horizon mu/elevation, sky/ground angular spans.
}
@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
const PI: f32 = 3.14159265359;
const R: f32 = 6371000.0;
const H: f32 = 100000.0;
// LUT RGB is encoded only at storage boundaries; transport stays per unit Sun.
// For this unchanged M<0.05 closure, upward L<=3.582+0.05, so GroundE<=pi*3.632
// and rho=0.1 gives terminal L<=0.1/pi+0.1*3.632. Thus encoded GroundE<11685
// and full Sky<4125 fit fp16. These fixed-profile envelopes do not certify the
// isotropic multiple-scattering/ground-feedback approximation; recheck on change.
// Aerial RGB uses the same Sky gain: L<3.632 encodes below 3720. T stays unscaled.
const MULTIPLE_LUT_GAIN: f32 = 65536.0;
const GROUND_IRRADIANCE_GAIN: f32 = 1024.0;
const GROUND_SUN_ENTRIES: u32 = 128u;
const GROUND_MU_SAMPLES: u32 = 16u;
const GROUND_AZIMUTH_SAMPLES: u32 = 16u;
fn ground_sun_mu(entry: u32) -> f32 {
    if entry==0u { return -1.0; }
    if entry==GROUND_SUN_ENTRIES-1u { return 1.0; }
    let t=2.0*f32(entry)/f32(GROUND_SUN_ENTRIES-1u)-1.0;
    return sin(sign(t)*t*t*PI*0.5);
}
fn ground_irradiance_uv(mu: f32) -> vec2<f32> {
    let elevation=asin(clamp(mu,-1.0,1.0));
    let q=sign(elevation)*sqrt(abs(elevation)/(PI*0.5));
    let unit=(q+1.0)*0.5;
    return vec2((0.5+f32(GROUND_SUN_ENTRIES-1u)*unit)/f32(GROUND_SUN_ENTRIES),0.5);
}
// Uniform midpoint mu/azimuth quadrature on the upward horizontal hemisphere.
// The weight already contains cos(theta): sum weights=pi for constant radiance.
fn ground_hemisphere_sample(index: u32, mu_samples: u32, azimuth_samples: u32) -> vec4<f32> {
    let mu=(f32(index/azimuth_samples)+0.5)/f32(mu_samples);
    let phi=2.0*PI*(f32(index%azimuth_samples)+0.5)/f32(azimuth_samples);
    let radial=sqrt(max(0.0,1.0-mu*mu));
    return vec4(radial*cos(phi),mu,radial*sin(phi),mu*2.0*PI/f32(mu_samples*azimuth_samples));
}
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
fn safe_segment(source: vec3<f32>, ext: vec3<f32>, dt: f32) -> vec3<f32> {
    return source*(vec3(1.0)-exp(-ext*dt))/max(ext,vec3(1e-12));
}
// Two disjoint 54-row angular-square halves. The two horizon rows store
// distinct one-sided limits at the same direction, at texel centres 53.5/108
// and 54.5/108. Lookups choose the sky side at exact tangency; neither half's
// filtering footprint crosses the discontinuity. Keep this fixed profile in
// step with gpu::EXTENTS[2].
const SKY_HALF_ROWS: u32 = 54u;
const SKY_ROWS: f32 = 108.0;
const SKY_HALF_INTERVALS: f32 = 53.0;
fn sun_azimuth(sun: vec3<f32>) -> f32 {
    if dot(sun.xz,sun.xz)<=1e-10 { return 0.0; }
    return atan2(sun.z,sun.x);
}
fn sky_angles(uv: vec2<f32>) -> vec2<f32> {
    let mapping=atmosphere.sky_mapping;
    var elevation: f32;
    if uv.y<0.5 {
        let q=clamp((53.5-uv.y*SKY_ROWS)/SKY_HALF_INTERVALS,0.0,1.0);
        elevation=mapping.y-mapping.w*q*q;
    } else {
        let q=clamp((uv.y*SKY_ROWS-54.5)/SKY_HALF_INTERVALS,0.0,1.0);
        elevation=mapping.y+mapping.z*q*q;
    }
    return vec2((uv.x*2.0-1.0)*PI,elevation);
}
fn sky_uv(direction: vec3<f32>, sun: vec3<f32>) -> vec2<f32> {
    var azimuth=0.0;
    if dot(direction.xz,direction.xz)>1e-10 {
        let relative=atan2(direction.z,direction.x)-sun_azimuth(sun);
        azimuth=atan2(sin(relative),cos(relative));
    }
    let mapping=atmosphere.sky_mapping;
    let elevation=asin(clamp(direction.y,-1.0,1.0));
    var vertical: f32;
    if direction.y>=mapping.x {
        if direction.y==mapping.x { vertical=54.5/SKY_ROWS; }
        else if direction.y>=1.0 { vertical=107.5/SKY_ROWS; }
        else {
            let q=sqrt(clamp((elevation-mapping.y)/mapping.z,0.0,1.0));
            vertical=(54.5+SKY_HALF_INTERVALS*q)/SKY_ROWS;
        }
    } else {
        if direction.y<=-1.0 { vertical=0.5/SKY_ROWS; }
        else {
            let q=sqrt(clamp((mapping.y-elevation)/mapping.w,0.0,1.0));
            vertical=(53.5-SKY_HALF_INTERVALS*q)/SKY_ROWS;
        }
    }
    return vec2(azimuth/(2.0*PI)+0.5,vertical);
}
struct SkyViewRay {
    direction: vec3<f32>,
    distance: f32,
    terminal_ground: u32,
}
fn sky_view_ray(uv: vec2<f32>, row: u32) -> SkyViewRay {
    let angles=sky_angles(uv);
    let azimuth=angles.x+sun_azimuth(atmosphere.sun_lux.xyz);
    let h=atmosphere.camera_altitude.w;
    let ground=row<SKY_HALF_ROWS;
    let horizon=row==SKY_HALF_ROWS-1u || row==SKY_HALF_ROWS;
    var mu=sin(angles.y);
    if row==0u { mu=-1.0; }
    else if row==107u { mu=1.0; }
    else if horizon { mu=atmosphere.sky_mapping.x; }
    let radial=sqrt(max(0.0,1.0-mu*mu));
    let direction=vec3(radial*cos(azimuth),mu,radial*sin(azimuth));
    var distance=top_distance(h,mu);
    if ground {
        if horizon {
            // The ground-side tangent ends at the planet, even when a rounded
            // classifier would call it sky. At h=0 this valid segment is zero.
            distance=sqrt(max(0.0,h*(2.0*R+h)));
        } else {
            // This row's side is known from its construction. Do not classify
            // it again from a rounded direction cosine.
            let b=(R+h)*mu;
            let c=h*(2.0*R+h);
            distance=c/max(0.001,-b+sqrt(max(0.0,b*b-c)));
        }
    }
    return SkyViewRay(direction,distance,select(0u,1u,ground));
}
