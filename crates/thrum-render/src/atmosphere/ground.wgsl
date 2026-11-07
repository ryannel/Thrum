// Profile-only unit-Sun sky irradiance at an h=0 receiver. This integrates the
// shared single+existing-M transport, excluding disk/direct beam and terminal
// ground. M keeps its existing approximate ground feedback; there is no G->M
// recurrence or claim of exact all-orders transport here.
var<workgroup> ground_lo: array<vec4<f32>,64>;
var<workgroup> ground_hi: array<vec4<f32>,64>;
@compute @workgroup_size(64)
fn ground_main(
    @builtin(workgroup_id) group: vec3<u32>,
    @builtin(local_invocation_index) lane: u32,
) {
    if group.x>=GROUND_SUN_ENTRIES { return; }
    let sun_mu=ground_sun_mu(group.x);
    let sun=vec3(sqrt(max(0.0,1.0-sun_mu*sun_mu)),sun_mu,0.0);
    var lo=vec4(0.0);
    var hi=vec4(0.0);
    let count=GROUND_MU_SAMPLES*GROUND_AZIMUTH_SAMPLES;
    for(var index=lane;index<count;index+=64u) {
        let sample=ground_hemisphere_sample(index,GROUND_MU_SAMPLES,GROUND_AZIMUTH_SAMPLES);
        let segment=sky_transport(0.0,sample.xyz,sun,top_distance(0.0,sample.y));
        lo+=segment.lo.radiance*sample.w;
        hi+=segment.hi.radiance*sample.w;
    }
    ground_lo[lane]=lo;
    ground_hi[lane]=hi;
    workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u) {
        if lane<stride {
            ground_lo[lane]+=ground_lo[lane+stride];
            ground_hi[lane]+=ground_hi[lane+stride];
        }
        workgroupBarrier();
    }
    if lane==0u {
        textureStore(output_lut,vec2<i32>(i32(group.x),0),ground_lo[0]*GROUND_IRRADIANCE_GAIN);
        textureStore(output_lut,vec2<i32>(i32(group.x),1),ground_hi[0]*GROUND_IRRADIANCE_GAIN);
    }
}
