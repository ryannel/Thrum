fn cloud_shape_uv(point:vec3<f32>)->vec3<f32> {
    return point/6400.0+vec3(cloud.noise.x,0.0,cloud.noise.y);
}
fn cloud_billow(uv:vec3<f32>,footprint:f32)->f32 {
    return textureSampleLevel(shape_noise,noise_sampler,uv,erosion_mip(footprint,50.0)).r;
}
