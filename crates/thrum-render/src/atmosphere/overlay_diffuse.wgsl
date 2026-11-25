@group(0) @binding(1) var sky: texture_2d<f32>;
@group(0) @binding(2) var aerial_sampler: sampler;
@group(0) @binding(3) var<storage,read_write> environment_light: EnvironmentLight;
var<workgroup> irradiance: array<vec4<f32>,64>;
var<workgroup> moments: array<array<vec4<f32>,9>,64>;

@compute @workgroup_size(64)
fn diffuse_main(@builtin(local_invocation_index) index: u32) {
    // Preserve the existing horizontal incident meter, including sample order
    // and reduction. It is not reconstructed from the truncated SH expansion.
    var sum=vec4(0.0);
    for(var sample=0u;sample<4u;sample++) {
        let number=index+sample*64u;
        let mu=(f32(number/16u)+0.5)/16.0;
        let phi=2.0*PI*(f32(number%16u)+0.5)/16.0;
        let radius=sqrt(max(0.0,1.0-mu*mu));
        let direction=vec3(radius*cos(phi),mu,radius*sin(phi));
        let uv=sky_uv(direction,atmosphere.sun_lux.xyz);
        let clear=textureSampleLevel(sky,aerial_sampler,uv,0.0);
        sum+=overlay_meter_sample(direction,clear)*(mu*2.0*PI/256.0);
    }
    irradiance[index]=sum;
    var local: array<vec4<f32>,9>;
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
        let clear=textureSampleLevel(sky,aerial_sampler,sky_uv(direction,atmosphere.sun_lux.xyz),0.0);
        let radiance=vec4(overlay_radiance(direction,clear,aerial_sampler),0.0);
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
    var coefficients: array<vec4<f32>,9>;
    for(var i=0u;i<9u;i++) { coefficients[i]=vec4(moments[0][i].rgb,0.0); }
    // Diagnostic only: count negative raw RGB on 64 fixed sphere normals, not
    // affected pixels. This cannot prove positivity at unsampled normals.
    let mu=1.0-2.0*(f32(index)+0.5)/64.0;
    let phi=f32(index)*2.39996322973;
    let radial=sqrt(max(0.0,1.0-mu*mu));
    let probe=evaluate_sky_diffuse(coefficients,vec3(radial*cos(phi),mu,radial*sin(phi)));
    // Every lane has consumed the shared meter before this scratch reuse.
    workgroupBarrier();
    if index==0u {
        environment_light.horizontal_irradiance=vec4(irradiance[0].rgb,1.0);
        environment_light.sky_diffuse=coefficients;
        write_haze_scale(irradiance[0]);
    }
    workgroupBarrier();
    irradiance[index]=vec4(select(0.0,1.0,any(probe<vec3(0.0))),min(probe.x,min(probe.y,probe.z)),0.0,0.0);
    workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u) {
        if index<stride {
            irradiance[index].x+=irradiance[index+stride].x;
            irradiance[index].y=min(irradiance[index].y,irradiance[index+stride].y);
        }
        workgroupBarrier();
    }
    if index==0u {
        environment_light.horizontal_irradiance.w+=irradiance[0].x;
        environment_light.sky_diffuse[0].w=irradiance[0].y;
    }
}
