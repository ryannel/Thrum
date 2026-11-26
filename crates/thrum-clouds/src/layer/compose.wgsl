@group(1) @binding(0) var depth:texture_depth_2d;
@group(2) @binding(0) var<uniform> cloud:CloudParams;
@group(2) @binding(1) var transport:texture_2d<f32>;
@group(2) @binding(2) var front:texture_2d<f32>;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32> {
    let p=array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));return vec4(p[i],0.0,1.0);
}
struct Output {@location(0) colour:vec4<f32>,@location(1) motion:vec4<f32>,@location(2) reactive:f32}
// Interpolate the sky's scattered light and transmittance from layer texel
// centres to render pixel centres. Clamp to the active rectangle at any scale.
fn sky_transport(uv:vec2<f32>)->vec4<f32> {
    let size=vec2<i32>(cloud.active_extent);
    let p=uv*vec2<f32>(size)-0.5;
    let base=vec2<i32>(floor(p));
    let f=fract(p);
    let a=textureLoad(transport,clamp(base,vec2(0),size-1),0);
    let b=textureLoad(transport,clamp(base+vec2(1,0),vec2(0),size-1),0);
    let c=textureLoad(transport,clamp(base+vec2(0,1),vec2(0),size-1),0);
    let d=textureLoad(transport,clamp(base+vec2(1),vec2(0),size-1),0);
    return mix(mix(a,b,f.x),mix(c,d,f.x),f.y);
}
@fragment fn fragment(@builtin(position) position:vec4<f32>)->Output {
    let uv=position.xy/aux.extent.xy;
    let at=vec2<i32>(clamp(uv*vec2<f32>(cloud.active_extent),vec2(0.0),vec2<f32>(cloud.active_extent)-1.0));
    let front_value=textureLoad(front,at,0);
    let field=cloud.field_overlay;
    let panel_uv=(uv-vec2(0.025))/vec2(0.30,0.42);
    if field!=0u && all(panel_uv>=vec2(0.0)) && all(panel_uv<vec2(1.0)) {
        return Output(field_colour(field,panel_uv),vec4(0.0,0.0,0.0,1.0),1.0);
    }
    let opaque_depth=textureLoad(depth,vec2<i32>(position.xy),0);
    var layer:vec4<f32>;
    if opaque_depth==0.0 {
        layer=sky_transport(uv);
        if front_value.y<=0.0 {
            if layer.a>=1.0 {discard;}
            return Output(layer,vec4(0.0),0.0);
        }
    } else {
        // Use the trace's own stop and front when testing against geometry.
        if front_value.y<=0.0 {discard;}
        layer=textureLoad(transport,at,0);
    }
    let far=cloud.inverse_raster*vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),0.0,1.0);
    let ray=normalize(far.xyz/far.w-view.camera.xyz);
    let point=view.camera.xyz+ray*front_value.x*1000.0;
    let raster=view.raster*vec4(point,1.0);
    // Reversed-Z zero denotes sky. Clouds may extend past the opaque
    // camera's far plane, where their projected depth is negative.
    if opaque_depth>0.0 && opaque_depth>raster.z/raster.w {discard;}
    var motion=vec2(65504.0);
    if cloud.temporal.x!=0u {
        let old_point=point-vec3(cloud.drift.z,0.0,cloud.drift.w);
        let previous=view.previous*vec4(old_point,1.0);
        let current=view.unjittered*vec4(point,1.0);
        if previous.w>0.0 && current.w>0.0 {motion=(previous.xy/previous.w-current.xy/current.w)*vec2(0.5,-0.5)*aux.extent.xy;}
    }
    let reactive=select(front_value.z,front_value.y,cloud.temporal.w!=0u);
    return Output(layer,vec4(motion,0.0,select(0.0,1.0,front_value.y>0.5)),reactive);
}
