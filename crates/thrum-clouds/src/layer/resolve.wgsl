@group(2) @binding(0) var<uniform> cloud:CloudParams;
@group(2) @binding(1) var fresh:texture_2d<f32>;
@group(2) @binding(2) var fresh_front:texture_2d<f32>;
@group(2) @binding(3) var previous:texture_2d<f32>;
@group(2) @binding(4) var previous_front:texture_2d<f32>;
@group(2) @binding(5) var next:texture_storage_2d<rgba16float,write>;
@group(2) @binding(6) var next_front:texture_storage_2d<rgba16float,write>;

struct CloudFresh {
    transport: vec4<f32>,
    front: f32,
    opacity: f32,
    low: vec4<f32>,
    high: vec4<f32>,
}

fn cloud_fresh(at: vec2<i32>, offset: vec2<i32>, stride: i32, traced: bool) -> CloudFresh {
    if traced {
        let value = textureLoad(fresh, at, 0);
        let front = textureLoad(fresh_front, at, 0);
        return CloudFresh(value, front.x, front.y, value, value);
    }

    // Reconstruct on the lattice of this frame's actual traced texels.
    // A block's single sample replicated over the block moves its contour
    // every time the refresh phase changes, most visibly at disocclusions.
    let grid = vec2<f32>(at - offset) / f32(stride);
    let last = (vec2<i32>(cloud.active_extent) - 1 - offset) / stride;
    let base = vec2<i32>(floor(grid));
    let lo = clamp(base, vec2(0), last) * stride + offset;
    let hi = clamp(base + 1, vec2(0), last) * stride + offset;
    let p10 = vec2(hi.x, lo.y);
    let p01 = vec2(lo.x, hi.y);
    let f = fract(grid);
    let v00 = textureLoad(fresh, lo, 0);
    let v10 = textureLoad(fresh, p10, 0);
    let v01 = textureLoad(fresh, p01, 0);
    let v11 = textureLoad(fresh, hi, 0);
    let value = mix(mix(v00, v10, f.x), mix(v01, v11, f.x), f.y);

    // Clear samples contribute transport but have no front to interpolate.
    let front_weights = vec4((1.0 - f.x) * (1.0 - f.y), f.x * (1.0 - f.y),
                             (1.0 - f.x) * f.y, f.x * f.y)
        * (1.0 - vec4(v00.a, v10.a, v01.a, v11.a));
    let fronts = vec4(textureLoad(fresh_front, lo, 0).x,
                      textureLoad(fresh_front, p10, 0).x,
                      textureLoad(fresh_front, p01, 0).x,
                      textureLoad(fresh_front, hi, 0).x);
    let opacity = dot(front_weights, vec4(1.0));
    let distance = dot(fronts, front_weights) / select(opacity, 1.0, opacity == 0.0);
    return CloudFresh(value, distance, opacity, min(min(v00, v10), min(v01, v11)),
                     max(max(v00, v10), max(v01, v11)));
}
fn cloud_history(field:texture_2d<f32>,uv:vec2<f32>)->vec4<f32> {
    let size=vec2<i32>(cloud.active_extent);let at=uv*vec2<f32>(size)-0.5;
    let lo=vec2<i32>(floor(at));let f=fract(at);
    return mix(mix(textureLoad(field,clamp(lo,vec2(0),size-1),0),textureLoad(field,clamp(lo+vec2(1,0),vec2(0),size-1),0),f.x),mix(textureLoad(field,clamp(lo+vec2(0,1),vec2(0),size-1),0),textureLoad(field,clamp(lo+vec2(1),vec2(0),size-1),0),f.x),f.y);
}
@compute @workgroup_size(8,8)
fn resolve(@builtin(global_invocation_id) id:vec3<u32>) {
    let size=cloud.active_extent;if any(id.xy>=size) {return;}
    let at=vec2<i32>(id.xy);
    let stride = cloud.temporal.z;
    let offset = vec2(cloud.temporal.y % stride, cloud.temporal.y / stride);
    let traced = cloud.temporal.w != 0u || all(id.xy % stride == offset);
    let sample = cloud_fresh(at, vec2<i32>(offset), i32(stride), traced);
    var value = sample.transport;
    var reactive=1.0-value.a;
    var age=0.0;
    if cloud.temporal.x!=0u {
        let uv=(vec2<f32>(id.xy)+0.5)/vec2<f32>(size);
        let far=cloud.inverse_raster*vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),0.0,1.0);
        let ray=normalize(far.xyz/far.w-view.camera.xyz);
        let point=view.camera.xyz+ray*sample.front*1000.0;
        let old_point=point-vec3(cloud.drift.z,0.0,cloud.drift.w);
        let clip=cloud.previous_raster*vec4(old_point,1.0);
        let old_uv=clip.xy/clip.w*vec2(0.5,-0.5)+0.5;
        if clip.w>0.0 && all(old_uv>=vec2(0.0)) && all(old_uv<=vec2(1.0)) {
            let old=cloud_history(previous,old_uv);
            let old_front=cloud_history(previous_front,old_uv);
            // A vanished front is a disocclusion, never a carried opaque cloud.
            if old_front.y>0.0 && sample.opacity>0.0 {
                if !traced {
                    value=clamp(old,sample.low,sample.high);
                    age=min(old_front.w+1.0, f32(stride*stride-1u));
                }
                reactive=abs(value.a-old.a)*(1.0-value.a);
            }
        }
    }
    textureStore(next,at,value);
    textureStore(next_front,at,vec4(sample.front,1.0-value.a,reactive,age));
}
