
@compute @workgroup_size(8,8)
fn trace(@builtin(global_invocation_id) id:vec3<u32>) {
    let size=cloud.active_extent;
    let stride=cloud.temporal.z;
    let offset=vec2(cloud.temporal.y%stride,cloud.temporal.y/stride);
    // Dispatch just the current phase of a sparse trace. This visits the
    // same texels as a full dispatch with skipped lanes, without idle groups.
    let texel=select(id.xy*stride+offset,id.xy,cloud.temporal.w!=0u);
    if any(texel>=size) {return;}
    let per_pixel = SURFACE_STOPS;
    let clear_ground = GROUND_TRACE && cloud.stop_mode == CLOUD_STOP_CLEAR_GROUND;
    let block_centre = min(vec2<f32>(texel * 2u + 1u), vec2<f32>(cloud.render_extent) - 0.5);
    let uv = select(cloud_texel_uv(texel), block_centre / vec2<f32>(cloud.render_extent),
                    per_pixel && !clear_ground);
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
    if cloud_deck_active() && cloud.density_profile.y == 0.0 {
        begin = max(begin, GROUND_RAMP_START_M);
    }
    let cloud_interval=vec2(begin,end);
    let fog = ground_interval(camera,ray,cloud.heights.w);
    let fog_interval = vec2(max(fog.x, GROUND_RAMP_START_M), fog.y);
    if fog_interval.x<fog_interval.y {
        begin=select(fog_interval.x,min(begin,fog_interval.x),begin<end);
        end=max(end,fog_interval.y);
    }
    var distances = vec4(end);
    var next_stop = end;
    var stored = array<bool, 4>(false, false, false, false);
    var stops: array<CloudStop, 4>;
    if per_pixel || clear_ground {
        for (var slot = 0u; slot < 4u; slot++) {
            let pixel = min(texel * 2u + cloud_stop_offset(slot), cloud.render_extent - 1u);
            distances[slot] = cloud_stop_distance(pixel, camera);
        }
        // In clear Ground, rays with all surfaces ahead of the whole cloud band
        // can use the Sky march. The other dispatch handles the remaining rays.
        // Each texel is integrated by exactly one of those dispatches.
        if clear_ground {
            let surfaces = distances < vec4(cloud.heights.w);
            let needs_surface_march = any(select(vec4(false), distances >= vec4(begin), surfaces));
            if needs_surface_march != per_pixel { return; }
        }
        end = min(end, max(max(distances.x, distances.y), max(distances.z, distances.w)));
        next_stop = min(min(distances.x, distances.y), min(distances.z, distances.w));
    } else {
        let pixel = vec2<i32>(uv * vec2<f32>(cloud.render_extent));
        let depth = textureLoad(scene_depth, pixel, 0);
        if depth > 0.0 {
            let hit = cloud.inverse_raster * vec4(uv * vec2(2.0,-2.0)+vec2(-1.0,1.0),depth,1.0);
            end = min(end, length(hit.xyz / hit.w - camera));
        }
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
    for(var i=0u;i<cloud.sample_cap && distance<end && total.transmittance*part.transmittance>transmission_floor;i++) {
        let in_cloud=distance>=cloud_interval.x && distance<cloud_interval.y;
        let in_ground=distance>=fog_interval.x && distance<fog_interval.y;
        if !in_cloud && !in_ground {
            let next_cloud=select(end,cloud_interval.x,distance<cloud_interval.x && cloud_interval.x<cloud_interval.y);
            let next_ground=select(end,fog_interval.x,distance<fog_interval.x && fog_interval.x<fog_interval.y);
            distance=min(next_cloud,next_ground);
            continue;
        }
        samples+=1u;
        var step=min(end-distance,max(cloud.heights.z,distance*0.003));
        if in_ground {
            step=min(step,min(fog_interval.y-distance,max(5.0,distance*0.012)));
        } else if distance<fog_interval.x && fog_interval.x<fog_interval.y {
            step=min(step,fog_interval.x-distance);
        }
        if per_pixel && cloud_interval.x < cloud_interval.y {
            if in_cloud {
                step = min(step, cloud_interval.y - distance);
            } else if distance < cloud_interval.x {
                step = min(step, cloud_interval.x - distance);
            }
        }
        let at=distance+step*0.5;
        let point=air_camera+ray*at;
        let world=camera+ray*at;
        let footprint=at/f32(size.y);
        let transmission=total.transmittance*part.transmittance;
        var sigma=0.0;
        var radiance=vec3(0.0);
        if in_cloud && cloud_deck_active() {
            let far = 1.0 - ground_near_weight(at);
            let deck = cloud_deck_light(world, ray, atmosphere.sun_lux.xyz, at, transmission);
            sigma = deck.extinction * far;
            radiance = (deck.ambient + sun_irradiance(world) * deck.surface_visibility * deck.solar_kernel) * far;
        }
        if in_cloud && (!cloud_deck_active() || cloud.density_profile.y > 0.0) {
            let placed=cloud_placed(point,footprint);
            var column=vec4(0.0);
            if !cloud_deck_active() { column=cloud_column(point); }
            if column.x+placed.x>0.0 {
                let upper_sigma=cloud_detail(column.x,point,footprint,cloud.physical.x,column.w)+placed.x;
                sigma+=upper_sigma;
                column.y += placed.y;
                column.z += placed.z;
                if upper_sigma>0.0 {
                    let solar=mix(sun_base,sun_top,clamp((point.y-cloud.heights.x)/(cloud.heights.y-cloud.heights.x),0.0,1.0));
                    let sunlit=any(solar>vec3(0.0));
                    var sun_depths=vec2(0.0);
                    var diffuse=vec3(0.0,0.0,1.0);
                    if sunlit {
                        sun_depths=cloud_local_sun(point,world,at,footprint,sun_alignment,placed.x>0.0);
                        diffuse=vec3(sun_depths,1.0);
                        if placed.x>0.0 && cloud.light_samples>0u {
                            diffuse=cloud_placed_diffuse(point,world,footprint,sun_alignment,sun_depths,placed.x/sigma);
                        }
                    }
                    var source=cloud_source_with_diffuse(solar,phase,sun_depths.x,diffuse.xy,diffuse.z,
                        column.yz,light_angles,at,transmission);
                    if bank>0.0 && placed.x==0.0 && sunlit {
                        source=cloud_bank_light(source,solar,point,sigma,at,footprint,sun_depths.x,sun_alignment,bank);
                    }
                    radiance+=upper_sigma*source;
                }
            }
        }
        if in_ground {
            let far=1.0-ground_near_weight(at);
            if far>0.0 {
                let fog=ground_light(world,ray,atmosphere.sun_lux.xyz,at,transmission,far<1.0);
                if fog.extinction>0.0 {
                    let solar=sun_irradiance(world)*fog.surface_visibility;
                    sigma+=fog.extinction*far;
                    radiance+=(fog.ambient+solar*fog.solar_kernel)*far;
                }
            }
        }
        if sigma>0.0 {
            let segment=cloud_beer(sigma,radiance,step);
            let air=environment_aerial(world,uv).transmission;
            // A part spans at most 0.25 of the lent air's optical depth.
            // Empty gaps require no composition; a distant occupied sample
            // starts a new part before it can be mixed with the nearer one.
            if part_mass>0.0 && any(-log(max(air,vec3(1e-20))/max(part_air,vec3(1e-20)))>=vec3(0.25)) {
                total=cloud_then(total,cloud_air_part(part,part_weighted,part_mass,camera,ray,uv));
                part=CloudTransport(vec3(0.0),1.0,0.0);part_weighted=0.0;part_mass=0.0;
            }
            if part_mass==0.0 {part_air=air;}
            if per_pixel && next_stop < distance + step {
                next_stop = cloud.heights.w;
                let state = CloudTracePart(total, part, weighted, opacity, part_weighted, part_mass);
                for (var slot = 0u; slot < 4u; slot++) {
                    if stored[slot] { continue; }
                    let stop = distances[slot];
                    if stop < distance + step {
                        stops[slot] = cloud_partial_stop(state, sigma, radiance, distance,
                            max(0.0, stop - distance), camera, ray, uv);
                        stored[slot] = true;
                    } else {
                        next_stop = min(next_stop, stop);
                    }
                }
            }
            let local_amount=part.transmittance*cloud_removed(segment.optical_depth);
            let amount=total.transmittance*local_amount;
            weighted+=amount*at;opacity+=amount;
            part_weighted+=local_amount*at;part_mass+=local_amount;
            part=cloud_then(part,segment);
        }
        distance+=step;
    }
    if per_pixel {
        let final_stop = cloud_stop(CloudTracePart(total, part, weighted, opacity, part_weighted, part_mass),
                                    camera, ray, uv);
        for (var slot = 0u; slot < 4u; slot++) {
            var stop = final_stop;
            if stored[slot] { stop = stops[slot]; }
            cloud_write_stop(vec2<i32>(texel), i32(slot), stop, samples, distances[slot] * 0.001);
        }
    } else {
        let front_distance = weighted / max(opacity, 1e-10);
        total = cloud_then(total, cloud_air_part(part, part_weighted, part_mass, camera, ray, uv));
        let value = vec4(total.scattering * environment_storage_scale(), total.transmittance);
        if clear_ground {
            for (var slot = 0u; slot < 4u; slot++) {
                // Depth beyond the trace limit receives the same full transport as sky.
                let reaches_far_edge = distances[slot] >= cloud.heights.w;
                textureStore(transport, vec2<i32>(texel), i32(slot), select(vec4(0.0, 0.0, 0.0, 1.0), value, reaches_far_edge));
                textureStore(front, vec2<i32>(texel), i32(slot),
                    vec4(select(vec2(0.0), vec2(front_distance * 0.001, opacity), reaches_far_edge),
                         f32(samples), distances[slot] * 0.001));
            }
        } else {
            textureStore(transport, vec2<i32>(texel), 0, value);
            textureStore(front, vec2<i32>(texel), 0, vec4(front_distance * 0.001, opacity, f32(samples), 1.0));
        }
    }
}
