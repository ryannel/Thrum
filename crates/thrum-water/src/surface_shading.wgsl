    let motion = cascade_motion(input.rest, row.current, row.track, water.carriage.w, water.carriage.y);
    let departure_dx = dpdx(motion.departure);
    let departure_dy = dpdy(motion.departure);
    let second_dx = dpdx(motion.second);
    let second_dy = dpdy(motion.second);
    if !row.owned { discard; }
    let surf_normal = input.surf_normal_variance.xyz;
    let use_sheet_normal = SURF_CACHE_ENABLED && input.surf_weight >= 0.999
        && dot(surf_normal,surf_normal) > 1e-8;
    // Select a cache only for spatial coverage. Hardware filters each layer
    // over the ellipse; its longest side never selects a level or mip.
    let cache_lod=long_lod(input.rest,1.0);let low_cache=u32(floor(cache_lod));
    var slope=vec2(0.0);
    let gain=tail_gain(row);
    let tail=water.lengths.w*0.5*gain*gain;
    let wake_xx=dot(vec2(wake_dx.x,wake_dy.x),vec2(wake_dx.x,wake_dy.x));
    let wake_xz=wake_dx.x*wake_dx.y+wake_dy.x*wake_dy.y;
    let wake_zz=dot(vec2(wake_dx.y,wake_dy.y),vec2(wake_dx.y,wake_dy.y));
    var covariance=vec3(tail,0.0,tail)+vec3(wake_xx,wake_xz,wake_zz)/12.0;
    var differential=vec3(1.0,0.0,1.0);
    let fade=1.0-smoothstep(4000.0,8000.0,length(input.rest-view.camera_xyz_time.xz));
    var long_mean=vec2(0.0);var long_second=vec3(0.0);var long_differential=vec3(0.0);
    if !use_sheet_normal {
        for(var side=0u;side<2u;side++) {
            let level=min(low_cache+side,7u);let scale=128.0*water.cache[level].z;
            let uv=(input.rest-water.cache[level].xy)/scale;
            let m=filtered_layer(long_field,bounded,uv,dx/scale,dy/scale,i32(level*5u));
            let w=select(1.0-fract(cache_lod),fract(cache_lod),side==1u);
            long_mean+=w*m.slope;
            long_second+=w*(m.second+water.long_covariance[level].xyz);
            long_differential+=w*m.differential;
        }
        slope=row.gains.x*fade*long_mean;
        covariance+=row.gains.x*row.gains.x*(long_second-fade*fade*vec3(long_mean.x*long_mean.x,long_mean.x*long_mean.y,long_mean.y*long_mean.y));
        differential+=row.gains.x*fade*long_differential;
    }
    slope+=wake_slope;
    for(var band=0u;band<3u;band++) {
        let scale=water.lengths[band];
        let layer=i32(band*5u);
        var m=filtered_layer(cascades,periodic,motion.departure/scale,departure_dx/scale,departure_dy/scale,layer);
        if motion.blend < 1.0 {
            let ambient = filtered_layer(cascades,periodic,motion.second/scale,second_dx/scale,second_dy/scale,layer);
            m = FilteredLayer(mix(ambient.slope,m.slope,motion.blend),
                mix(ambient.second,m.second,motion.blend),mix(ambient.differential,m.differential,motion.blend));
        }
        let gain=row.gains[band+1u];
        slope+=gain*m.slope;
        covariance+=gain*gain*(m.second-vec3(m.slope.x*m.slope.x,m.slope.x*m.slope.y,m.slope.y*m.slope.y));
        differential+=gain*m.differential;
    }
