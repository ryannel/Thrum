// The night profile depends only on elevation, altitude and aerosol.
// Adjacent azimuths on a chart row share its spherical integration.
var<workgroup> night_row: array<f32, LUT_WORKGROUP_EDGE>;

@compute @workgroup_size(LUT_WORKGROUP_EDGE, LUT_WORKGROUP_EDGE)
fn sky_main(
    @builtin(global_invocation_id) id: vec3<u32>,
    @builtin(local_invocation_id) lane: vec3<u32>,
) {
    let size=textureDimensions(output_lut);
    let h=atmosphere.camera_altitude.w;
    if lane.x==0u && id.y<size.y {
        let row = sky_view_ray(vec2(0.5, (f32(id.y) + 0.5) / f32(size.y)), id.y);
        night_row[lane.y] = aerosol_night_terminal(h, row.direction.y, atmosphere.sky_mapping.x,
            row.terminal_ground != 0u, lut_sampler);
    }
    workgroupBarrier();
    if any(id.xy>=size) { return; }
    let ray=sky_view_ray((vec2<f32>(id.xy)+0.5)/vec2<f32>(size),id.y);
    let sun=atmosphere.sun_lux.xyz;
    let direction=ray.direction;
    let distance=ray.distance;
    let transport=sky_transport(h,direction,sun,distance);
    var lo=transport.lo.radiance;
    var hi=transport.hi.radiance;
    if ray.terminal_ground!=0u {
        let p=vec3(0.0,h,0.0)+direction*distance;
        let n=normalize(p+vec3(0.0,R,0.0));
        let receiver_h=altitude(p+n*0.1);
        let mu=dot(n,sun);
        lo+=transport.lo.transmission*ground_radiance(receiver_h,mu,0u);
        hi+=transport.hi.transmission*ground_radiance(receiver_h,mu,1u);
    }
    textureStore(output_lut,vec2<i32>(id.xy),vec4(project_spectrum(lo,hi)*SKY_LUT_GAIN,night_row[lane.y]));
}
