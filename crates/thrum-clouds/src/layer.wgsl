struct CloudParams {
    weather:array<vec4<f32>,4>, physical:vec4<f32>, erosion:vec4<f32>,
    drift:vec4<f32>, heights:vec4<f32>, settings:vec4<u32>,
}
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

fn cloud_smooth(x:f32)->f32 {let a=clamp(x,0.0,1.0);return a*a*(3.0-2.0*a);}
fn cloud_hash(i:u32,j:u32,g:u32,stream:u32)->f32 {
    var x=i*0x9e3779b1u+j*0x85ebca77u+g*0xc2b2ae3du+(stream+131u*cloud.settings.x)*0x27d4eb2fu;
    x=x^(x>>15u);x=x*0x2c1b3c6du;x=x^(x>>12u);x=x*0x297a2d39u;x=x^(x>>15u);
    return f32(x)*0.00000000023283064365386963;
}
fn cloud_noise(point:vec3<f32>,scale:f32,stream:u32)->f32 {
    let p=point/scale;let lo=vec3<i32>(floor(p));let f=vec3(cloud_smooth(fract(p.x)),cloud_smooth(fract(p.y)),cloud_smooth(fract(p.z)));
    var n=0.0;
    for(var x=0;x<=1;x++){for(var y=0;y<=1;y++){for(var z=0;z<=1;z++){
        let w=select(vec3(1.0)-f,f,vec3<bool>(x!=0,y!=0,z!=0));
        n+=w.x*w.y*w.z*cloud_hash(u32(lo.x+x),u32(lo.y+y),u32(lo.z+z),stream);
    }}}return n;
}
fn cloud_bilinear(field:texture_2d_array<f32>,coordinate:vec2<f32>,level:i32)->vec4<f32> {
    let size=vec2<i32>(textureDimensions(field));let p=coordinate-0.5;let lo=vec2<i32>(floor(p));let f=fract(p);
    return mix(mix(textureLoad(field,clamp(lo,vec2(0),size-1),level,0),textureLoad(field,clamp(lo+vec2(1,0),vec2(0),size-1),level,0),f.x),mix(textureLoad(field,clamp(lo+vec2(0,1),vec2(0),size-1),level,0),textureLoad(field,clamp(lo+vec2(1),vec2(0),size-1),level,0),f.x),f.y);
}
// Density, above optical depth, below optical depth, local cloud thickness.
fn cloud_column(point:vec3<f32>)->vec4<f32> {
    var result=vec4(0.0);
    for(var level=3;level>=0;level--) {
        let m=cloud.weather[level];let uv=(point.xz-m.xy)/m.z;
        let edge=min(uv,vec2(m.w)-uv);let w=cloud_smooth(min(edge.x,edge.y)/(m.w*0.15));
        if w==0.0 {continue;}
        let c=cloud_bilinear(columns,uv,level);let o=cloud_bilinear(optical,uv,level);
        let thickness=c.y-c.x;var value=vec4(0.0);
        if thickness>0.0 && point.y>c.x && point.y<c.y {
            let h=(point.y-c.x)/thickness;
            let vertical=cloud_smooth(h/o.z)*(1.0-cloud_smooth((h-(1.0-o.w))/o.w));
            value=vec4(o.y/thickness*vertical/(1.0-(o.z+o.w)*0.5),o.y*(1.0-h),o.y*h,thickness);
        }
        result=mix(result,value,w);
    }
    return result;
}
fn cloud_density(point:vec3<f32>,footprint:f32)->f32 {
    let sigma=cloud_column(point).x;
    if sigma<=0.0 || cloud.physical.x<=0.0 || cloud.physical.y==0.0 {return sigma;}
    let signal=min(1.0,sigma/cloud.physical.x);
    let shape=mix(cloud_noise(point,200.0,40u),0.5,clamp((footprint-100.0)/300.0,0.0,1.0));
    let detail=mix(cloud_noise(point,50.0,41u),cloud.physical.z,clamp(footprint/100.0,0.0,1.0));
    let threshold=cloud.physical.y*shape;
    let shaped=max(0.0,(signal-threshold)/(1.0-threshold));
    let eroded=max(0.0,shaped-cloud.physical.y*detail*(1.0-shaped));
    return sigma*min(1.0,eroded/signal);
}
fn cloud_hg(c:f32,g:f32)->f32 {return (1.0-g*g)/(4.0*PI*pow(1.0+g*g-2.0*g*c,1.5));}

