@group(0) @binding(1) var sky: texture_2d<f32>;
@group(0) @binding(2) var lut_sampler: sampler;
@group(0) @binding(3) var<storage,read_write> directional: array<vec4<f32>,4>;
var<workgroup> irradiance: array<vec3<f32>,64>;

// Integrate incident sky radiance over the upper hemisphere with its physical
// projected-solid-angle weight: E = integral L(omega) cos(theta) d omega.
// This is horizontal illuminance; the existing material closure uses it as
// an isotropic diffuse approximation until directional environment SH lands.
@compute @workgroup_size(64)
fn diffuse_main(@builtin(local_invocation_index) index: u32) {
    var sum=vec3(0.0);
    for(var sample=0u;sample<4u;sample++) {
        let number=index+sample*64u;
        let mu=(f32(number/16u)+0.5)/16.0;
        let phi=2.0*PI*(f32(number%16u)+0.5)/16.0;
        let radius=sqrt(max(0.0,1.0-mu*mu));
        let direction=vec3(radius*cos(phi),mu,radius*sin(phi));
        let uv=sky_uv(direction,atmosphere.sun_lux.xyz);
        let radiance=textureSampleLevel(sky,lut_sampler,uv,0.0).rgb/SKY_LUT_GAIN;
        sum+=radiance*(mu*2.0*PI/256.0);
    }
    irradiance[index]=sum;
    workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u) {
        if index<stride { irradiance[index]+=irradiance[index+stride]; }
        workgroupBarrier();
    }
    if index==0u {
        directional[3]=vec4(irradiance[0]*atmosphere.sun_lux.w*atmosphere.colour_pre_exposure.rgb,0.0);
    }
}
