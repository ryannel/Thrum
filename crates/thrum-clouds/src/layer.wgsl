@group(2) @binding(0) var<uniform> cloud:CloudParams;
@group(2) @binding(1) var columns:texture_2d_array<f32>;
@group(2) @binding(2) var optical:texture_2d_array<f32>;
@group(2) @binding(3) var transport:texture_storage_2d<rgba16float,write>;
@group(2) @binding(4) var front:texture_storage_2d<rgba16float,write>;
@group(1) @binding(0) var scene_depth:texture_depth_2d;
@group(2) @binding(5) var sun_near:texture_2d_array<f32>;
@group(2) @binding(6) var sun_middle:texture_2d_array<f32>;
@group(2) @binding(7) var sun_far:texture_2d_array<f32>;
struct CloudMapping {region:vec4<f32>,plane_drift:vec4<f32>,path_border:vec4<f32>,far_value:vec4<f32>,near:vec4<f32>,middle:vec4<f32>}
@group(2) @binding(8) var<uniform> mapping:CloudMapping;
@group(2) @binding(11) var shape_noise: texture_3d<f32>;
@group(2) @binding(12) var detail_noise: texture_3d<f32>;
@group(2) @binding(13) var noise_sampler: sampler;

fn cloud_smooth(x:f32)->f32 {let a=clamp(x,0.0,1.0);return a*a*(3.0-2.0*a);}
fn cloud_bilinear(field:texture_2d_array<f32>,coordinate:vec2<f32>,level:i32)->vec4<f32> {
    let size=vec2<i32>(textureDimensions(field));let p=coordinate-0.5;let lo=vec2<i32>(floor(p));let f=fract(p);
    return mix(mix(textureLoad(field,clamp(lo,vec2(0),size-1),level,0),textureLoad(field,clamp(lo+vec2(1,0),vec2(0),size-1),level,0),f.x),mix(textureLoad(field,clamp(lo+vec2(0,1),vec2(0),size-1),level,0),textureLoad(field,clamp(lo+vec2(1),vec2(0),size-1),level,0),f.x),f.y);
}
// Integral of the cubic edge ramp from its clear end to height h.
fn cloud_fade_integral(h:f32,width:f32)->f32 {
    let t=clamp(h/width,0.0,1.0);
    return width*t*t*t*(1.0-0.5*t)+max(0.0,h-width);
}
// Density, above optical depth, below optical depth, and height through the column.
fn cloud_column(point:vec3<f32>)->vec4<f32> {
    var result=vec4(0.0);
    var remaining=1.0;
    // A fully covering fine window replaces every coarser value.
    // Read coarse windows only for the uncovered fraction of the overlap.
    for(var level=0;level<4;level++) {
        let m=cloud.weather[level];let uv=(point.xz-m.xy)/m.z;
        let edge=min(uv,vec2(m.w)-uv);let w=cloud_smooth(min(edge.x,edge.y)/(m.w*0.15));
        if w==0.0 {continue;}
        let c=cloud_bilinear(columns,uv,level);
        let thickness=c.y-c.x;var value=vec4(0.0);
        if thickness>0.0 && point.y>c.x && point.y<c.y {
            let o=cloud_bilinear(optical,uv,level);
            let h=(point.y-c.x)/thickness;
            let vertical=cloud_smooth(h/o.z)*cloud_smooth((1.0-h)/o.w);
            let integral=1.0-(o.z+o.w)*0.5;
            let above=clamp((cloud_fade_integral(1.0-h,o.w)-max(0.0,0.5*o.z-cloud_fade_integral(h,o.z)))/integral,0.0,1.0);
            // Interpolate the profile itself. Interpolating the integral and
            // dividing by thickness again would keep the cloudy texel's full
            // profile as its neighbour thins continuously toward clear air.
            let peak=cloud.physical.x*c.z;
            let tau=peak*thickness*integral;
            value=vec4(peak*vertical,tau*above,tau*(1.0-above),h);
        }
        result+=value*(remaining*w);
        if w==1.0 {break;}
        remaining*=1.0-w;
    }
    return result;
}
fn cloud_density(point:vec3<f32>,footprint:f32)->f32 {
    let placed = cloud_placed(point, footprint);
    let column=cloud_column(point);
    return cloud_detail(column.x,point,footprint,cloud.physical.x,column.w) + placed.x;
}
fn cloud_billow(point:vec3<f32>,footprint:f32)->f32 {
    let uv=point/6400.0+vec3(cloud.noise.x,0.0,cloud.noise.y);
    return textureSampleLevel(shape_noise,noise_sampler,uv,erosion_mip(footprint,50.0)).r;
}
fn cloud_bank_weight()->f32 {
    return smoothstep(0.12,0.45,cloud.physical.z)*(1.0-smoothstep(0.73,0.98,cloud.physical.z));
}
fn cloud_bank_billow(point:vec3<f32>,footprint:f32)->f32 {
    // Smaller lobes break up an otherwise continuous underside. The octave
    // is anchored in the same air coordinates as the parent bank.
    let air=point/6400.0+vec3(cloud.noise.x,0.0,cloud.noise.y);
    return textureSampleLevel(shape_noise,noise_sampler,air*vec3(2.0,3.0,2.0),erosion_mip(2.0*footprint,50.0)).r;
}
fn cloud_detail(sigma:f32,point:vec3<f32>,footprint:f32,extinction:f32,height:f32)->f32 {
    if sigma<=0.0 || extinction<=0.0 || cloud.physical.y==0.0 {return sigma;}
    let detail_uv = point / 800.0 + vec3(cloud.noise.z, 0.0, cloud.noise.w);
    let detail = textureSampleLevel(detail_noise, noise_sampler, detail_uv, erosion_mip(footprint, 12.5)).r;
    let shape=cloud_billow(point,footprint);
    let eroded=cloud_erode(sigma,extinction,cloud.physical.y,shape,detail);
    let bank=cloud_bank_weight();
    if bank==0.0 {return eroded;}
    // The weather map already varies floor height by bank. Keep only
    // smaller scale erosion in the view trace, inside that column.
    let lift=0.22*cloud_bank_billow(point,footprint);
    return eroded*mix(1.0,smoothstep(0.7*lift,lift+0.14,height),bank);
}
fn cloud_sun_depths(world:vec3<f32>,start_height:f32)->vec3<f32> {
    let point=world.xz-mapping.path_border.xz*(world.y-mapping.plane_drift.x)/mapping.path_border.y;
    let heights=vec2(world.y,start_height);
    let near=(point-mapping.near.xy)/mapping.near.z;
    let nw=cloud_smooth(min(min(near.x,near.y),min(1.0-near.x,1.0-near.y))/0.15);
    if nw==1.0 {return cloud_path_sample(sun_near,near,heights);}
    let middle=(point-mapping.middle.xy)/mapping.middle.z;
    let mw=cloud_smooth(min(min(middle.x,middle.y),min(1.0-middle.x,1.0-middle.y))/0.15);
    var tau=vec3(0.0);
    if mw==1.0 {tau=cloud_path_sample(sun_middle,middle,heights);}
    else {
        let region=vec4(mapping.region.xy,mapping.region.z-mapping.region.x,0.0);
        tau=cloud_path_sample(sun_far,clamp((point-region.xy)/region.z,vec2(0.0),vec2(1.0)),heights);
        if mw>0.0 {tau=mix(tau,cloud_path_sample(sun_middle,middle,heights),mw);}
    }
    if nw>0.0 {tau=mix(tau,cloud_path_sample(sun_near,near,heights),nw);}
    return tau;
}

