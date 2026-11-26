struct Field { displacement:vec4<f32>, slope:vec4<f32>, velocity:vec4<f32>, }
fn add_field(a:Field,b:Field,w:f32)->Field {return Field(a.displacement+w*b.displacement,a.slope+w*b.slope,a.velocity+w*b.velocity);}
fn cascade_at(point:vec2<f32>,band:u32)->Field {
    let uv=point/water.lengths[band];let layer=i32(band*5u);
    let displacement=textureSampleLevel(cascades,periodic,uv,layer,0.0);
    let residual=textureSampleLevel(cascades,periodic,uv,layer+4,0.0).yzw;
    return Field(displacement+vec4(residual,0.0),textureSampleLevel(cascades,periodic,uv,layer+1,0.0),textureSampleLevel(cascades,periodic,uv,layer+2,0.0));
}
fn long_level(point:vec2<f32>,level:u32)->Field {
    let uv=(point-water.cache[level].xy)/(128.0*water.cache[level].z);let layer=i32(level*5u);
    let displacement=textureSampleLevel(long_field,bounded,uv,layer,0.0);
    let residual=textureSampleLevel(long_field,bounded,uv,layer+4,0.0).yzw;
    let result=Field(displacement+vec4(residual,0.0),textureSampleLevel(long_field,bounded,uv,layer+1,0.0),textureSampleLevel(long_field,bounded,uv,layer+2,0.0));
    // The skirt closes at the mathematical horizon, beyond all resolved waves.
    let fade=1.0-smoothstep(4000.0,8000.0,length(point-view.camera_xyz_time.xz));
    return add_field(Field(vec4(0.0),vec4(0.0),vec4(0.0)),result,fade);
}
fn long_lod(point:vec2<f32>,spacing:f32)->f32 {
    let level=clamp(log2(max(max(spacing,1.0),length(point-view.camera_xyz_time.xz)/32.0)),0.0,7.0);
    // Both caches cover this point. Retain the finer resolved waves until
    // the outer part of their overlap, rather than turning them into the
    // coarser level's statistics as soon as that level becomes available.
    return floor(level)+smoothstep(0.5,1.0,fract(level));
}
fn long_at(point:vec2<f32>,spacing:f32)->Field {
    let lod=long_lod(point,spacing);let low=u32(floor(lod));
    let a=long_level(point,low);let b=long_level(point,min(low+1u,7u));
    return add_field(add_field(Field(vec4(0.0),vec4(0.0),vec4(0.0)),a,1.0-fract(lod)),b,fract(lod));
}
fn drifting_field(point: vec2<f32>, drift: vec2<f32>, band: u32) -> Field {
    var sample = cascade_at(point, band);
    // Motion vectors track the displaced sheet at its fixed mesh point.
    let bend = sample.slope.zw;
    sample.velocity -= vec4(
        dot(vec2(bend.x, bend.y), drift),
        dot(sample.slope.xy, drift),
        dot(vec2(bend.y, sample.velocity.w), drift),
        0.0);
    return sample;
}

fn transported_field(motion: CascadeMotion, band: u32) -> Field {
    var sample = drifting_field(motion.departure, motion.drift, band);
    if motion.blend < 1.0 {
        let second = drifting_field(motion.second, motion.second_drift, band);
        let renewal = motion.blend_rate * (sample.displacement - second.displacement);
        sample = add_field(add_field(Field(vec4(0.0), vec4(0.0), vec4(0.0)),
            second, 1.0 - motion.blend), sample, motion.blend);
        sample.velocity += renewal;
    }
    return sample;
}

fn field_at(point:vec2<f32>,spacing:f32,row:SurfaceRow)->Field {
    let long=long_at(point,spacing);
    var f=add_field(Field(vec4(0.0),vec4(0.0),vec4(0.0)),long,row.gains.x);
