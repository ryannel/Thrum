struct GpuSunMapping {
    region: vec4<f32>,
    plane_drift: vec4<f32>,
    path_border: vec4<f32>,
    far: vec4<f32>,
    near: vec4<f32>,
    middle: vec4<f32>,
};
@group(1) @binding(13) var cloud_optical_depth: texture_2d<f32>;
@group(1) @binding(14) var<uniform> cloud_field: GpuSunMapping;

fn cloud_path_supported() -> bool {
    return cloud_field.plane_drift.w != 0.0;
}

fn atlas_tau(point:vec2<f32>,region:vec4<f32>,tile:vec2<f32>)->f32 {
    let size=vec2<f32>(textureDimensions(cloud_optical_depth));
    let uv=clamp((point-region.xy)/region.z,1.0/size,vec2<f32>(1.0)-1.0/size);
    return max(0.0,textureSampleLevel(cloud_optical_depth,aerial_sampler,(tile+uv)*0.5,0.0).r);
}
fn atlas_weight(point:vec2<f32>,region:vec4<f32>)->f32 {
    let edge=min(point-region.xy,region.xy+region.z-point);
    return smoothstep(0.0,region.z*0.15,min(edge.x,edge.y));
}

fn cloud_plane_factor(world_point:vec3<f32>)->f32 {
    if !cloud_path_supported() || world_point.y>=cloud_field.plane_drift.x {return 1.0;}
    let direction=cloud_field.path_border.xyz;
    let distance=(cloud_field.plane_drift.x-world_point.y)/direction.y;
    let point=world_point.xz+direction.xz*distance-cloud_field.plane_drift.yz;
    let uv=(point-cloud_field.region.xy)/(cloud_field.region.zw-cloud_field.region.xy);
    let edge=min(point-cloud_field.region.xy,cloud_field.region.zw-point);
    let width=max(0.001,cloud_field.path_border.w);
    let w=smoothstep(0.0,width,min(edge.x,edge.y));
    var tau=0.0;
    if cloud_field.far.y==1.0 {
        let outer=vec4<f32>(cloud_field.region.xy,cloud_field.region.z-cloud_field.region.x,0.0);
        tau=atlas_tau(point,outer,vec2<f32>(0.0,1.0));
        tau=mix(tau,atlas_tau(point,cloud_field.middle,vec2<f32>(1.0,0.0)),atlas_weight(point,cloud_field.middle));
        tau=mix(tau,atlas_tau(point,cloud_field.near,vec2<f32>(0.0)),atlas_weight(point,cloud_field.near));
    }else{tau=max(0.0,textureSampleLevel(cloud_optical_depth,aerial_sampler,clamp(uv,vec2<f32>(0.0),vec2<f32>(1.0)),0.0).r);}
    return mix(exp(-max(0.0,cloud_field.far.x)),exp(-tau),w);
}
