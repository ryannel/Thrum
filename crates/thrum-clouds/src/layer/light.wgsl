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
