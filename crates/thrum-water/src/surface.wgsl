struct ViewRecord {
    view_proj: mat4x4<f32>, previous_view_proj: mat4x4<f32>,
    camera_xyz_time: vec4<f32>, pre_exposure_inv_exposure_jitter: vec4<f32>,
    unjittered_view_proj: mat4x4<f32>, mip_bias: f32, jitter_phase: u32, padding: vec2<u32>,
}
struct ViewAuxRecord {
    inverse_view_proj: mat4x4<f32>, viewport_inv_viewport: vec4<f32>,
    near_far: vec4<f32>, time: vec4<f32>, flags: vec4<u32>, subrectangle: vec4<f32>,
}
@group(0) @binding(0) var<uniform> view: ViewRecord;
@group(0) @binding(1) var<uniform> view_aux: ViewAuxRecord;
struct Parameters { level_dt_reset: vec4<f32>, gains: vec4<f32>, lengths: vec4<f32>, cache: array<vec4<f32>,8>, scatter:vec4<f32>, }
@group(3) @binding(0) var<uniform> water: Parameters;
struct Field { displacement:vec4<f32>, slope:vec4<f32>, velocity:vec4<f32>, }
@group(3) @binding(1) var<storage,read> cascades:array<Field>;
@group(3) @binding(2) var<storage,read> long_field:array<Field>;
struct Moments { a:vec4<f32>, b:vec4<f32>, }
@group(3) @binding(3) var<storage,read> moments4:array<Moments>;
@group(3) @binding(4) var<storage,read> moments8:array<Moments>;
@group(3) @binding(5) var<storage,read> moments0:array<Moments>;
fn add_field(a:Field,b:Field,w:f32)->Field {return Field(a.displacement+w*b.displacement,a.slope+w*b.slope,a.velocity+w*b.velocity);}
fn cascade_at(point:vec2<f32>,band:u32)->Field {
    let q=fract(point/water.lengths[band])*256.0-0.5;
    let base=vec2<i32>(floor(q)); let t=fract(q);
    var result=Field(vec4(0.0),vec4(0.0),vec4(0.0));
    for(var j=0;j<2;j++){for(var i=0;i<2;i++){
        let at=vec2<u32>((base+vec2(i,j)+vec2(512))%vec2(256));
        let weight=select(1.0-t.x,t.x,i==1)*select(1.0-t.y,t.y,j==1);
        result=add_field(result,cascades[band*65536u+at.y*256u+at.x],weight);
    }}
    return result;
}
fn long_level(point:vec2<f32>,level:u32)->Field {
    let q=clamp((point-water.cache[level].xy)/water.cache[level].z-0.5,vec2(0.0),vec2(126.999));
    let base=vec2<u32>(floor(q));let t=fract(q);
    var result=Field(vec4(0.0),vec4(0.0),vec4(0.0));
    for(var j=0u;j<2u;j++){for(var i=0u;i<2u;i++){
        let weight=select(1.0-t.x,t.x,i==1u)*select(1.0-t.y,t.y,j==1u);
        result=add_field(result,long_field[4096u+level*16384u+(base.y+j)*128u+base.x+i],weight);
    }}
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
fn moment_at(point:vec2<f32>,band:u32,level:u32)->Moments {
    let width=256u>>level;
    let q=fract(point/water.lengths[band])*f32(width)-0.5;
    let base=vec2<i32>(floor(q));let t=fract(q);
    var result=Moments(vec4(0.0),vec4(0.0));
    for(var j=0;j<2;j++){for(var i=0;i<2;i++){
        let at=vec2<u32>((base+vec2(i,j)+vec2(i32(width)*2))%vec2(i32(width)));
        let index=band*width*width+at.y*width+at.x;
        var m=Moments(vec4(0.0),vec4(0.0));
        if level==0u {m=moments0[index];}else if level==4u {m=moments4[index];}else {m=moments8[index];}
        let w=select(1.0-t.x,t.x,i==1)*select(1.0-t.y,t.y,j==1);
        result=Moments(result.a+w*m.a,result.b+w*m.b);
    }}
    return result;
}
fn field_at(point:vec2<f32>,spacing:f32)->Field {
    // The cache has already inverted the Gerstner map at this world point.
    var long=long_at(point,spacing);long.displacement.x=0.0;long.displacement.z=0.0;
    var f=add_field(Field(vec4(0.0),vec4(0.0),vec4(0.0)),long,water.gains.x);
    for(var band=0u;band<3u;band++) {
        let fade=1.0-smoothstep(water.lengths[band]/128.0,water.lengths[band]/32.0,spacing);
        f=add_field(f,cascade_at(point,band),water.gains[band+1u]*fade);
    }
    return f;
}
struct VertexOut {
    @builtin(position) clip:vec4<f32>, @location(0) world:vec3<f32>,
    @location(1) rest:vec2<f32>, @location(2) velocity:vec3<f32>,
    @location(3) current:vec4<f32>, @location(4) previous:vec4<f32>,
}
@vertex fn water_vertex(@location(0) vertex:vec4<f32>)->VertexOut {
    let anchor=floor(view.camera_xyz_time.xz);
    let rest=anchor+vertex.xy;
    var f=field_at(rest,vertex.z);
    let local=vertex.xy/vertex.z;
    if max(abs(local.x),abs(local.y))>=32.0 {
        let lo=floor(local/2.0)*2.0;let hi=ceil(local/2.0)*2.0;
        let a=field_at(anchor+lo*vertex.z,vertex.z*2.0);
        let b=field_at(anchor+hi*vertex.z,vertex.z*2.0);
        f=add_field(add_field(Field(vec4(0.0),vec4(0.0),vec4(0.0)),a,0.5),b,0.5);
    }
    var world=vec3(rest.x+f.displacement.x,water.level_dt_reset.x+f.displacement.y,rest.y+f.displacement.z);
    // Infinite final edge: clamp reverse-Z to the far plane, preserving the horizon.
    if max(abs(vertex.x),abs(vertex.y))>=524288.0 {world=vec3(anchor.x+vertex.x*100.0,water.level_dt_reset.x,anchor.y+vertex.y*100.0);}
    var clip=view.view_proj*vec4(world,1.0);clip.z=max(clip.z,0.0000001);
    let current=view.unjittered_view_proj*vec4(world,1.0);
    let previous=view.previous_view_proj*vec4(world-f.velocity.xyz*water.level_dt_reset.y-vec3(0.0,water.level_dt_reset.x-water.level_dt_reset.w,0.0),1.0);
    return VertexOut(clip,world,rest,f.velocity.xyz,current,previous);
}
struct Output { @location(0) colour:vec4<f32>,@location(1) motion:vec2<f32>,@location(2) reactive:f32, }
@fragment fn water_fragment(input:VertexOut)->Output {
    let footprint=max(length(dpdx(input.rest)),length(dpdy(input.rest)));
    var slope=long_at(input.rest,max(0.01,footprint)).slope.xy*water.gains.x;
    let cache_lod=long_lod(input.rest,max(0.01,footprint));let low_cache=u32(floor(cache_lod));
    let long_variance=water.lengths.w+water.gains.x*water.gains.x*mix(water.cache[low_cache].w,water.cache[min(low_cache+1u,7u)].w,fract(cache_lod));
    var covariance=vec3(long_variance*0.5,0.0,long_variance*0.5);
    var differential=vec3(1.0,0.0,1.0);
    for(var band=0u;band<3u;band++) {
        let lod=clamp(log2(max(1.0,footprint*256.0/water.lengths[band]))/4.0,0.0,2.0);
        let low=u32(floor(lod))*4u;let high=min(low+4u,8u);
        let a=moment_at(input.rest,band,low);let b=moment_at(input.rest,band,high);
        let m=Moments(mix(a.a,b.a,fract(lod)),mix(a.b,b.b,fract(lod)));
        let gain=water.gains[band+1u];
        slope+=gain*m.a.xy;
        covariance+=gain*gain*vec3(max(0.0,m.a.z-m.a.x*m.a.x),m.a.w-m.a.x*m.a.y,max(0.0,m.b.x-m.a.y*m.a.y));
        let field=cascade_at(input.rest,band);
        differential+=gain*(1.0-lod*0.5)*vec3(field.slope.z,field.slope.w,field.velocity.w);
    }
    // The cascades store derivatives in rest coordinates. The drawn horizontal
    // map compresses crests and stretches troughs; both normal and covariance
    // must be carried through its inverse Jacobian, as the long cache already is.
    let determinant=max(0.05,differential.x*differential.z-differential.y*differential.y);
    slope=vec2(slope.x*differential.z-slope.y*differential.y,slope.y*differential.x-slope.x*differential.y)/determinant;
    let a=differential.z;let b=-differential.y;let c=differential.x;
    covariance=vec3(
        a*a*covariance.x+2.0*a*b*covariance.y+b*b*covariance.z,
        a*b*covariance.x+(a*c+b*b)*covariance.y+b*c*covariance.z,
        b*b*covariance.x+2.0*b*c*covariance.y+c*c*covariance.z)/(determinant*determinant);
    let variance=max(0.0,covariance.x+covariance.z);
    let n=normalize(vec3(-slope.x,1.0,-slope.y));
    let v=normalize(view.camera_xyz_time.xyz-input.world);
    let nv=max(dot(n,v),0.001);
    // GGX's perceptual roughness is sqrt(alpha); alpha is the RMS unresolved slope.
    let roughness=clamp(pow(max(variance,0.000004),0.25),0.045,0.8);
    let reflected=environment_reflection(reflect(-v,n),roughness,nv);
    let fresnel=water_mean_fresnel(slope,covariance,v);
    // A dielectric interface splits the reflected and transmitted paths by R.
    // The environment lends its filtered radiance; its opaque-material DFG is not R.
    var light=reflected.radiance*fresnel;
    // Deep-water in-scatter, driven only by the shared hemispherical irradiance.
    let l=environment_light.direction_to_light.xyz;
    let sunlight=sun_irradiance(input.world);
    let incident=environment_diffuse(vec3(0.0,1.0,0.0))+sunlight*max(l.y,0.0);
    light+=(1.0-fresnel)*incident*water.scatter.rgb/3.14159265;
    let h=normalize(l+v);let nh=max(dot(n,h),0.0);let nl=max(dot(n,l),0.0);
    let a2=pow(roughness,4.0);let d=a2/(3.14159265*pow(nh*nh*(a2-1.0)+1.0,2.0));
    let gv=2.0*nv/(nv+sqrt(a2+(1.0-a2)*nv*nv));
    let gl=2.0*nl/(nl+sqrt(a2+(1.0-a2)*nl*nl));
    let fh=water_fresnel(max(dot(v,h),0.0));
    light+=sunlight*d*gv*gl*fh/(4.0*nv+0.00001);
    let uv=input.clip.xy*view_aux.viewport_inv_viewport.zw;
    let air=environment_aerial(input.world,uv);
    let colour=(light*air.transmission+air.scatter)*environment_storage_scale();
    var motion=(input.previous.xy/input.previous.w-input.current.xy/input.current.w)*vec2(0.5,-0.5)*view_aux.viewport_inv_viewport.xy;
    if water.level_dt_reset.z>0.5 {motion=vec2(65504.0);}
    return Output(vec4(colour,1.0),motion,clamp(fresnel+0.1,0.0,1.0));
}
