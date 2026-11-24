// The cloud-provider variant extends only sky-ending rays; finite points keep
// the original cumulative-volume arithmetic, including its storage decoding.
fn environment_aerial(world_point: vec3<f32>, uv: vec2<f32>) -> EnvironmentAerial {
    let offset=world_point-atmosphere.camera_altitude.xyz;
    let distance=length(offset);
    // Endpoint construction and length rounding at this scale can differ by
    // a few millimetres. Keep the finite branch through that boundary shell.
    if distance<=32000.01 { return environment_aerial_volume(world_point,uv); }
    let direction=offset/distance;
    let origin=vec3(0.0,atmosphere.camera_altitude.w,0.0);
    let height=origin.y;
    if blocked(height,direction.y) { return environment_aerial_volume(world_point,uv); }
    let end=origin+direction*min(distance,top_distance(height,direction.y));
    let end_height=min(H,altitude(end));
    let end_mu=dot(normalize(end+vec3(0.0,R,0.0)),direction);
    let infinity=sample_sun_transmittance(height,direction.y,surface_transmittance,aerial_sampler);
    let remaining=sample_sun_transmittance(end_height,end_mu,surface_transmittance,aerial_sampler);
    let transmission=clamp(infinity/max(remaining,vec3(1e-20)),vec3(0.0),vec3(1.0));
    let boundary=environment_aerial_volume(atmosphere.camera_altitude.xyz+direction*32000.0,uv);
    let at_boundary=origin+direction*32000.0;
    let boundary_mu=dot(normalize(at_boundary+vec3(0.0,R,0.0)),direction);
    let boundary_remaining=sample_sun_transmittance(altitude(at_boundary),boundary_mu,surface_transmittance,aerial_sampler);
    let boundary_transmission=clamp(infinity/max(boundary_remaining,vec3(1e-20)),vec3(0.0),vec3(1.0));
    let weight=clamp((boundary_transmission-transmission)/max(boundary_transmission-infinity,vec3(1e-8)),vec3(0.0),vec3(1.0));
    let continued=boundary.transmission*clamp(transmission/max(boundary_transmission,vec3(1e-20)),vec3(0.0),vec3(1.0));
    return EnvironmentAerial(continued,mix(boundary.scatter,environment_far_sky(direction,uv),weight));
}

fn environment_aerial_volume(world_point: vec3<f32>, uv: vec2<f32>) -> EnvironmentAerial {
    let sample=aerial_sample(length(world_point-atmosphere.camera_altitude.xyz),clamp(uv,vec2(0.0),vec2(1.0)));
    return EnvironmentAerial(sample.transmission,aerial_radiance(sample.scatter.rgb)+vec3(sample.night));
}

