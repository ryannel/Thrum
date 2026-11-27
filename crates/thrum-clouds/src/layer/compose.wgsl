@group(1) @binding(0) var depth:texture_depth_2d;
@group(2) @binding(0) var<uniform> cloud:CloudParams;
@group(2) @binding(1) var transport:texture_2d_array<f32>;
@group(2) @binding(2) var front:texture_2d_array<f32>;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32> {
    let p=array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));return vec4(p[i],0.0,1.0);
}
struct Output {@location(0) colour:vec4<f32>,@location(1) motion:vec4<f32>,@location(2) reactive:f32}
// Interpolate the sky's scattered light and transmittance from layer texel
// centres to render pixel centres. Clamp to the active rectangle at any scale.
fn sky_tap(texel:vec2<i32>, size:vec2<i32>)->vec4<f32> {
    let at=clamp(texel,vec2(0),size-1);
    return textureLoad(transport,at,0,0);
}

fn sky_transport(uv:vec2<f32>)->vec4<f32> {
    let size=vec2<i32>(cloud.active_extent);
    let p=uv*vec2<f32>(size)-0.5;
    let base=vec2<i32>(floor(p));
    let f=fract(p);
    let a=sky_tap(base,size);
    let b=sky_tap(base+vec2(1,0),size);
    let c=sky_tap(base+vec2(0,1),size);
    let d=sky_tap(base+vec2(1),size);
    return mix(mix(a,b,f.x),mix(c,d,f.x),f.y);
}
// Interpolate shared ray directions at a continuous opaque surface. Use
// the nearest stop in each block and reject silhouettes and distant surfaces.
fn surface_transport(pixel: vec2<u32>, uv: vec2<f32>, distance: f32, own: vec4<f32>) -> vec4<f32> {
    let grid = cloud_ground_grid(uv);
    let base = vec2<i32>(floor(grid));
    let blend = fract(grid);
    var value = vec4(0.0);
    var total = 0.0;
    for (var slot = 0u; slot < 4u; slot++) {
        let corner = cloud_stop_offset(slot);
        let block = clamp(base + vec2<i32>(corner), vec2(0), vec2<i32>(cloud.active_extent) - 1);
        let block_pixel = block * 2;
        let neighbor = min(vec2<u32>(block_pixel + clamp(vec2<i32>(pixel) - block_pixel,
                                  vec2(0), vec2(1))), cloud.render_extent - 1u);
        let neighbor_slot = cloud_stop_slot(neighbor);
        let stop = textureLoad(front, block, neighbor_slot, 0).w;
        if stop == 0.0 { continue; }
        let relative_delta = abs(distance - stop) / max(distance, 0.001);
        let axis = mix(1.0 - blend, blend, vec2<f32>(corner));
        let weight = axis.x * axis.y * (1.0 - smoothstep(0.006, 0.025, relative_delta));
        if weight > 0.0 {
            var sample = textureLoad(transport, block, neighbor_slot, 0);
            // Estimate this ray's transport at the receiver's stop. Selecting
            // the nearest subpixel alone alternates an early and a late stop
            // on successive rows, even after interpolating the ray directions.
            if distance != stop {
                let adjacent_slot = neighbor_slot ^ 2;
                let adjacent = textureLoad(front, block, adjacent_slot, 0).w;
                let span = adjacent - stop;
                if adjacent > 0.0 && abs(span) > 1e-6 && abs(span) < 0.025 * stop {
                    let other = textureLoad(transport, block, adjacent_slot, 0);
                    let phase = clamp((distance - stop) / span, -1.0, 1.0);
                    sample = max(vec4(0.0), mix(sample, other, phase));
                    sample.a = min(sample.a, 1.0);
                }
            }
            value += weight * sample;
            total += weight;
        }
    }
    return select(own, value / max(total, 1e-8), total > 0.0);
}

@fragment fn fragment(@builtin(position) position:vec4<f32>)->Output {
    let uv=position.xy/aux.extent.xy;
    let pixel = vec2<u32>(position.xy);
    let per_pixel = cloud.per_pixel_stops != 0u;
    let opaque_depth = textureLoad(depth, vec2<i32>(pixel), 0);
    let filter_sky = opaque_depth == 0.0;
    let legacy_texel = vec2<u32>(clamp(uv * vec2<f32>(cloud.active_extent),
                                      vec2(0.0), vec2<f32>(cloud.active_extent) - 1.0));
    let at = vec2<i32>(select(legacy_texel, pixel / 2u, per_pixel));
    let slot = select(0, cloud_stop_slot(pixel), per_pixel);
    let front_value = textureLoad(front, at, slot, 0);
    var layer: vec4<f32>;
    if filter_sky {
        if per_pixel {
            layer = cloud_ground_sky(transport, front, uv);
        } else {
            layer = sky_transport(uv);
        }
        if front_value.y <= 0.0 {
            if layer.a >= 1.0 { discard; }
            return Output(layer, vec4(0.0), 0.0);
        }
    } else {
        if front_value.y <= 0.0 { discard; }
        layer = textureLoad(transport, at, slot, 0);
        if per_pixel {
            layer = surface_transport(pixel, uv, front_value.w, layer);
        }
    }
    let far=cloud.inverse_raster*vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),0.0,1.0);
    let ray=normalize(far.xyz/far.w-view.camera.xyz);
    let point=view.camera.xyz+ray*front_value.x*1000.0;
    let raster=view.raster*vec4(point,1.0);
    // Reversed-Z zero denotes sky. Clouds may extend past the opaque
    // camera's far plane, where their projected depth is negative.
    if !per_pixel && opaque_depth>0.0 && opaque_depth>raster.z/raster.w {discard;}
    var motion=vec2(65504.0);
    if cloud.temporal.x!=0u {
        let old_point=point-vec3(cloud.drift.z,0.0,cloud.drift.w);
        let previous=view.previous*vec4(old_point,1.0);
        let current=view.unjittered*vec4(point,1.0);
        if previous.w>0.0 && current.w>0.0 {motion=(previous.xy/previous.w-current.xy/current.w)*vec2(0.5,-0.5)*aux.extent.xy;}
    }
    let reactive=select(front_value.z,front_value.y,!per_pixel && cloud.temporal.w!=0u);
    return Output(layer,vec4(motion,0.0,select(0.0,1.0,front_value.y>0.5)),reactive);
}