// Six balanced offsets in the plane across the Sun ray.
const SUN_FOOTPRINT = array(
    vec2(1.0,0.0), vec2(-0.5,0.8660254), vec2(-0.5,-0.8660254),
    vec2(-1.0,0.0), vec2(0.5,-0.8660254), vec2(0.5,0.8660254),
);

fn cloud_march_sun_depth(point:vec3<f32>,direction:vec3<f32>,reach:f32,
                         footprint:f32,near_step:f32)->f32 {
    var depth=0.0;
    var distance=0.0;
    // Resolve the nearby lobe, then grow the footprint across distant
    // condensate. The covered distance exceeds the cloud band at grazing Sun.
    for (var segment=0u; segment<56u && distance<reach; segment++) {
        let width=min(reach-distance,max(near_step,0.16*distance));
        depth+=cloud_density(point+direction*(distance+0.5*width),max(footprint,0.5*width))*width;
        distance+=width;
    }
    return depth;
}
fn cloud_local_sun(point:vec3<f32>,world:vec3<f32>,distance:f32,footprint:f32,sun_alignment:f32,placed:bool)->vec2<f32> {
    let samples = cloud.light_samples;
    if samples == 0u { return cloud_sun_depths(world,world.y).xy; }
    let path = mapping.path_border.xyz;
    // At high Sun the local interval is short. Its midpoint resolves the
    // current lobe, while the cumulative path supplies the remaining column.
    // Blend smoothly into full refinement as the Sun approaches grazing.
    let short_column = smoothstep(0.35,0.55,path.y)*select(1.0,0.0,placed);
    if short_column==1.0 {
        let reach=min(120.0,(cloud.heights.y-point.y)/path.y);
        let local=cloud_density(point+path*(0.5*reach),footprint)*reach;
        let handoff=world+path*reach;
        let depths=cloud_sun_depths(handoff,handoff.y);
        let toward_sun=local+depths.x;
        return vec2(toward_sun,max(toward_sun,depths.y));
    }
    // At very low Sun, cached height intervals can cover hundreds of
    // metres along the ray. A close silver lining reveals those intervals.
    // The coarse path cannot resolve the authored volume's internal lobes.
    // Use the same density at both ends of the grazing ray before handing it
    // back to the cached path as the Sun rises.
    let rim_begin = select(0.10, 0.18, placed);
    let rim_end = select(0.17, 0.28, placed);
    let rim_detail=(1.0-smoothstep(rim_begin,rim_end,path.y))*smoothstep(0.75,0.94,sun_alignment)
        *(1.0-smoothstep(10000.0,13500.0,distance));
    var direct=vec2(0.0);
    if rim_detail>0.0 {
        let near_step=select(120.0, 45.0, placed)/f32(samples);
        let outside=cloud_march_sun_depth(point,path,(cloud.heights.y-point.y)/path.y,footprint,near_step);
        let inside=cloud_march_sun_depth(point,-path,(point.y-cloud.heights.x)/path.y,footprint,near_step);
        direct=vec2(outside,outside+inside);
        if rim_detail==1.0 {return direct;}
    }
    // A grazing ray can stay inside the same lobe for hundreds of metres.
    // Resolve that relief in the full density before asking the cached path
    // for everything beyond it. Keep the sample spacing when extending.
    let grazing = 1.0 - smoothstep(0.18, 0.35, path.y);
    let reach = min(mix(120.0, 720.0, grazing), (cloud.heights.y - point.y) / path.y);
    let step = min(reach, 120.0) / f32(samples);
    let tangent = vec3(path.z, 0.0, -path.x);
    let across = tangent * inverseSqrt(max(dot(tangent,tangent),1e-6));
    let up = cross(path, across);
    var local = 0.0;
    for (var j = 0u; j < samples * 6u; j++) {
        let start = f32(j) * step;
        let width = min(step, reach - start);
        if width <= 0.0 { break; }
        // Average across a grazing footprint with the existing taps.
        // Spread less along the ray so nearby gaps stay visible.
        let solar_footprint = max(footprint, 2.0 * width * grazing);
        let distance = start + 0.5 * width;
        let offset = SUN_FOOTPRINT[j % 6u];
        let along = select(-0.23,0.23,j % 2u == 0u);
        let spread = (across * offset.x + up * offset.y + path * along)
            * min(430.0, 175.0 + distance * 0.46) * grazing;
        local += cloud_density(point + path * distance + spread, solar_footprint) * width;
    }
    // In the distance the coarse path drives the complete diffuse column.
    // Replacing part of that column would expose the product's height nodes.
    let near = 1.0 - smoothstep(0.46, 0.59, distance / mapping.near.z);
    let refinement = grazing * near;
    let handoff = world + path * reach;
    let start_height = select(handoff.y,world.y,refinement > 0.0);
    let depths = cloud_sun_depths(handoff,start_height);
    let toward_sun = local + depths.x;
    // At low Sun the removed cached interval can span the whole lobe. Keep
    // the cached depth behind the sample and add it to the refined depth ahead.
    let below = max(0.0, depths.y - depths.z);
    var cached=vec2(toward_sun, mix(max(toward_sun, depths.y), toward_sun + below, refinement));
    if short_column>0.0 {
        let midpoint=cloud_density(point+path*(0.5*reach),footprint)*reach+depths.x;
        cached=mix(cached,vec2(midpoint,max(midpoint,depths.y)),short_column);
    }
    return mix(cached,direct,rim_detail);
}

