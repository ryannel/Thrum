fn cloud_smooth(x:f32)->f32 {let a=clamp(x,0.0,1.0);return a*a*(3.0-2.0*a);}

fn cloud_window_depths(world:vec3<f32>,start_height:f32,limits:vec2<f32>)->vec3<f32> {
    let point=world.xz-mapping.path_border.xz*(world.y-mapping.plane_drift.x)/mapping.path_border.y;
    let heights=vec2(world.y,start_height);
    let near=(point-mapping.near.xy)/mapping.near.z;
    let nw=cloud_smooth(min(min(near.x,near.y),min(1.0-near.x,1.0-near.y))/0.15);
    if nw==1.0 {return cloud_path_sample(sun_near,near,heights,limits);}
    let middle=(point-mapping.middle.xy)/mapping.middle.z;
    let mw=cloud_smooth(min(min(middle.x,middle.y),min(1.0-middle.x,1.0-middle.y))/0.15);
    var tau=vec3(0.0);
    if mw==1.0 {tau=cloud_path_sample(sun_middle,middle,heights,limits);}
    else {
        let region=vec4(mapping.region.xy,mapping.region.z-mapping.region.x,0.0);
        tau=cloud_path_sample(sun_far,clamp((point-region.xy)/region.z,vec2(0.0),vec2(1.0)),heights,limits);
        if mw>0.0 {tau=mix(tau,cloud_path_sample(sun_middle,middle,heights,limits),mw);}
    }
    if nw>0.0 {tau=mix(tau,cloud_path_sample(sun_near,near,heights,limits),nw);}
    return tau;
}

