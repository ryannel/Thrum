// uv is the point's normalized position in the active view, top-left origin.
fn environment_aerial(world_point: vec3<f32>, uv: vec2<f32>) -> EnvironmentAerial {
    let sample=aerial_sample(length(world_point-atmosphere.camera_altitude.xyz),clamp(uv,vec2(0.0),vec2(1.0)));
    return EnvironmentAerial(sample.transmission,aerial_radiance(sample.scatter.rgb)+vec3(sample.night));
}