fn cloud_air_part(part:CloudTransport,weighted:f32,mass:f32,camera:vec3<f32>,ray:vec3<f32>,uv:vec2<f32>)->CloudTransport {
    if mass<=0.0 {return part;}
    let aerial=environment_aerial(camera+ray*(weighted/mass),uv);
    return cloud_with_air(part,aerial.transmission,aerial.scatter);
}

fn cloud_bank_light(source:vec3<f32>,solar:vec3<f32>,point:vec3<f32>,sigma:f32,
                    distance:f32,footprint:f32,sun_depth:f32,sun_alignment:f32,bank:f32)->vec3<f32> {
    // A partly joined bank has side walls around each opening. Ahead of a
    // lit wall the density drops toward the Sun; inside a lobe it stays high.
    let ahead=cloud_density(point+mapping.path_border.xyz*max(170.0,2.0*footprint),footprint);
    let high_sun=smoothstep(0.32,0.66,mapping.path_border.y);
    // Small, thin gaps contribute less direct light when the Sun is high
    // and their walls no longer resolve within the distant pixel footprint.
    let distant=high_sun*smoothstep(4200.0,9500.0,distance);
    let relative=(sigma-ahead)/max(sigma,mix(0.08,0.32,distant)*cloud.physical.x);
    let exposed=smoothstep(-0.18,0.78,relative);
    let beam=exp(-mix(0.28,0.14,high_sun)*sun_depth);
    let interior=exp(-0.085*sun_depth);
    let billow=smoothstep(0.20,0.79,cloud_bank_billow(point,footprint));
    let shade=(0.29+0.36*interior+0.38*exposed)*mix(0.61,1.18,billow);
    // At noon the Sun lights the inward wall of a gap from above. Keep
    // the wider transmission lobe on exposed walls, with a narrower falloff
    // into the bank so the same light does not wash out its whole underside.
    let rim=beam*exposed*exposed*(0.095+0.16*high_sun*exposed)*mix(1.0,0.48,distant);
    let glare=1.0-0.70*smoothstep(0.72,0.96,sun_alignment);
    return mix(source,source*shade*glare+solar*rim,bank);
}

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
    let bank=cloud_bank_weight();
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
