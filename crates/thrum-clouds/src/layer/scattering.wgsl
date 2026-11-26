fn cloud_solar_beam(phases:vec3<f32>,sun_depth:f32,droplet:vec3<f32>)->f32 {
    let reduced=1.0-droplet.x;
    // Let the narrowed middle lobe take its full share at grazing Sun.
    // Both forward lobes retain their normalisation.
    let beam_rate=mix(max(droplet.y,reduced),reduced,0.75*(1.0-phases.z));
    let beam=exp(-beam_rate*sun_depth);
    let remaining=exp(-reduced*sun_depth);
    return dot(phases.xy,vec2(beam,max(0.0,remaining-beam)));
}

// Diffuse solar radiance along the effective lit plane.
fn cloud_solar_diffuse(sun_depths:vec2<f32>,light_angles:vec4<f32>,
                       view_depth:f32,boundary:f32,droplet:vec3<f32>,g:f32)->f32 {
    let reduced=1.0-droplet.x;
    let remaining=exp(-reduced*sun_depths.x);

    let incidence=light_angles.x;
    let reduced_cosine=(1.0-g)*incidence;
    let x=reduced_cosine*sun_depths.x;
    let column=reduced_cosine*max(sun_depths.x,sun_depths.y);
    let escape=(3.0/7.0)*(1.0+2.0*incidence);
    let transmitted=min(1.0,(escape+(1.0-escape)*exp(-reduced*sun_depths.y))/(1.0+0.75*column));
    let mean=max(0.0,incidence/(2.0*PI)*(1.0-transmitted*(1.0+1.5*x)+(2.0*escape-1.0)*(1.0-remaining)));
    let flow=3.0*incidence/(4.0*PI)*(transmitted-remaining);
    let cosine=light_angles.y;
    // At the back of a heap, diffuse light escapes across the view boundary
    // over a longer distance than the projection on the grazing Sun column.
    let length_scale=(column+4.0/3.0)/PI*light_angles.w;
    let view=(1.0-g)*view_depth;
    let depth=select(column-x,x,cosine>0.0);
    let plane_boundary=select(0.0,clamp(1.0-view*abs(cosine)/max(depth,1e-6),0.0,1.0),depth>0.0);
    let free_boundary=boundary*max(plane_boundary,light_angles.z);
    let adjustment=free_boundary*exp(-(2.0/3.0)*view/length_scale)/(2.0/3.0+length_scale);
    let diffuse=(1.0-adjustment*length_scale)*(mean-g*flow*cosine)+g*mean*adjustment*(2.0/3.0);
    return max(0.0,diffuse);
}