fn cloud_path_sample(field:texture_2d_array<f32>,uv:vec2<f32>,height:f32)->f32 {
    let count=textureNumLayers(field)*4u;
    let q=clamp((cloud.heights.y-height)/(cloud.heights.y-cloud.heights.x),0.0,1.0)*f32(count-1u);
    let lo=u32(floor(q));let hi=min(count-1u,lo+1u);
    let pixel=uv*vec2<f32>(textureDimensions(field));
    return mix(cloud_bilinear(field,pixel,i32(lo/4u))[lo%4u],cloud_bilinear(field,pixel,i32(hi/4u))[hi%4u],fract(q));
}
fn cloud_sun_tau(world:vec3<f32>)->f32 {
    if world.y>=cloud.heights.y {return 0.0;}
    let point=world.xz-mapping.path_border.xz*(world.y-mapping.plane_drift.x)/mapping.path_border.y;
    let region=vec4(mapping.region.xy,mapping.region.z-mapping.region.x,0.0);
    var tau=cloud_path_sample(sun_far,clamp((point-region.xy)/region.z,vec2(0.0),vec2(1.0)),world.y);
    let middle=(point-mapping.middle.xy)/mapping.middle.z;
    let mw=cloud_smooth(min(min(middle.x,middle.y),min(1.0-middle.x,1.0-middle.y))/0.15);
    if mw>0.0 {tau=mix(tau,cloud_path_sample(sun_middle,middle,world.y),mw);}
    let near=(point-mapping.near.xy)/mapping.near.z;
    let nw=cloud_smooth(min(min(near.x,near.y),min(1.0-near.x,1.0-near.y))/0.15);
    if nw>0.0 {tau=mix(tau,cloud_path_sample(sun_near,near,world.y),nw);}
    return tau;
}

@compute @workgroup_size(8,8)
fn trace(@builtin(global_invocation_id) id:vec3<u32>) {
    let size=textureDimensions(transport);if any(id.xy>=size) {return;}
    let uv=(vec2<f32>(id.xy)+0.5)/vec2<f32>(size);
    let clip=atmosphere.inverse_view_projection*vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),0.0,1.0);
    let camera=atmosphere.camera_altitude.xyz;let ray=normalize(clip.xyz/clip.w-camera);
    let air_camera=camera-vec3(cloud.drift.x,0.0,cloud.drift.y);
    var radiance=vec3(0.0);var transmission=1.0;var weighted=0.0;var opacity=0.0;
    var begin=0.0;var end=cloud.heights.w;
    if abs(ray.y)>1e-7 {
        let a=(cloud.heights.x-camera.y)/ray.y;let b=(cloud.heights.y-camera.y)/ray.y;
        begin=max(0.0,min(a,b));end=min(end,max(a,b));
    }else if camera.y<cloud.heights.x || camera.y>cloud.heights.y {end=0.0;}
    // A conservative depth stop; composition tests each render pixel separately.
    let pixel=vec2<i32>(uv*vec2<f32>(textureDimensions(scene_depth)));
    let depth=textureLoad(scene_depth,pixel,0);
    if depth>0.0 {
        let hit=atmosphere.inverse_view_projection*vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),depth,1.0);
        end=min(end,length(hit.xyz/hit.w-camera));
    }
    let cosine=dot(ray,atmosphere.sun_lux.xyz);
    let phase=vec4(cloud_hg(cosine,0.85),cloud_hg(cosine,0.43),cloud_hg(cosine,0.215),cloud_hg(cosine,0.1075));
    let weights=vec4(0.7,0.2,0.07,0.03);let branches=vec4(1.0,0.5,0.25,0.125);
    let up=environment_diffuse(vec3(0.0,1.0,0.0))/PI;
    let down=environment_diffuse(vec3(0.0,-1.0,0.0))/PI;
    let sun_base=sun_irradiance(vec3(camera.x,cloud.heights.x,camera.z));
    let sun_top=sun_irradiance(vec3(camera.x,cloud.heights.y,camera.z));
    var distance=begin;
    for(var i=0u;i<768u && distance<end && transmission>0.001;i++) {
        let step=min(end-distance,max(cloud.heights.z,distance*0.002));
        let at=distance+step*0.5;let point=air_camera+ray*at;
        let column=cloud_column(point);
        if column.x>0.0 {
            let sigma=cloud_density(point,at/f32(size.y));
            if sigma>0.0 {
                let sun=atmosphere.sun_lux.xyz;
                let world=camera+ray*at;
                var tau=cloud_sun_tau(world);
                if cloud.settings.y>0u {
                    let reach=min(600.0,(cloud.heights.y-point.y)/max(0.1,sun.y));
                    var local=0.0;
                    for(var j=0u;j<cloud.settings.y;j++) {
                        local+=cloud_density(point+sun*((f32(j)+0.5)*reach/f32(cloud.settings.y)),at/f32(size.y))*reach/f32(cloud.settings.y);
                    }
                    tau=local+cloud_sun_tau(world+sun*reach);
                }
                let solar=mix(sun_base,sun_top,clamp((point.y-cloud.heights.x)/(cloud.heights.y-cloud.heights.x),0.0,1.0));
                let ambient=up/(1.0+0.1125*column.y)+down/(1.0+0.1125*column.z);
                let source=solar*dot(weights*phase,exp(-branches*tau))+ambient;
                let step_t=exp(-sigma*step);let amount=transmission*(1.0-step_t);
                radiance+=amount*source;weighted+=amount*at;opacity+=amount;transmission*=step_t;
            }
        }
        distance+=step;
    }
    let front_distance=weighted/max(opacity,1e-10);
    if opacity>0.0 {
        let aerial=environment_aerial(camera+ray*front_distance,uv);
        radiance=(aerial.transmission*radiance+opacity*aerial.scatter)*environment_storage_scale();
    }
    textureStore(transport,vec2<i32>(id.xy),vec4(radiance,transmission));
    // Distances are kilometres to remain representable in fp16.
    textureStore(front,vec2<i32>(id.xy),vec4(front_distance*0.001,opacity,0.0,0.0));
}
