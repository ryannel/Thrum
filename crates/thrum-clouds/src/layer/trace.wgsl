@compute @workgroup_size(8,8)
fn trace(@builtin(global_invocation_id) id:vec3<u32>) {
    let size=textureDimensions(transport);
    let stride=cloud.temporal.z;
    let offset=vec2(cloud.temporal.y%stride,cloud.temporal.y/stride);
    // Dispatch just the current phase of a sparse trace. This visits the
    // same texels as a full dispatch with skipped lanes, without idle groups.
    let texel=select(id.xy*stride+offset,id.xy,cloud.temporal.w!=0u);
    if any(texel>=size) {return;}
    let uv=(vec2<f32>(texel)+0.5)/vec2<f32>(size);
    let clip=cloud.inverse_raster*vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),0.0,1.0);
    let camera=atmosphere.camera_altitude.xyz;let ray=normalize(clip.xyz/clip.w-camera);
    let air_camera=camera-vec3(cloud.drift.x,0.0,cloud.drift.y);
    var total=CloudTransport(vec3(0.0),1.0,0.0);
    var part=CloudTransport(vec3(0.0),1.0,0.0);
    var part_weighted=0.0;var part_mass=0.0;var part_air=vec3(1.0);
    var weighted=0.0;var opacity=0.0;
    var begin=0.0;var end=cloud.heights.w;
    if abs(ray.y)>1e-7 {
        let a=(cloud.heights.x-camera.y)/ray.y;let b=(cloud.heights.y-camera.y)/ray.y;
        begin=max(0.0,min(a,b));end=min(end,max(a,b));
    }else if camera.y<cloud.heights.x || camera.y>cloud.heights.y {end=0.0;}
    // The depth allocation can be larger than the drawn rectangle.
    // Composition tests each render pixel separately.
    let pixel=vec2<i32>(uv*vec2<f32>(cloud.render_extent));
    let depth=textureLoad(scene_depth,pixel,0);
    if depth>0.0 {
        let hit=cloud.inverse_raster*vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),depth,1.0);
        end=min(end,length(hit.xyz/hit.w-camera));
    }
    let phase=cloud_phases(ray,atmosphere.sun_lux.xyz);
    let light_angles=cloud_light_angles(-ray);
    let sun_alignment=dot(ray,mapping.path_border.xyz);
    // The sky behind the layer includes the high radiance of the solar disc.
    // Within its filtered footprint use the full transmission range of RGBA16F.
    let solar_margin = cloud.sun_angular_radius + 4.0 / f32(size.y);
    let covers_disc = dot(ray, atmosphere.sun_lux.xyz) >= cos(solar_margin);
    let transmission_floor = select(0.001, 1e-8, covers_disc);
    let sun_base=sun_irradiance(vec3(camera.x,cloud.heights.x,camera.z));
    let sun_top=sun_irradiance(vec3(camera.x,cloud.heights.y,camera.z));
    let bank=cloud_bank_weight(cloud.bank_noise.w);
    var distance=begin;var samples=0u;
    for(var i=0u;i<768u && distance<end && total.transmittance*part.transmittance>transmission_floor;i++) {
        samples+=1u;
        let step=min(end-distance,max(cloud.heights.z,distance*0.003));
        let at=distance+step*0.5;let point=air_camera+ray*at;
        let footprint=at/f32(size.y);
        let placed = cloud_placed(point, footprint);
        var column=cloud_column(point);
        if column.x+placed.x>0.0 {
            let sigma=cloud_detail(column.x,point,footprint,cloud.physical.x,column.w)+placed.x;
            column.y += placed.y;
            column.z += placed.z;
            if sigma>0.0 {
                let world=camera+ray*at;
                let solar=mix(sun_base,sun_top,clamp((point.y-cloud.heights.x)/(cloud.heights.y-cloud.heights.x),0.0,1.0));
                var sun_depths = vec2(0.0);
                if any(solar > vec3(0.0)) { sun_depths = cloud_local_sun(point,world,at,footprint,sun_alignment,placed.x>0.0); }
                var source=cloud_source(solar,phase,sun_depths,column.yz,light_angles,at,total.transmittance*part.transmittance);
                if bank>0.0 && placed.x==0.0 && any(solar>vec3(0.0)) {
                    source=cloud_bank_light(source,solar,point,sigma,at,footprint,sun_depths.x,sun_alignment,bank);
                }
                let segment=cloud_beer(sigma,sigma*source,step);
                let air=environment_aerial(world,uv).transmission;
                // A part spans at most 0.25 of the lent air's optical depth.
                // Empty gaps require no composition; a distant occupied sample
                // starts a new part before it can be mixed with the nearer one.
                if part_mass>0.0 && any(-log(max(air,vec3(1e-20))/max(part_air,vec3(1e-20)))>=vec3(0.25)) {
                    total=cloud_then(total,cloud_air_part(part,part_weighted,part_mass,camera,ray,uv));
                    part=CloudTransport(vec3(0.0),1.0,0.0);part_weighted=0.0;part_mass=0.0;
                }
                if part_mass==0.0 {part_air=air;}
                let local_amount=part.transmittance*cloud_removed(segment.optical_depth);
                let amount=total.transmittance*local_amount;
                weighted+=amount*at;opacity+=amount;
                part_weighted+=local_amount*at;part_mass+=local_amount;
                part=cloud_then(part,segment);
            }
        }
        distance+=step;
    }
    let front_distance=weighted/max(opacity,1e-10);
    total=cloud_then(total,cloud_air_part(part,part_weighted,part_mass,camera,ray,uv));
    textureStore(transport,vec2<i32>(texel),vec4(total.scattering*environment_storage_scale(),total.transmittance));
    // Distances are kilometres to remain representable in fp16.
    textureStore(front,vec2<i32>(texel),vec4(front_distance*0.001,opacity,f32(samples),1.0));
}
