@group(1) @binding(11) var surface_transmittance: texture_2d<f32>;

fn sun_irradiance(world_position: vec3<f32>) -> vec3<f32> {
    // The atmosphere's tangent origin follows the camera in world XZ, while
    // its checked physical altitude can have a different world-space datum.
    let p=world_position-aerial_frame.camera_altitude.xyz
        +vec3(0.0,aerial_frame.camera_altitude.w,0.0);
    // Start from the unattenuated source. The camera-height directional value
    // is retained for metering, never multiplied into this receiver path.
    let source=aerial_frame.sun_lux.w*aerial_frame.colour_pre_exposure.rgb;
    var h=altitude(p);
    var mu=dot(normalize(p+vec3(0.0,R,0.0)),aerial_frame.sun_lux.xyz);
    if h>H {
        // The chart starts inside the shell. Vacuum contributes no extinction;
        // if the Sun ray enters, use its inward cosine at the shell boundary.
        let b=(R+h)*mu;
        let c=(h-H)*(2.0*R+h+H);
        let discriminant=b*b-c;
        if b>=0.0 || discriminant<=0.0 { return source; }
        h=H;
        mu=-sqrt(discriminant)/(R+H);
    }
    return sample_sun_transmittance(h,mu,surface_transmittance,aerial_sampler)*source;
}
