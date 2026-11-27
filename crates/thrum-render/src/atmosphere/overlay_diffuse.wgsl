@group(0) @binding(6) var<uniform> overlay_sun: DirectionalLight;

fn overlay_open_sun() -> f32 {
    return dot(overlay_sun.irradiance.rgb, REC709_LUMA) * max(overlay_sun.direction_to_light.y, 0.0);
}

@group(0) @binding(1) var sky: texture_2d<f32>;
@group(0) @binding(2) var aerial_sampler: sampler;
@group(0) @binding(3) var<storage,read_write> environment_light: EnvironmentLight;
@group(0) @binding(7) var overlay_faces: texture_2d_array<f32>;
@group(0) @binding(8) var<uniform> overlay_chart: ChartLighting;
var<workgroup> clear_meter: array<f32,64>;
var<workgroup> has_cloud: u32;
var<workgroup> irradiance: array<vec4<f32>,64>;
var<workgroup> moments: array<array<vec4<f32>,9>,64>;

@compute @workgroup_size(64)
fn diffuse_main(@builtin(local_invocation_index) index: u32) {
    // Preserve the existing horizontal incident meter, including sample order
    // and reduction. It is not reconstructed from the truncated SH expansion.
    var active_overlay = overlay_chart.cloudy != 0u;
    if overlay_chart.mode == 0u {
        clear_meter[index] = f32(overlay_face_has_cloud(index));
        workgroupBarrier();
        if index == 0u {
            has_cloud = 0u;
            for (var i = 0u; i < 64u; i++) {
                has_cloud |= u32(clear_meter[i]);
            }
        }
        active_overlay = workgroupUniformLoad(&has_cloud) != 0u;
    }
    let solar=atmosphere.sun_lux.w*atmosphere.colour_pre_exposure.rgb;
    var sum=vec4(0.0);
    var clear_lux = 0.0;
    for(var sample=0u;sample<4u;sample++) {
        let direction = overlay_meter_direction(index + sample * 64u);
        let mu = direction.y;
        let uv=sky_uv(direction,atmosphere.sun_lux.xyz);
        let clear=textureSampleLevel(sky,aerial_sampler,uv,0.0);
        if active_overlay {
            let layer = textureSampleLevel(sky_overlay, aerial_sampler, direction, 0.0);
            let clear_rgb = overlay_clear(clear);
            sum += vec4(clear_rgb * layer.a + layer.rgb, 0.0) * (mu*2.0*PI/256.0);
            clear_lux += dot(clear_rgb, REC709_LUMA) * (mu*2.0*PI/256.0);
        } else {
            sum += clear*(mu*2.0*PI/256.0);
        }
    }
    irradiance[index]=sum;
    clear_meter[index] = clear_lux;
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
        var radiance=clear;
        if active_overlay {
            radiance=vec4(overlay_radiance(direction,clear,aerial_sampler),0.0);
        }
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
            clear_meter[index] += clear_meter[index+stride];
            for(var i=0u;i<9u;i++) { moments[index][i]+=moments[index+stride][i]; }
        }
        workgroupBarrier();
    }
    var coefficients: array<vec4<f32>,9>;
    for(var i=0u;i<9u;i++) {
        var rgb = moments[0][i].rgb;
        if !active_overlay { rgb = decode_sky_light(moments[0][i], solar); }
        coefficients[i] = vec4(rgb, 0.0);
    }
    // Diagnostic only: count negative raw RGB on 64 fixed sphere normals, not
    // affected pixels. This cannot prove positivity at unsampled normals.
    let mu=1.0-2.0*(f32(index)+0.5)/64.0;
    let phi=f32(index)*2.39996322973;
    let radial=sqrt(max(0.0,1.0-mu*mu));
    let probe=evaluate_sky_diffuse(coefficients,vec3(radial*cos(phi),mu,radial*sin(phi)));
    // Every lane has consumed the shared meter before this scratch reuse.
    workgroupBarrier();
    if index==0u {
        var horizontal = irradiance[0].rgb;
        if !active_overlay { horizontal = decode_sky_light(irradiance[0], solar); }
        environment_light.horizontal_irradiance=vec4(horizontal,1.0);
        environment_light.sky_diffuse=coefficients;
        environment_light.sky_diffuse[2].w = select(0.0, 1.0, active_overlay);
        write_overlay_light(horizontal, clear_meter[0], active_overlay);
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
