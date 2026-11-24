struct Params {
    integers: vec4<u32>,
    origin: vec4<f32>,
    weather: vec4<f32>,
    physical: vec4<f32>,
    vertical: vec4<f32>,
    noise: vec4<u32>,
    rows: vec4<u32>,
    turret: vec4<f32>,
};
@group(0) @binding(0) var<uniform> p: Params;
@group(0) @binding(1) var columns: texture_storage_2d_array<rgba32float,write>;
@group(0) @binding(2) var optical: texture_storage_2d_array<rgba32float,write>;
@group(0) @binding(3) var identities: texture_storage_2d_array<rgba32uint,write>;
@group(0) @binding(4) var<storage,read> axes:array<vec4<u32>>;

fn hash(i:u32,j:u32,g:u32,stream:u32)->u32 {
    var x=i*0x9e3779b1u+j*0x85ebca77u+g*0xc2b2ae3du+(stream+131u*p.integers.w)*0x27d4eb2fu;
    x=x^(x>>15u); x=x*0x2c1b3c6du; x=x^(x>>12u);x=x*0x297a2d39u;return x^(x>>15u);
}
fn random(i:u32,j:u32,g:u32,stream:u32)->f32 {return f32(hash(i,j,g,stream))*0.00000000023283064365386963;}
fn smooth_unit(x:f32)->f32 {let a=clamp(x,0.0,1.0);return a*a*(3.0-2.0*a);}
fn turret_cap(site: vec2<u32>, local: vec2<f32>) -> f32 {
    var cap = 0.0;
    for (var x = -1; x <= 1; x++) {
        for (var z = -1; z <= 1; z++) {
            let i = site.x + u32(x);
            let j = site.y + u32(z);
            let rank = pow(random(i, j, 7u, 73u), 1.6);
            let radius = min(1.0, (0.45 + 0.45 * rank) * (0.85 + 0.3 * random(i, j, 7u, 72u)));
            let centre = vec2<f32>(f32(x), f32(z)) +
                vec2<f32>(random(i, j, 7u, 70u), random(i, j, 7u, 71u));
            let delta = local - centre;
            let r2 = dot(delta, delta) / (radius * radius);
            cap = max(cap, (0.3 + 0.7 * rank) * sqrt(max(0.0, 1.0 - r2)));
        }
    }
    return cap;
}
fn noise(point:vec3<f32>)->f32 {
    let offset=vec3<i32>(floor(point));let f=vec3<f32>(smooth_unit(fract(point.x)),smooth_unit(fract(point.y)),smooth_unit(fract(point.z)));
    var n=0.0;
    for(var x=0;x<=1;x++){for(var y=0;y<=1;y++){for(var z=0;z<=1;z++){
        let w=select(vec3<f32>(1.0)-f,f,vec3<bool>(x!=0,y!=0,z!=0));
        n+=w.x*w.y*w.z*random(p.noise.x+u32(offset.x+x),p.noise.y+u32(offset.y+y),p.noise.z+u32(offset.z+z),20u);
    }}}return n;
}
@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=p.rows.z || id.y>=p.rows.y {return;}
    let row=(p.rows.x+id.y)%p.rows.z;
    let coordinate=p.origin.xy+(vec2<f32>(f32(id.x),f32(row))+0.5)*p.origin.w;
    let ax=axes[id.x];let az=axes[p.rows.z+row];
    let site=vec2<u32>(ax.x,az.x);
    let local=vec2<f32>(bitcast<f32>(ax.y),bitcast<f32>(az.y));
    let turret = turret_cap(vec2<u32>(ax.z, az.z), bitcast<vec2<f32>>(vec2<u32>(ax.w, az.w)));
    var coverage=0.0;var depth=0.0;var age=0.0;var generation_mask=0u;
    for(var di=-1;di<=1;di++){for(var dj=-1;dj<=1;dj++){
        let i=site.x+u32(di);let j=site.y+u32(dj);
        let birth_offset=random(i,j,0u,1u);
        let goff=select(0,-1,p.noise.w<hash(i,j,0u,1u));
        if goff<0 {generation_mask=generation_mask|(1u<<u32((di+1)*3+dj+1));}
        for(var old=-1;old<=0;old++){
            let g=p.integers.z+u32(goff+old);
            let birth=birth_offset+random(i,j,g,2u);
            let life=0.4+0.6*random(i,j,g,3u);
            let phase=(p.origin.z-f32(goff+old)-birth)/life;
            var envelope=0.0;
            if phase>=0.0 && phase<1.0 {let s=sin(3.141592653589793*phase);envelope=s*s;}
            envelope*=smooth_unit((0.95*smooth_unit(p.weather.x/0.6)-random(i,j,g,4u))/0.15);
            let r=random(i,j,g,7u);
            let radius=(0.3+0.5*r*r)*(0.5+0.5*envelope);
            let centre=vec2<f32>(f32(di),f32(dj))+vec2<f32>(random(i,j,g,5u),random(i,j,g,6u));
            let delta=local-centre;
            let dome=envelope*max(0.0,1.0-dot(delta,delta)/(radius*radius))*(1.0-p.vertical.x);
            let share=min(pow(dome,0.7),envelope*turret);
            let clearing=smooth_unit((share-p.turret.x)/p.turret.y);
            let d=dome*clearing;
            if d>coverage {age=phase;}
            coverage=max(coverage,d);
            let h=random(i,j,g,8u);
            depth=max(depth,mix(p.physical.y,p.physical.z,h)*share*clearing);
        }
    }}
    let rem=vec2<f32>(vec2<i32>(p.integers.xy)-vec2<i32>(p.noise.xy)*4);
    let grem=f32(i32(p.integers.z)-i32(p.noise.z)*2);
    let n=noise(vec3<f32>((coordinate+rem)/4.0,(p.origin.z+grem)/2.0));
    let deck=p.vertical.x*smooth_unit((n-(1.0-1.3*smooth_unit((p.weather.x-0.45)/0.55)))/0.2);
    coverage=max(coverage,deck);
    depth=max(depth,p.physical.y*pow(deck,0.7)*(0.6+0.4*n));
    let tau=p.physical.w*depth*coverage*(1.0-(p.vertical.y+p.vertical.z)*0.5);
    if coverage==0.0 || depth==0.0 {coverage=0.0;depth=0.0;age=0.0;}
    let q=vec2<i32>(i32(id.x),i32(row));let layer=i32(p.rows.w);
    textureStore(columns,q,layer,vec4<f32>(p.physical.x,p.physical.x+depth,coverage,p.weather.y));
    textureStore(optical,q,layer,vec4<f32>(age,tau,p.vertical.yz));
    let generation=p.integers.z-u32(p.noise.w<hash(site.x,site.y,0u,1u));
    textureStore(identities,q,layer,vec4<u32>(site,generation,generation_mask));
}
