@group(0) @binding(6) var volume_sky: texture_2d_array<f32>;

@fragment fn fragment(input: VertexOutput) -> SkyOutput {
    let far=atmosphere.inverse_view_projection*vec4(input.ndc,0.0,1.0);
    let direction=normalize(far.xyz/far.w-atmosphere.camera_altitude.xyz);
    let sun=atmosphere.sun_lux.xyz;
    let uv=sky_uv(direction,sun);
    let size=vec2<f32>(textureDimensions(sky));
    let encoded=textureSampleLevel(sky,lut_sampler,clamp(uv,0.5/size,1.0-0.5/size),0.0);
    let angular_radius=SUN_ANGULAR_RADIUS;
    let cosine=dot(direction,sun);
    // Approximate coverage in angular space. A cosine derivative expands the
    // filter far from the Sun and can paint a false disk on the horizon.
    // Orthographic views have constant direction, so keep distinct edges.
    let pixel_radius=clamp(0.5*max(length(dpdx(direction)),length(dpdy(direction))),0.0001,0.05);
    let disk=smoothstep(cos(angular_radius+pixel_radius),cos(max(0.0,angular_radius-pixel_radius)),cosine);
    let h=atmosphere.camera_altitude.w;
    let trans=sample_sun_transmittance(h,direction.y,transmittance,lut_sampler);
    let beam=disk*trans/(PI*sin(angular_radius)*sin(angular_radius));
    let solar=atmosphere.sun_lux.w*atmosphere.colour_pre_exposure.rgb;
    let radiance=(encoded.rgb/SKY_LUT_GAIN+beam)*solar+vec3(encoded.a/NIGHT_LUT_GAIN);
    let screen_uv = input.ndc * vec2(0.5, -0.5) + vec2(0.5);
    let terminal = textureSampleLevel(volume_sky, lut_sampler, screen_uv,
        i32(textureNumLayers(volume_sky)) - 1, 0.0);
    var corrected = radiance;
    if terminal.a != 1.0 || any(terminal.rgb != vec3(0.0)) {
        corrected = terminal.a * radiance + terminal.rgb * (solar / SKY_LUT_GAIN);
    }
    let stored=corrected*sky_exposure.x;
    return SkyOutput(vec4(clamp(stored,vec3(0.0),vec3(MAX_FINITE_HALF)),1.0),infinity_motion(direction,disk), sky_motion.extent.z);
}
