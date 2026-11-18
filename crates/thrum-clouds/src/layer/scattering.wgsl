// Unscattered beam and diffuse radiance along the effective lit plane.
fn cloud_solar_source(phases:vec2<f32>,sun_depths:vec2<f32>,above:f32,viewer:vec3<f32>,
                      view_depth:f32,boundary:f32,path:vec3<f32>,droplet:vec3<f32>,g:f32)->f32 {
    let reduced=1.0-droplet.x;
    // Shift the rim's energy toward the narrow disc lobe. This softens
    // pearly halos at large angles without raising the Sun or ambient level.
    let beam_rate=mix(max(droplet.y,reduced),reduced,0.75);
    let beam=exp(-beam_rate*sun_depths.x);
    let remaining=exp(-reduced*sun_depths.x);
    let forward=dot(phases,vec2(beam,max(0.0,remaining-beam)));

    let path_cosine=length(path.xz);
    // The vertical column is uneroded. Give short, edge-grazing paths
    // a deeper margin before inferring a plane from unequal depths.
    let margin=8.0;
    let ratio=(above+margin)/(sun_depths.x+margin/path.y);
    let cotangent=(ratio-path.y)/max(0.3,path_cosine);
    let slope=inverseSqrt(1.0+cotangent*cotangent);
    let incidence=min(1.0,ratio*slope);
    let along=path.xz/max(path_cosine,1e-6);
    let normal=slope*vec3(cotangent*along.x,1.0,cotangent*along.y);
    let reduced_cosine=(1.0-g)*incidence;
    let x=reduced_cosine*sun_depths.x;
    let column=reduced_cosine*max(sun_depths.x,sun_depths.y);
    let escape=(3.0/7.0)*(1.0+2.0*incidence);
    let transmitted=min(1.0,(escape+(1.0-escape)*exp(-reduced*sun_depths.y))/(1.0+0.75*column));
    let mean=max(0.0,incidence/(2.0*PI)*(1.0-transmitted*(1.0+1.5*x)+(2.0*escape-1.0)*(1.0-remaining)));
    let flow=3.0*incidence/(4.0*PI)*(transmitted-remaining);
    let cosine=dot(viewer,normal);
    let length_scale=(column+4.0/3.0)/PI;
    let view=(1.0-g)*view_depth;
    let depth=select(column-x,x,cosine>0.0);
    let free_boundary=select(0.0,boundary*clamp(1.0-view*abs(cosine)/max(depth,1e-6),0.0,1.0),depth>0.0);
    let adjustment=free_boundary*exp(-(2.0/3.0)*view/length_scale)/(2.0/3.0+length_scale);
    let diffuse=(1.0-adjustment*length_scale)*(mean-g*flow*cosine)+g*mean*adjustment*(2.0/3.0);
    return forward+max(0.0,diffuse);
}
