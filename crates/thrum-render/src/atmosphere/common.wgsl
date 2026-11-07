// Fixed Earth profile, distances in metres, local +Y up at the camera.
struct Atmosphere {
    inverse_view_projection: mat4x4<f32>,
    camera_altitude: vec4<f32>,
    sun_lux: vec4<f32>,
    colour_pre_exposure: vec4<f32>,
    sky_mapping: vec4<f32>, // Horizon mu/elevation, sky/ground angular spans.
}
@group(0) @binding(0) var<uniform> atmosphere: Atmosphere;
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
struct Medium {
    rayleigh: vec3<f32>,
    mie: f32,
    extinction: vec3<f32>,
}
fn medium(h: f32) -> Medium {
    let rayleigh=vec3(5.802e-6,13.558e-6,33.100e-6)*exp(-h/8000.0);
    let mie_density=exp(-h/1200.0);
    let ozone=vec3(0.650e-6,1.881e-6,0.085e-6)*max(0.0,1.0-abs(h-25000.0)/15000.0);
    return Medium(rayleigh,3.996e-6*mie_density,rayleigh+vec3(4.4e-6*mie_density)+ozone);
}
fn scattering_phase(cosine: f32) -> vec2<f32> {
    let rayleigh=3.0*(1.0+cosine*cosine)/(16.0*PI);
    let g=0.8;
    let mie=(1.0-g*g)/(4.0*PI*pow(max(0.001,1.0+g*g-2.0*g*cosine),1.5));
    return vec2(rayleigh,mie);
}
fn local_sun_mu(p: vec3<f32>, sun: vec3<f32>) -> f32 {
    return dot(normalize(p+vec3(0.0,R,0.0)),sun);
}
fn ground_distance(h: f32, mu: f32) -> f32 {
    if !blocked(h,mu) { return -1.0; }
    let b = (R+h)*mu;
    let c = h*(2.0*R+h);
    return c/max(0.001,-b+sqrt(max(0.0,b*b-c)));
}
struct Segment {
    radiance: vec3<f32>,
    transmission: vec3<f32>,
}
fn integrate_segment(source: vec3<f32>, extinction: vec3<f32>, dt: f32) -> Segment {
    let transmission=exp(-extinction*dt);
    let radiance=source*(vec3(1.0)-transmission)/max(extinction,vec3(1e-12));
    return Segment(radiance,transmission);
}
fn lit_segment(
    p: vec3<f32>, sun: vec3<f32>, phase: vec2<f32>, dt: f32,
    transmittance: texture_2d<f32>, multiple_scattering: texture_2d<f32>, linear: sampler,
) -> Segment {
    let h=altitude(p);
    let air=medium(h);
    let mu=local_sun_mu(p,sun);
    let direct=sample_sun_transmittance(h,mu,transmittance,linear);
    let multiple=textureSampleLevel(multiple_scattering,linear,vec2(mu*0.5+0.5,h/H),0.0).rgb/MULTIPLE_LUT_GAIN;
    let single=(air.rayleigh*phase.x+air.mie*phase.y)*direct;
    return integrate_segment(single+(air.rayleigh+air.mie)*multiple,air.extinction,dt);
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
