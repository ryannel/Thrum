fn cloud_phase_row(row:u32,s2:f32)->f32 {
    let entries=u32(cloud.erosion.w);
    let at=sqrt(sqrt(clamp(s2,0.0,1.0)))*f32(entries-1u);
    let lo=min(u32(floor(at)),entries-2u);
    let start=row*entries+lo;
    return mix(cloud_phase[start],cloud_phase[start+1u],at-f32(lo));
}
fn cloud_phases(ray:vec3<f32>,sun:vec3<f32>)->vec3<f32> {
    // |ray-sun|^2/4 preserves the forward peak where 1-dot loses digits.
    let s2=clamp(dot(ray-sun,ray-sun)*0.25,0.0,1.0);
    let row=u32(cloud.erosion.x);
    let first=mix(cloud_phase_row(row,s2),cloud_phase_row(row+1u,s2),cloud.erosion.y);
    // Narrow the middle lobe to avoid a bright ring far outside the Sun's vicinity.
    let grazing=1.0-smoothstep(0.18,0.35,mapping.path_border.y);
    let middle=mix(cloud.erosion.z*cloud.scattering.z,cloud.erosion.z,0.65);
    return vec3(first,cloud_hg_phase(s2,mix(middle,0.96,grazing)),grazing);
}
fn cloud_light_angles(viewer:vec3<f32>)->vec4<f32> {
    let path=mapping.path_border.xyz;
    // The plane is shared by the whole cloud. Turn it less toward the Sun
    // at the lowest elevations to retain contrast on the warm flank.
    let turn=cloud.scattering.w*mix(0.50,1.0,smoothstep(0.04,0.32,path.y));
    let normal=normalize(mix(vec3(0.0,1.0,0.0),path,turn));
    // At grazing angles a view into the Sun exposes the back boundary of
    // a heap even where the Sun column continues into neighbouring lobes.
    let back=clamp(2.5*cloud.scattering.w,0.0,1.0)*(1.0-smoothstep(0.18,0.35,path.y))
        *smoothstep(0.74,0.96,-dot(viewer,path));
    let escape_depth=mix(8.0,4.0,smoothstep(0.07,0.17,path.y));
    return vec4(dot(normal,path),dot(normal,viewer),back,mix(1.0,escape_depth,back));
}
fn cloud_source(solar:vec3<f32>,phases:vec3<f32>,sun_depths:vec2<f32>,
                vertical_depths:vec2<f32>,light_angles:vec4<f32>,distance:f32,transmission:f32)->vec3<f32> {
    return cloud_source_with_diffuse(solar,phases,sun_depths.x,sun_depths,1.0,
                                     vertical_depths,light_angles,distance,transmission);
}

fn cloud_source_with_diffuse(solar:vec3<f32>,phases:vec3<f32>,sun_depth:f32,diffuse_column:vec2<f32>,
                             diffuse_scale:f32,vertical_depths:vec2<f32>,light_angles:vec4<f32>,
                             distance:f32,transmission:f32)->vec3<f32> {
    let g=cloud.erosion.z;
    let kernel=cloud_sun_term(phases,sun_depth,diffuse_column,diffuse_scale,
                              light_angles,distance,transmission,vec4(cloud.scattering.xyz,g));
    let diffuse_depth=0.75*(1.0-g)*vertical_depths;
    let ambient=cloud_ambient[0].rgb/(1.0+diffuse_depth.x)+cloud_ambient[1].rgb/(1.0+diffuse_depth.y);
    return solar*kernel+ambient;
}

fn cloud_ambient_light(vertical_depths: vec2<f32>) -> vec3<f32> {
    let diffuse_depth=0.75*(1.0-cloud.erosion.z)*vertical_depths;
    return cloud_ambient[0].rgb/(1.0+diffuse_depth.x)+cloud_ambient[1].rgb/(1.0+diffuse_depth.y);
}
