@group(2) @binding(0) var<uniform> cloud:CloudParams;
@group(2) @binding(1) var fresh:texture_2d<f32>;
@group(2) @binding(2) var fresh_front:texture_2d<f32>;
@group(2) @binding(3) var previous:texture_2d<f32>;
@group(2) @binding(4) var previous_front:texture_2d<f32>;
@group(2) @binding(5) var next:texture_storage_2d<rgba16float,write>;
@group(2) @binding(6) var next_front:texture_storage_2d<rgba16float,write>;

fn cloud_fresh_at(pixel:vec2<u32>)->vec2<i32> {
    let size=cloud.active_extent;
    let stride=cloud.temporal.z;
    let offset=vec2(cloud.temporal.y%stride,cloud.temporal.y/stride);
    let block=min(pixel/stride,(size-1u-offset)/stride);
    return vec2<i32>(block*stride+offset);
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
    let fresh_at=select(cloud_fresh_at(id.xy),at,cloud.temporal.w!=0u);
    var value=textureLoad(fresh,fresh_at,0);
    let front_value=textureLoad(fresh_front,fresh_at,0);
    var reactive=1.0-value.a;
    var age=0.0;
    if cloud.temporal.x!=0u {
        let uv=(vec2<f32>(id.xy)+0.5)/vec2<f32>(size);
        let far=cloud.inverse_raster*vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),0.0,1.0);
        let ray=normalize(far.xyz/far.w-view.camera.xyz);
        let point=view.camera.xyz+ray*front_value.x*1000.0;
        let old_point=point-vec3(cloud.drift.z,0.0,cloud.drift.w);
        let clip=cloud.previous_raster*vec4(old_point,1.0);
        let old_uv=clip.xy/clip.w*vec2(0.5,-0.5)+0.5;
        if clip.w>0.0 && all(old_uv>=vec2(0.0)) && all(old_uv<=vec2(1.0)) {
            let old=cloud_history(previous,old_uv);
            let old_front=cloud_history(previous_front,old_uv);
            // A vanished front is a disocclusion, never a carried opaque cloud.
            if old_front.y>0.0 && front_value.y>0.0 {
                let stride=cloud.temporal.z;
                let offset=vec2(cloud.temporal.y%stride,cloud.temporal.y/stride);
                let fresh_pixel=cloud.temporal.w!=0u || all(id.xy%stride==offset);
                if !fresh_pixel {
                    var low=value;var high=value;
                    for(var y=-1;y<=1;y++){for(var x=-1;x<=1;x++){
                        let neighbor=vec2<u32>(clamp(at+vec2(x,y)*i32(stride),vec2(0),vec2<i32>(size)-1));
                        let sample=textureLoad(fresh,cloud_fresh_at(neighbor),0);
                        low=min(low,sample);high=max(high,sample);
                    }}
                    value=clamp(old,low,high);
                    age=min(old_front.w+1.0, f32(stride*stride-1u));
                }
                reactive=abs(value.a-old.a)*(1.0-value.a);
            }
        }
    }
    textureStore(next,at,value);
    textureStore(next_front,at,vec4(front_value.x,1.0-value.a,reactive,age));
}
