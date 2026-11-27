    let air=environment_aerial(input.world,uv);
    let colour=(light*air.transmission+air.scatter)*environment_storage_scale();
    var opacity=surface_opacity(row,input.world,v.y);
    if OPTICS_ENABLED { opacity=smoothstep(0.0,0.095,paths.edge_depth); }
