fn cloud_shape_uv(point:vec3<f32>)->vec3<f32> {
    return point/6400.0+vec3(cloud.noise.x,0.0,cloud.noise.y);
}
fn cloud_billow(uv:vec3<f32>,footprint:f32)->f32 {
    return textureSampleLevel(shape_noise,noise_sampler,uv,erosion_mip(footprint,50.0)).r;
}

fn cloud_eroded_density(sigma:f32,point:vec3<f32>,footprint:f32,extinction:f32,height:f32,detail:f32)->f32 {
    if sigma<=0.0 || extinction<=0.0 || cloud.physical.y==0.0 {return sigma;}
    let uv=cloud_shape_uv(point);
    let shape=cloud_billow(uv,footprint);
    let eroded=cloud_erode(sigma,extinction,cloud.physical.y,shape,detail);
    if eroded==0.0 {return 0.0;}
    return eroded*cloud_bank_shape(shape_noise,noise_sampler,point,uv,cloud.bank_noise,
                                  footprint,height,cloud_bank_weight(cloud.bank_noise.w));
}
