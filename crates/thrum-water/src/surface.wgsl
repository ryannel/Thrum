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
struct Field { displacement:vec4<f32>, slope:vec4<f32>, velocity:vec4<f32>, }
fn add_field(a:Field,b:Field,w:f32)->Field {return Field(a.displacement+w*b.displacement,a.slope+w*b.slope,a.velocity+w*b.velocity);}
fn cascade_at(point:vec2<f32>,band:u32)->Field {
    let uv=point/water.lengths[band];let layer=i32(band*5u);
    let displacement=textureSampleLevel(cascades,periodic,uv,layer,0.0);
    let residual=textureSampleLevel(cascades,periodic,uv,layer+4,0.0).yzw;
    return Field(displacement+vec4(residual,0.0),textureSampleLevel(cascades,periodic,uv,layer+1,0.0),textureSampleLevel(cascades,periodic,uv,layer+2,0.0));
}
fn long_level(point:vec2<f32>,level:u32)->Field {
    let uv=(point-water.cache[level].xy)/(128.0*water.cache[level].z);let layer=i32(level*5u);
    let displacement=textureSampleLevel(long_field,bounded,uv,layer,0.0);
    let residual=textureSampleLevel(long_field,bounded,uv,layer+4,0.0).yzw;
    let result=Field(displacement+vec4(residual,0.0),textureSampleLevel(long_field,bounded,uv,layer+1,0.0),textureSampleLevel(long_field,bounded,uv,layer+2,0.0));
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
fn field_at(point:vec2<f32>,spacing:f32,gains:vec4<f32>)->Field {
    let long=long_at(point,spacing);
    var f=add_field(Field(vec4(0.0),vec4(0.0),vec4(0.0)),long,gains.x);
    for(var band=0u;band<3u;band++) {
        let fade=1.0-smoothstep(water.lengths[band]/128.0,water.lengths[band]/32.0,spacing);
        f=add_field(f,cascade_at(point,band),gains[band+1u]*fade);
    }
    return f;
}
struct SurfaceRow {
    level: f32,
    previous_level: f32,
    gains: vec4<f32>,
    bed: f32,
    owned: bool,
    swash: f32,
    loss: f32,
}
struct VertexOut {
    @builtin(position) clip:vec4<f32>, @location(0) world:vec3<f32>,
    @location(1) rest:vec2<f32>, @location(2) velocity:vec3<f32>,
    @location(3) current:vec4<f32>, @location(4) previous:vec4<f32>,
    @location(5) @interpolate(flat) body:u32,
    @location(6) surf_weight:f32,
    @location(7) surf_normal:vec3<f32>,
}

fn sea_surf_normal(index:u32, local:vec2<f32>, spacing:f32)->vec3<f32> {
    // Neighbours stay inside this ring's full grid. Its inner hole skips
    // indices only, so the cache also holds the nodes either side of the seam.
    let row = 2u*SURF_GRID_RADIUS+1u;
    let radius = f32(SURF_GRID_RADIUS);
    let left = select(1u, 0u, local.x <= -radius);
    let right = select(1u, 0u, local.x >= radius);
    let below = select(1u, 0u, local.y <= -radius);
    let above = select(1u, 0u, local.y >= radius);
    let x0 = surf_current(index-left);
    let x1 = surf_current(index+right);
    let z0 = surf_current(index-row*below);
    let z1 = surf_current(index+row*above);
    if min(min(x0.w, x1.w), min(z0.w, z1.w)) < 0.5 { return vec3(0.0); }
    let u = x1.xyz-x0.xyz+vec3(spacing*f32(left+right), 0.0, 0.0);
    let v = z1.xyz-z0.xyz+vec3(0.0, 0.0, spacing*f32(below+above));
    let normal = cross(v, u);
    return normal/max(length(normal), 1e-20);
}

@vertex fn water_vertex(@location(0) vertex:vec4<f32>, @builtin(vertex_index) index:u32)->VertexOut {
    let sea=vertex.w==0.0;
    let body=u32(vertex.w);
    let anchor=select(vec2(0.0),floor(view.camera_xyz_time.xz),sea);
    let rest=anchor+vertex.xy;
    let row=surface_vertex(rest,body);
    var f=field_at(rest,vertex.z,row.gains);
    var surf_weight = 0.0;
    var surf_motion = vec3(0.0);
    var surf_normal = vec3(0.0);
    let transformed = surf_current(index);
    let local=vertex.xy/vertex.z;
    if sea && max(abs(local.x),abs(local.y))>=32.0 {
        let lo=floor(local/2.0)*2.0;let hi=ceil(local/2.0)*2.0;
        let p=anchor+lo*vertex.z;let q=anchor+hi*vertex.z;
        let a=field_at(p,vertex.z*2.0,surface_vertex(p,body).gains);
        let b=field_at(q,vertex.z*2.0,surface_vertex(q,body).gains);
        f=add_field(add_field(Field(vec4(0.0),vec4(0.0),vec4(0.0)),a,0.5),b,0.5);
        if SURF_CACHE_ENABLED && transformed.w > 0.0 {
            let long_p = long_at(p, vertex.z*2.0);
            let long_q = long_at(q, vertex.z*2.0);
            let long = add_field(add_field(Field(vec4(0.0),vec4(0.0),vec4(0.0)),
                long_p,0.5*surface_vertex(p,body).gains.x),long_q,0.5*surface_vertex(q,body).gains.x);
            f = add_field(f,long,-1.0);
        }
    } else if SURF_CACHE_ENABLED && transformed.w > 0.0 {
        f = add_field(f,long_at(rest,vertex.z),-row.gains.x);
    }
    if SURF_CACHE_ENABLED && transformed.w > 0.0 {
        surf_weight = transformed.w;
        f.displacement += vec4(transformed.xyz, 0.0);
        surf_motion = transformed.xyz - surf_previous(index);
        if sea { surf_normal = sea_surf_normal(index, local, vertex.z); }
    }
    let wake=wake_at(rest,0.0);
    f.displacement.y+=wake.x;
    f.velocity.y+=wake.w;
    var world=vec3(rest.x+f.displacement.x,row.level+f.displacement.y,rest.y+f.displacement.z);
    // Infinite final edge: clamp reverse-Z to the far plane, preserving the horizon.
    if sea && max(abs(vertex.x),abs(vertex.y))>=524288.0 {world=vec3(anchor.x+vertex.x*100.0,row.level,anchor.y+vertex.y*100.0);}
    var clip=view.view_proj*vec4(world,1.0);clip.z=max(clip.z,0.0000001);
    let current=view.unjittered_view_proj*vec4(world,1.0);
    let previous=view.previous_view_proj*vec4(world-f.velocity.xyz*water.level_dt_reset.y-surf_motion-vec3(0.0,row.level-row.previous_level,0.0),1.0);
    return VertexOut(clip,world,rest,f.velocity.xyz,current,previous,body,surf_weight,surf_normal);
}
struct Output { @location(0) colour:vec4<f32>,@location(1) motion:vec2<f32>,@location(2) reactive:f32, }
struct FilteredLayer { slope:vec2<f32>, second:vec3<f32>, differential:vec3<f32>, }
fn filtered_layer(t:texture_2d_array<f32>,s:sampler,uv:vec2<f32>,dx:vec2<f32>,dy:vec2<f32>,layer:i32)->FilteredLayer {
    let derivative=sample_field(t,s,uv,layer+1,dx,dy);
    let moments=sample_field(t,s,uv,layer+3,dx,dy);
    return FilteredLayer(derivative.xy,moments.xyz,vec3(derivative.zw,moments.w));
}
fn reflected_sky(slope:vec2<f32>,covariance:vec3<f32>,v:vec3<f32>)->vec4<f32> {
    // Integrate visible unresolved facets, clamping each reflected direction
    // before the lent sky lookup. A prefiltered cube at the horizon would
    // already include its ground hemisphere even if its centre were clamped.
    let nodes=array<f32,4>(-2.334414218,-0.741963784,0.741963784,2.334414218);
    let weights=array<f32,4>(0.045875855,0.454124145,0.454124145,0.045875855);
    let a=sqrt(max(0.0,covariance.x));
    let limit=sqrt(max(0.0,covariance.x*covariance.z));
    let b=clamp(covariance.y,-limit,limit)/max(a,0.000001);
    let c=sqrt(max(0.0,covariance.z-b*b));
    var result=vec4(0.0);var visible=0.0;
    for(var i=0u;i<4u;i++){for(var j=0u;j<4u;j++) {
        let s=slope+vec2(a*nodes[i],b*nodes[i]+c*nodes[j]);
        let n=normalize(vec3(-s.x,1.0,-s.y));
        let ray=reflect(-v,n);
        let f=water_fresnel(dot(n,v));
        let weight=weights[i]*weights[j]*max(0.0,v.y-dot(v.xz,s));
        let sky=environment_sky_radiance(normalize(vec3(ray.x,max(0.001,ray.y),ray.z)));
        result+=weight*vec4(sky*f,f);visible+=weight;
    }}
    return result/max(visible,0.0000001);
}
@fragment fn water_fragment(input:VertexOut)->Output {
    let dx=dpdx(input.rest);let dy=dpdy(input.rest);
    let wake_slope=wake_at(input.rest,0.0).yz;
    let wake_dx=dpdx(wake_slope);let wake_dy=dpdy(wake_slope);
    let row=surface_pixel(input.rest,input.body);
    if !row.owned { discard; }
    // Select a cache only for spatial coverage. Hardware filters each layer
    // over the ellipse; its longest side never selects a level or mip.
    let cache_lod=long_lod(input.rest,1.0);let low_cache=u32(floor(cache_lod));
    var slope=vec2(0.0);
    let gain=tail_gain(row);
    let tail=water.lengths.w*0.5*gain*gain;
    let wake_xx=dot(vec2(wake_dx.x,wake_dy.x),vec2(wake_dx.x,wake_dy.x));
    let wake_xz=wake_dx.x*wake_dx.y+wake_dy.x*wake_dy.y;
    let wake_zz=dot(vec2(wake_dx.y,wake_dy.y),vec2(wake_dx.y,wake_dy.y));
    var covariance=vec3(tail,0.0,tail)+vec3(wake_xx,wake_xz,wake_zz)/12.0;
    var differential=vec3(1.0,0.0,1.0);
    let fade=1.0-smoothstep(4000.0,8000.0,length(input.rest-view.camera_xyz_time.xz));
    var long_mean=vec2(0.0);var long_second=vec3(0.0);var long_differential=vec3(0.0);
    for(var side=0u;side<2u;side++) {
        let level=min(low_cache+side,7u);let scale=128.0*water.cache[level].z;
        let uv=(input.rest-water.cache[level].xy)/scale;
        let m=filtered_layer(long_field,bounded,uv,dx/scale,dy/scale,i32(level*5u));
        let w=select(1.0-fract(cache_lod),fract(cache_lod),side==1u);
        long_mean+=w*m.slope;
        long_second+=w*(m.second+water.long_covariance[level].xyz);
        long_differential+=w*m.differential;
    }
    slope=row.gains.x*fade*long_mean;
    slope+=wake_slope;
    covariance+=row.gains.x*row.gains.x*(long_second-fade*fade*vec3(long_mean.x*long_mean.x,long_mean.x*long_mean.y,long_mean.y*long_mean.y));
    differential+=row.gains.x*fade*long_differential;
    for(var band=0u;band<3u;band++) {
        let scale=water.lengths[band];
        let m=filtered_layer(cascades,periodic,input.rest/scale,dx/scale,dy/scale,i32(band*5u));
        let gain=row.gains[band+1u];
        slope+=gain*m.slope;
        covariance+=gain*gain*(m.second-vec3(m.slope.x*m.slope.x,m.slope.x*m.slope.y,m.slope.y*m.slope.y));
        differential+=gain*m.differential;
    }
    // The cascades store derivatives in rest coordinates. The drawn horizontal
    // map compresses crests and stretches troughs; both normal and covariance
    // must be carried through the inverse of the summed horizontal Jacobian.
    let determinant=max(0.05,differential.x*differential.z-differential.y*differential.y);
    slope=vec2(slope.x*differential.z-slope.y*differential.y,slope.y*differential.x-slope.x*differential.y)/determinant;
    let a=differential.z;let b=-differential.y;let c=differential.x;
    covariance=vec3(
        a*a*covariance.x+2.0*a*b*covariance.y+b*b*covariance.z,
        a*b*covariance.x+(a*c+b*b)*covariance.y+b*c*covariance.z,
        b*b*covariance.x+2.0*b*c*covariance.y+c*c*covariance.z)/(determinant*determinant);
    if SURF_CACHE_ENABLED && input.surf_weight >= 0.999 && dot(input.surf_normal,input.surf_normal)>1e-8 {
        // Interpolate normals of the displaced rest sheet before adding the
        // fine ripple slopes. The depth test still selects the nearest branch.
        let up = select(-input.surf_normal, input.surf_normal, input.surf_normal.y >= 0.0);
        slope += -up.xz / max(up.y, length(up) * 0.03 + 1e-20) - row.gains.x*fade*long_mean;
    }
    var whitewater=vec4(0.0);
    if FOAM_ENABLED {
        whitewater=foam_surface(input.world,row,max(length(dx),length(dy)),determinant);
        covariance*=1.0-0.65*whitewater.w;
    }
    let variance=max(0.0,covariance.x+covariance.z);
    let n=normalize(vec3(-slope.x,1.0,-slope.y));
    let v=normalize(view.camera_xyz_time.xyz-input.world);
    let nv=max(dot(n,v),0.001);
    // GGX's perceptual roughness is sqrt(alpha); alpha is the RMS unresolved slope.
    let roughness=clamp(pow(max(variance,0.000004),0.25),0.045,0.8);
    let reflected=reflected_sky(slope,covariance,v);
    let fresnel=clamp(reflected.w,0.0,1.0);
    // A dielectric interface splits the reflected and transmitted paths by R.
    // The environment lends its filtered radiance; its opaque-material DFG is not R.
    var light=reflected.rgb;
    // Deep-water in-scatter, driven only by the shared hemispherical irradiance.
    let l=environment_sun_direction();
    let sunlight=sun_irradiance(input.world);
    let incident=environment_diffuse(vec3(0.0,1.0,0.0))+sunlight*max(l.y,0.0);
    light+=(1.0-fresnel)*incident*water.scatter.rgb/3.14159265;
    // Height in RMS-wave units and horizontal compression locate the thin
    // crest. Its extra in-scatter is bounded and uses the same lent irradiance
    // as the body; no independent light, pattern, or geometric sharpening.
    let crest_height=max(0.0,input.world.y-row.level)/max(water.scatter.w,0.001);
    let crest=1.0-exp(-crest_height*max(0.0,1.0-determinant));
    light+=(1.0-fresnel)*incident*water.scatter.rgb*crest;
    let h=normalize(l+v);let nh=max(dot(n,h),0.0);let nl=max(dot(n,l),0.0);
    let a2=pow(roughness,4.0);let d=a2/(3.14159265*pow(nh*nh*(a2-1.0)+1.0,2.0));
    let gv=2.0*nv/(nv+sqrt(a2+(1.0-a2)*nv*nv));
    let gl=2.0*nl/(nl+sqrt(a2+(1.0-a2)*nl*nl));
    let fh=water_fresnel(max(dot(v,h),0.0));
    light+=sunlight*d*gv*gl*fh/(4.0*nv+0.00001);
    if FOAM_ENABLED {
        light+=incident*vec3(0.035,0.19,0.19)*whitewater.x*(1.0-fresnel);
        light=mix(light,incident*vec3(0.86,0.90,0.89)/3.14159265,whitewater.y);
    }
    let uv=input.clip.xy*view_aux.viewport_inv_viewport.zw;
    let air=environment_aerial(input.world,uv);
    let colour=(light*air.transmission+air.scatter)*environment_storage_scale();
    var motion=(input.previous.xy/input.previous.w-input.current.xy/input.current.w)*vec2(0.5,-0.5)*view_aux.viewport_inv_viewport.xy;
    if water.level_dt_reset.z>0.5 {motion=vec2(65504.0);}
    var opacity=surface_opacity(row,input.world,v.y);
    if FOAM_ENABLED { opacity=max(opacity,whitewater.y); }
    return Output(vec4(colour,opacity),motion,clamp(fresnel+0.1+whitewater.y,0.0,1.0));
}
