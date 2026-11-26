@group(2) @binding(9) var<storage,read> cloud_phase:array<f32>;
@group(2) @binding(10) var<storage,read> cloud_ambient:array<vec4<f32>,2>;
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

fn cloud_placed_diffuse(point:vec3<f32>,world:vec3<f32>,footprint:f32,sun_alignment:f32,
                        sun_depths:vec2<f32>,placed_share:f32)->vec3<f32> {
    // Under a high Sun, scattering travels across several narrow lobes.
    // Use the broader column for diffuse light and keep the local beam.
    let high_sun=smoothstep(0.20,0.55,mapping.path_border.y);
    let broad=high_sun*placed_share;
    var column=sun_depths;
    if broad>0.0 {
        column=mix(sun_depths,cloud_sun_depths(world,world.y).xy,broad);
    }

    // Read the density change across a billow. The wider footprint keeps
    // diffuse light from tracing each wisp.
    let light_footprint=max(55.0,footprint);
    let probe=mapping.path_border.xyz*light_footprint;
    let ahead=cloud_density(point+probe,light_footprint);
    let behind=cloud_density(point-probe,light_footprint);
    let falloff=(behind-ahead)/(max(behind,ahead)+0.22*cloud.physical.x);
    let exposed=smoothstep(-0.32,0.62,falloff);
    let light=exposed*(0.65+0.35*exp(-0.12*sun_depths.x));
    let strength=mix(0.52,1.0,high_sun)*placed_share;
    let scale=mix(1.0,0.76+0.64*light,strength);

    // Let the sheltered flank turn grey at low elevations while preserving
    // the forward rim and clear-sky ambient.
    let flank=(1.0-high_sun)*(1.0-smoothstep(0.55,0.90,sun_alignment));
    let shadow=smoothstep(0.28,0.86,sun_depths.x/max(1.0,sun_depths.y))
        *smoothstep(1.0,7.0,sun_depths.x);
    return vec3(column,scale*(1.0-0.61*placed_share*flank*shadow));
}
