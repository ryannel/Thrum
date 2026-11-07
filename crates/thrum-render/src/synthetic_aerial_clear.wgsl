fn aerial_colour(colour: vec3<f32>, world_position: vec3<f32>, screen_position: vec4<f32>) -> vec3<f32> {
    let sample = aerial_sample(length(world_position - aerial_frame.camera_altitude.xyz), aerial_uv(screen_position));
    return sample.transmission * colour + aerial_radiance(sample.scatter.rgb);
}
