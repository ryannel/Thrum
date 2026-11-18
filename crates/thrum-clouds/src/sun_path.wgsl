struct Params {
    weather: array<vec4<f32>,4>, origin:vec4<f32>, direction:vec4<f32>,
    physical:vec4<f32>, erosion:vec4<f32>, noise:vec4<u32>, rows:vec4<u32>,
};
@group(0) @binding(0) var<uniform> p:Params;
@group(0) @binding(1) var columns:texture_2d_array<f32>;
@group(0) @binding(2) var optical:texture_2d_array<f32>;
@group(0) @binding(3) var product:texture_storage_2d_array<rgba16float,write>;
@group(0) @binding(4) var near_path:texture_2d_array<f32>;
@group(0) @binding(5) var middle_path:texture_2d_array<f32>;
@group(0) @binding(6) var far_path:texture_2d_array<f32>;
@group(0) @binding(7) var ground_field:texture_storage_2d<rgba16float,write>;
struct Mapping {region:vec4<f32>,plane_drift:vec4<f32>,path_border:vec4<f32>,far_value:vec4<f32>,near:vec4<f32>,middle:vec4<f32>};
@group(0) @binding(8) var<storage,read_write> mapping:Mapping;

fn smooth_unit(x:f32)->f32 {let a=clamp(x,0.0,1.0);return a*a*(3.0-2.0*a);}
fn hash(i:u32,j:u32,g:u32,stream:u32)->f32 {
    var x=i*0x9e3779b1u+j*0x85ebca77u+g*0xc2b2ae3du+(stream+131u*p.noise.w)*0x27d4eb2fu;
    x=x^(x>>15u);x=x*0x2c1b3c6du;x=x^(x>>12u);x=x*0x297a2d39u;x=x^(x>>15u);
    return f32(x)*0.00000000023283064365386963;
}
fn shape_noise(point:vec3<f32>)->f32 {
    let local=point/CLOUD_SHAPE_METRES+vec3<f32>(p.erosion.y,0.0,p.erosion.z);
    let offset=vec3<i32>(floor(local));let f=vec3<f32>(smooth_unit(fract(local.x)),smooth_unit(fract(local.y)),smooth_unit(fract(local.z)));
    var n=0.0;
    for(var x=0;x<=1;x++){for(var y=0;y<=1;y++){for(var z=0;z<=1;z++){
        let w=select(vec3<f32>(1.0)-f,f,vec3<bool>(x!=0,y!=0,z!=0));
        n+=w.x*w.y*w.z*hash(p.noise.x+u32(offset.x+x),u32(offset.y+y),p.noise.z+u32(offset.z+z),40u);
    }}}return n;
}
fn bilinear(field:texture_2d_array<f32>,coordinate:vec2<f32>,level:i32)->vec4<f32> {
    let dimensions=vec2<i32>(textureDimensions(field));
    let q=coordinate-0.5;let lo=vec2<i32>(floor(q));let f=fract(q);
    let a=textureLoad(field,clamp(lo,vec2<i32>(0),dimensions-1),level,0);
    let b=textureLoad(field,clamp(lo+vec2<i32>(1,0),vec2<i32>(0),dimensions-1),level,0);
    let c=textureLoad(field,clamp(lo+vec2<i32>(0,1),vec2<i32>(0),dimensions-1),level,0);
    let d=textureLoad(field,clamp(lo+vec2<i32>(1),vec2<i32>(0),dimensions-1),level,0);
    return mix(mix(a,b,f.x),mix(c,d,f.x),f.y);
}
fn column_density(point:vec3<f32>)->f32 {
    var sigma=0.0;
    for(var level=3;level>=0;level--) {
        let m=p.weather[level];let uv=(point.xz-m.xy)/m.z;
        let edge=min(uv,vec2<f32>(m.w)-uv);
        let w=smooth_unit(min(edge.x,edge.y)/(m.w*0.15));
        if w==0.0 {continue;}
        let c=bilinear(columns,uv,level);let o=bilinear(optical,uv,level);
        let depth=c.y-c.x;var density=0.0;
        if depth>0.0 && point.y>c.x && point.y<c.y {
            let h=(point.y-c.x)/depth;
            let vertical=smooth_unit(h/o.z)*(1.0-smooth_unit((h-(1.0-o.w))/o.w));
            density=o.y/depth*vertical/(1.0-(o.z+o.w)*0.5);
        }
        sigma=mix(sigma,density,w);
    }
    return sigma;
}
fn density(point:vec3<f32>)->f32 {
    let sigma=column_density(point);
    if sigma<=0.0 || p.physical.z<=0.0 || p.physical.w==0.0 {return sigma;}
    let signal=min(1.0,sigma/p.physical.z);
    let threshold=p.physical.w*shape_noise(point);
    let shaped=max(0.0,(signal-threshold)/(1.0-threshold));
    let eroded=max(0.0,shaped-p.physical.w*p.erosion.x*(1.0-shaped));
    return sigma*min(1.0,eroded/signal);
}
@compute @workgroup_size(8,8)
fn path(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=u32(p.origin.w) || id.y>=p.rows.y {return;}
    let row=(p.rows.x+id.y)%u32(p.origin.w);
    let xy=(vec2<f32>(f32(id.x),f32(row))+0.5)*p.origin.z;
    let dh=(p.physical.y-p.physical.x)/f32(p.rows.z-1u);
    var tau=0.0;var packed=vec4<f32>(0.0);
    for(var slice=0u;slice<p.rows.z;slice++) {
        if slice>0u {
            // Stored height nodes are not integration steps. At low Sun their
            // ray intervals are long enough to skip whole shape-noise lobes.
            let steps=u32(ceil(dh/(p.direction.y*40.0)));
            let sub=dh/f32(steps);
            for(var j=0u;j<steps;j++) {
                let height=p.physical.y-(f32(slice-1u)*dh+(f32(j)+0.5)*sub);
                let offset=p.direction.xz*(height-p.physical.x)/p.direction.y;
                tau+=density(vec3<f32>(xy.x+offset.x,height,xy.y+offset.y))*sub/p.direction.y;
            }
        }
        packed[slice%4u]=min(65504.0,tau);
        if slice%4u==3u {textureStore(product,vec2<i32>(i32(id.x),i32(row)),i32(slice/4u),packed);}
    }
}
fn ground_tau(field:texture_2d_array<f32>,point:vec2<f32>,level:u32)->f32 {
    let m=p.weather[level];let coordinate=(point-m.xy)/m.z;
    return bilinear(field,coordinate,i32(u32(m.w)/4u-1u)).w;
}
@compute @workgroup_size(8,8)
fn ground(@builtin(global_invocation_id) id:vec3<u32>) {
    if any(id.xy>=vec2<u32>(u32(p.origin.w))) {return;}
    let half=u32(p.origin.w)/2u;let tile=id.xy/half;let local=id.xy%half;
    if all(tile==vec2<u32>(1u)) {textureStore(ground_field,vec2<i32>(id.xy),vec4<f32>(0.0));return;}
    let level=select(tile.x,2u,tile.y==1u);let m=p.weather[level];
    let point=m.xy+(vec2<f32>(local)+0.5)*m.z*p.physical.x/f32(half);
    var tau=0.0;
    if level==0u {tau=ground_tau(near_path,point,0u);}else if level==1u {tau=ground_tau(middle_path,point,1u);}else{tau=ground_tau(far_path,point,2u);}
    textureStore(ground_field,vec2<i32>(id.xy),vec4<f32>(tau,0.0,0.0,0.0));
}
var<workgroup> sums:array<f32,256>;
@compute @workgroup_size(256)
fn far(@builtin(local_invocation_index) lane:u32) {
    let size=textureDimensions(far_path);let layers=textureNumLayers(far_path);
    var sum=0.0;
    for(var i=lane;i<size.x*size.y;i+=256u) {sum+=exp(-textureLoad(far_path,vec2<i32>(i32(i%size.x),i32(i/size.x)),i32(layers)-1,0).w);}
    sums[lane]=sum;workgroupBarrier();
    for(var step=128u;step>0u;step/=2u) {if lane<step {sums[lane]+=sums[lane+step];}workgroupBarrier();}
    if lane==0u {mapping.far_value.x=-log(max(1e-30,sums[0]/f32(size.x*size.y)));}
}
