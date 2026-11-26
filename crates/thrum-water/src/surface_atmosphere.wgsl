    let uv=input.clip.xy*view_aux.viewport_inv_viewport.zw;
    let air=environment_aerial(input.world,uv);
    let colour=(light*air.transmission+air.scatter)*environment_storage_scale();
    var screen_motion=(input.previous.xy/input.previous.w-input.current.xy/input.current.w)*vec2(0.5,-0.5)*view_aux.viewport_inv_viewport.xy;
    if water.level_dt_reset.z>0.5 {screen_motion=vec2(65504.0);}
    var opacity=surface_opacity(row,input.world,v.y);
    if OPTICS_ENABLED { opacity=smoothstep(0.0,0.095,paths.edge_depth); }
