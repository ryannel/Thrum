    // AO affects the ambient source only; direct Sun and local lights retain
    // their independent visibility and incident irradiance.
    let surface_base = material.base.rgb*base.rgb;
    let metalness = material.surface.z*orm.b;
    let roughness = material.surface.y*orm.g;
    var n = normalize(input.normal);
    var v = vec3(0.0,0.0,-1.0);
    var h = light.half_vector_fresnel.xyz;
    var fresnel_power = light.half_vector_fresnel.w;
    if view_aux.flags.x == 1u {
        v = normalize(view.camera_xyz_time.xyz-input.world_position);
        let hv = light.direction_to_light.xyz+v;
        h = v;
        if dot(hv,hv)>0.000001 { h = normalize(hv); }
        fresnel_power = pow(clamp(1.0-dot(v,h),0.0,1.0),5.0);
    }
    // Smooth interpolation can put a shading normal behind V on a geometrically
    // front-facing triangle. Lift it just onto V's visible side, so direct,
    // local and environment lighting share a valid normal. Reflecting it instead
    // flips grazing slopes that face the Sun away from a camera looking into it.
    // Preserve real raster backfaces and the authored normal used above for
    // geometric shadow bias.
    if front_facing {
        let behind = dot(n,v)-0.01;
        if behind < 0.0 { n = normalize(n-behind*v); }
    }
    let colour = direct_brdf(surface_base,metalness,roughness,n,
            light.direction_to_light.xyz,v,sun_irradiance(input.world_position)*sun_factor,h,fresnel_power)
        + local_radiance(surface_base,metalness,roughness,n,v,input.world_position,input.position)
