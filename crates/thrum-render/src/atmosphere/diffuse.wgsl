@group(0) @binding(1) var sky: texture_2d<f32>;
@group(0) @binding(2) var lut_sampler: sampler;
@group(0) @binding(3) var<storage,read_write> directional: array<vec4<f32>,13>;
var<workgroup> irradiance: array<vec3<f32>,64>;
var<workgroup> moments: array<array<vec3<f32>,9>,64>;

// GL16 on [0,1], derived by Legendre recurrence/Newton iteration. The squared
// horizon-distance warp resolves the bright horizon on both disjoint LUT sides.
const GL: array<vec2<f32>,16> = array<vec2<f32>,16>(
    vec2(0.005299532504,0.01357622971),
    vec2(0.02771248846,0.03112676197),
    vec2(0.06718439881,0.04757925584),
    vec2(0.1222977958,0.06231448563),
    vec2(0.1910618778,0.07479799441),
    vec2(0.2709916112,0.0845782597),
    vec2(0.3591982246,0.09130170752),
    vec2(0.4524937451,0.09472530523),
    vec2(0.5475062549,0.09472530523),
    vec2(0.6408017754,0.09130170752),
    vec2(0.7290083888,0.0845782597),
    vec2(0.8089381222,0.07479799441),
    vec2(0.8777022042,0.06231448563),
    vec2(0.9328156012,0.04757925584),
    vec2(0.9722875115,0.03112676197),
    vec2(0.9947004675,0.01357622971));

@compute @workgroup_size(64)
fn diffuse_main(@builtin(local_invocation_index) index: u32) {
    // Preserve the existing horizontal incident meter, including sample order
    // and reduction. It is not reconstructed from the truncated SH expansion.
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
    var local: array<vec3<f32>,9>;
    for(var sample=0u;sample<16u;sample++) {
        let number=index+sample*64u;
        let side=number/512u;
        let node=GL[(number%512u)/32u];
        let horizon=atmosphere.sky_mapping.x;
        let span=select(1.0-horizon,1.0+horizon,side==0u);
        let sign=select(1.0,-1.0,side==0u);
        let mu=horizon+sign*span*node.x*node.x;
        let phi=2.0*PI*(f32(number%32u)+0.5)/32.0;
        let radius=sqrt(max(0.0,1.0-mu*mu));
        let direction=vec3(radius*cos(phi),mu,radius*sin(phi));
        let radiance=textureSampleLevel(sky,lut_sampler,sky_uv(direction,atmosphere.sun_lux.xyz),0.0).rgb/SKY_LUT_GAIN;
        let weighted=radiance*(2.0*span*node.x*node.y*2.0*PI/32.0);
        // M0/4, M1/2, 15Qdiag/32, 15Qcross/16. No coefficient clamp.
        let basis=array<f32,9>(0.25,0.5*direction.x,0.5*direction.y,0.5*direction.z,
            15.0/32.0*(direction.x*direction.x-1.0/3.0),
            15.0/32.0*(direction.y*direction.y-1.0/3.0),
            15.0/16.0*direction.x*direction.y,
            15.0/16.0*direction.x*direction.z,
            15.0/16.0*direction.y*direction.z);
        for(var i=0u;i<9u;i++) { local[i]+=weighted*basis[i]; }
    }
    moments[index]=local;
    workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u) {
        if index<stride {
            irradiance[index]+=irradiance[index+stride];
            for(var i=0u;i<9u;i++) { moments[index][i]+=moments[index+stride][i]; }
        }
        workgroupBarrier();
    }
    let solar=atmosphere.sun_lux.w*atmosphere.colour_pre_exposure.rgb;
    var coefficients: array<vec4<f32>,9>;
    for(var i=0u;i<9u;i++) { coefficients[i]=vec4(moments[0][i]*solar,0.0); }
    // Diagnostic only: count negative raw RGB on 64 fixed sphere normals, not
    // affected pixels. This cannot prove positivity at unsampled normals.
    let mu=1.0-2.0*(f32(index)+0.5)/64.0;
    let phi=f32(index)*2.39996322973;
    let radial=sqrt(max(0.0,1.0-mu*mu));
    let probe=evaluate_sky_diffuse(coefficients,vec3(radial*cos(phi),mu,radial*sin(phi)));
    // Every lane has consumed the shared meter before this scratch reuse.
    workgroupBarrier();
    if index==0u {
        directional[3]=vec4(irradiance[0]*atmosphere.sun_lux.w*atmosphere.colour_pre_exposure.rgb,1.0);
        for(var i=0u;i<9u;i++) { directional[4u+i]=coefficients[i]; }
    }
    workgroupBarrier();
    irradiance[index]=vec3(select(0.0,1.0,any(probe<vec3(0.0))),min(probe.x,min(probe.y,probe.z)),0.0);
    workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u) {
        if index<stride {
            irradiance[index].x+=irradiance[index+stride].x;
            irradiance[index].y=min(irradiance[index].y,irradiance[index+stride].y);
        }
        workgroupBarrier();
    }
    if index==0u {
        directional[3].w+=irradiance[0].x;
        directional[4].w=irradiance[0].y;
    }
}
