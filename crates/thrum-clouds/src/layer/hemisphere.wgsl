@group(1) @binding(0) var<storage,read_write> hemisphere_light:array<vec4<f32>>;
var<workgroup> hemisphere_sum:array<vec3<f32>,64>;
@compute @workgroup_size(64)
fn hemispheres(@builtin(workgroup_id) group:vec3<u32>,@builtin(local_invocation_index) lane:u32) {
    if group.x == 2u {
        // Ground caps dark-side twilight against the sky above that bearing.
        let count = arrayLength(&hemisphere_light) - 3u;
        if lane == count {
            hemisphere_light[2u + count] = environment_directional.irradiance;
        }
        if lane < count {
            let phi = f32(lane) * (2.0 * PI / f32(count));
            let direction = vec3(cos(phi), 0.012, sin(phi));
            hemisphere_light[2u + lane] = vec4(environment_sky_radiance(direction), 0.0);
        }
        return;
    }
    var sum=vec3(0.0);
    // Equal-solid-angle midpoint quadrature. Irradiance / (2*pi) is the
    // isotropic hemispheric source, so the solid-angle factor cancels here.
    for(var i=lane;i<512u;i+=64u) {
        let mu=(f32(i/32u)+0.5)/16.0;
        let phi=(f32(i%32u)+0.5)*(2.0*PI/32.0);
        let radial=sqrt(1.0-mu*mu);
        let direction=vec3(radial*cos(phi),select(mu,-mu,group.x==1u),radial*sin(phi));
        sum+=environment_sky_radiance(direction)*mu/512.0;
    }
    hemisphere_sum[lane]=sum;workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u) {
        if lane<stride {hemisphere_sum[lane]+=hemisphere_sum[lane+stride];}
        workgroupBarrier();
    }
    if lane==0u {hemisphere_light[group.x]=vec4(hemisphere_sum[0],0.0);}
}
