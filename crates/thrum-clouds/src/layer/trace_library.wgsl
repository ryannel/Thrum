@group(2) @binding(11) var shape_noise: texture_3d<f32>;
@group(2) @binding(12) var detail_noise: texture_3d<f32>;

fn cloud_density(point:vec3<f32>,footprint:f32)->f32 {
    let placed = cloud_placed(point, footprint);
    let column=cloud_column(point);
    return cloud_detail(column.x,point,footprint,cloud.physical.x,column.w) + placed.x;
}
fn cloud_detail(sigma:f32,point:vec3<f32>,footprint:f32,extinction:f32,height:f32)->f32 {
    if sigma<=0.0 || extinction<=0.0 || cloud.physical.y==0.0 {return sigma;}
    let detail_uv = point / 800.0 + vec3(cloud.noise.z, 0.0, cloud.noise.w);
    let detail = textureSampleLevel(detail_noise, noise_sampler, detail_uv, erosion_mip(footprint, 12.5)).r;
    return cloud_eroded_density(sigma,point,footprint,extinction,height,detail);
}

fn cloud_sun_depths(world:vec3<f32>,start_height:f32)->vec3<f32> {
    return cloud_window_depths(world,start_height,cloud.heights.xy);
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
    let rim_detail=(1.0-smoothstep(0.10,0.17,path.y))*smoothstep(0.75,0.94,sun_alignment)
        *(1.0-smoothstep(10000.0,13500.0,distance));
    // Authored lobes can be narrower than a Sun-path column even at noon.
    // Resolve their column on the same density as the view ray, including
    // the unlit side. Blend back to the cached path beyond the placed range.
    let placed_detail=select(0.0,1.0-smoothstep(19000.0,32000.0,distance),placed);
    let direct_weight=max(rim_detail,placed_detail);
    var direct=vec2(0.0);
    if direct_weight>0.0 {
        let near_step=select(120.0, 66.0, placed)/f32(samples);
        let outside=cloud_march_sun_depth(point,path,(cloud.heights.y-point.y)/path.y,footprint,near_step);
        let inside=cloud_march_sun_depth(point,-path,(point.y-cloud.heights.x)/path.y,footprint,near_step);
        direct=vec2(outside,outside+inside);
        if direct_weight==1.0 {return direct;}
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
    return mix(cached,direct,direct_weight);
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
    let billow=smoothstep(0.20,0.79,
        cloud_bank_billow(shape_noise,noise_sampler,cloud_shape_uv(point),footprint));
    let shade=(mix(0.29,0.19,high_sun)+0.36*interior+mix(0.38,0.29,high_sun)*exposed)
        *mix(0.61,1.18,billow);
    // At noon the Sun lights the inward wall of a gap from above. Keep
    // the wider transmission lobe on exposed walls, with a narrower falloff
    // into the bank so the same light does not wash out its whole underside.
    let edge_light=1.0-smoothstep(0.075,0.43,sigma/cloud.physical.x);
    let rim=beam*exposed*exposed*(0.095+0.21*high_sun*exposed*edge_light)*mix(1.0,0.48,distant);
    let glare=1.0-0.70*smoothstep(0.72,0.96,sun_alignment);
    return mix(source,source*shade*glare+solar*rim,bank);
}
