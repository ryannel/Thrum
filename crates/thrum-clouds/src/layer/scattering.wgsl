// Pure local source shared by the production trace and retained-slab observer.
fn cloud_order_source(solar:vec3<f32>,phases:vec4<f32>,sun_depth:f32,vertical_depths:vec2<f32>,mean_cosine:f32,above:vec3<f32>,below:vec3<f32>)->vec3<f32> {
    let diffuse_depth=0.75*(1.0-mean_cosine)*vertical_depths;
    let ambient=above/(1.0+diffuse_depth.x)+below/(1.0+diffuse_depth.y);
    return solar*dot(CLOUD_WEIGHTS*phases,exp(-CLOUD_BRANCHES*sun_depth))+ambient;
}
