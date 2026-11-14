// Physical Fourier amplitudes: unnormalized positive-sign transforms.
struct Params {
    time: vec4<u32>, // loop fraction, long count, query count, unused
    physical: vec4<f32>, // choppiness, unused
    lengths: vec4<f32>,
    levels: array<vec4<f32>, 8>, // local origin x/z, texel width, max k
}
struct Bin { physical: vec4<f32>, clock: vec4<u32> }
struct Long { physical: vec4<f32>, rate: vec4<f32> }
struct Field { displacement: vec4<f32>, slope: vec4<f32>, velocity: vec4<f32> }
struct Moments { slope: vec4<f32>, extra: vec4<f32> }
@group(0) @binding(0) var<uniform> p: Params;
@group(0) @binding(1) var<storage, read> modes: array<Bin>;
@group(0) @binding(2) var<storage, read> components: array<Long>;
@group(0) @binding(3) var<storage, read> queries: array<vec2<f32>>;
@group(0) @binding(4) var<storage, read_write> a: array<vec2<f32>>;
@group(0) @binding(5) var<storage, read_write> b: array<vec2<f32>>;
@group(0) @binding(6) var<storage, read_write> field: array<Field>;
@group(0) @binding(7) var<storage, read_write> moments: array<Moments>;
@group(0) @binding(8) var<storage, read_write> long_field: array<Field>;
const TAU: f32 = 6.283185307179586;
const N: u32 = 256u;
const PIXELS: u32 = 65536u;
const CHANNELS: u32 = 11u;
fn mul(x: vec2<f32>, y: vec2<f32>) -> vec2<f32> {
    return vec2(x.x*y.x-x.y*y.y, x.x*y.y+x.y*y.x);
}
fn fourier(bin: Bin, channel: u32, patch_length: f32) -> vec2<f32> {
    let k = bin.physical.xy;
    let magnitude = length(k);
    if magnitude == 0.0 { return vec2(0.0); }
    // Unsigned multiplication is modulo 2^32: phase precision is independent of uptime.
    let turns = bin.clock.x * p.time.x;
    let phase = bin.physical.w - f32(turns) * (TAU / 4294967296.0)
        + (k.x+k.y) * patch_length / 512.0;
    let h = 0.5 * bin.physical.z * vec2(cos(phase), sin(phase));
    let omega = f32(bin.clock.x) * TAU / 1024.0;
    var factor = vec2(0.0);
    switch channel {
        case 0u: { factor.y = k.x/magnitude; }
        case 1u: { factor.x = 1.0; }
        case 2u: { factor.y = k.y/magnitude; }
        case 3u: { factor.y = k.x; }
        case 4u: { factor.y = k.y; }
        case 5u: { factor.x = -k.x*k.x/magnitude; }
        case 6u: { factor.x = -k.x*k.y/magnitude; }
        case 7u: { factor.x = -k.y*k.y/magnitude; }
        case 8u: { factor.x = omega*k.x/magnitude; }
        case 9u: { factor.y = -omega; }
        default: { factor.x = omega*k.y/magnitude; }
    }
    return mul(h, factor);
}
@compute @workgroup_size(64)
fn evolve(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if i >= 3u*PIXELS { return; }
    let band = i/PIXELS;
    let xy = i%PIXELS;
    let opposite = band*PIXELS + ((N-xy/N)%N)*N + (N-xy%N)%N;
    for(var channel=0u; channel<CHANNELS; channel++) {
        let v = fourier(modes[i], channel, p.lengths[band]);
        let conjugate = fourier(modes[opposite], channel, p.lengths[band]);
        a[(band*CHANNELS+channel)*PIXELS+xy] = v + vec2(conjugate.x,-conjugate.y);
    }
}
var<workgroup> values: array<vec2<f32>,256>;
fn transform(lane: u32) {
    for(var width=2u; width<=N; width*=2u) {
        let half = width/2u;
        let offset = lane%half;
        let first = (lane/half)*width + offset;
        var u = vec2(0.0); var v = vec2(0.0);
        if lane<128u {
            let angle = TAU*f32(offset)/f32(width);
            u=values[first]; v=mul(values[first+half],vec2(cos(angle),sin(angle)));
        }
        workgroupBarrier();
        if lane<128u { values[first]=u+v; values[first+half]=u-v; }
        workgroupBarrier();
    }
}
@compute @workgroup_size(256)
fn rows(@builtin(local_invocation_index) lane: u32, @builtin(workgroup_id) group: vec3<u32>) {
    let reversed = reverseBits(lane)>>24u;
    values[reversed]=a[group.y*PIXELS+group.x*N+lane];
    workgroupBarrier(); transform(lane);
    b[group.y*PIXELS+group.x*N+lane]=values[lane];
}
@compute @workgroup_size(256)
fn columns(@builtin(local_invocation_index) lane: u32, @builtin(workgroup_id) group: vec3<u32>) {
    let reversed = reverseBits(lane)>>24u;
    values[reversed]=b[group.y*PIXELS+lane*N+group.x];
    workgroupBarrier(); transform(lane);
    a[group.y*PIXELS+lane*N+group.x]=values[lane];
}
fn channel(band: u32, c: u32, i: u32) -> f32 { return a[(band*CHANNELS+c)*PIXELS+i].x; }
@compute @workgroup_size(64)
fn unpack(@builtin(global_invocation_id) id: vec3<u32>) {
    let i=id.x; if i>=3u*PIXELS { return; }
    let band=i/PIXELS; let pixel=i%PIXELS;
    let dx=channel(band,5u,pixel); let cross=channel(band,6u,pixel); let dz=channel(band,7u,pixel);
    let jac=(1.0+dx)*(1.0+dz)-cross*cross;
    let sx=channel(band,3u,pixel); let sz=channel(band,4u,pixel);
    field[i]=Field(vec4(channel(band,0u,pixel),channel(band,1u,pixel),channel(band,2u,pixel),jac),
        vec4(sx,sz,dx,cross),vec4(channel(band,8u,pixel),channel(band,9u,pixel),channel(band,10u,pixel),dz));
    // Raw second moments filter linearly; covariance is second - outer(mean).
    moments[band*PIXELS+pixel]=Moments(vec4(sx,sz,sx*sx,sx*sz),vec4(sz*sz,jac,jac*jac,0.0));
}
// Mips use a separate small uniform at group 1: parent offset, child offset, width.
@group(1) @binding(0) var<uniform> mip: vec4<u32>;
fn parent_moment(index: u32) -> Moments {
    let i=index*4u;
    return Moments(vec4(b[i],b[i+1u]),vec4(b[i+2u],b[i+3u]));
}
@compute @workgroup_size(64)
fn reduce(@builtin(global_invocation_id) id: vec3<u32>) {
    let width=mip.z; let count=width*width;
    if id.x>=3u*count { return; }
    let band=id.x/count; let pixel=id.x%count;
    let base=band*4u*count+(pixel/width)*4u*width+(pixel%width)*2u;
    let v0=parent_moment(base); let v1=parent_moment(base+1u);
    let v2=parent_moment(base+2u*width); let v3=parent_moment(base+2u*width+1u);
    moments[band*count+pixel]=Moments((v0.slope+v1.slope+v2.slope+v3.slope)*0.25,
        (v0.extra+v1.extra+v2.extra+v3.extra)*0.25);
}
// Shared body-query/cache evaluator: invert horizontal displacement at the world point.
fn sample_long(point: vec2<f32>, max_k: f32) -> Field {
    var offset=vec2(0.0);
    for(var iteration=0u; iteration<24u; iteration++) {
        var next=vec2(0.0);
        for(var j=0u; j<p.time.y; j++) {
            let c=components[j]; let k=c.physical.xy; let magnitude=length(k);
            if magnitude<=max_k {
                let angle=dot(k,point+offset)+c.physical.w;
                next+=p.physical.x*c.physical.z*k/magnitude*sin(angle);
            }
        }
        let error=length(next-offset); offset=next;
        if error<0.000001 { break; }
    }
    var h=0.0; var gradient=vec2(0.0); var differential=vec3(0.0); var velocity=vec3(0.0);
    for(var j=0u; j<p.time.y; j++) {
        let c=components[j]; let k=c.physical.xy; let magnitude=length(k);
        if magnitude<=max_k {
            let angle=dot(k,point+offset)+c.physical.w; let s=sin(angle); let co=cos(angle);
            let amp=c.physical.z; let omega=c.rate.x;
            h+=amp*co; gradient-=amp*k*s;
            differential-=p.physical.x*amp/magnitude*vec3(k.x*k.x,k.x*k.y,k.y*k.y)*co;
            velocity+=amp*omega*vec3(p.physical.x*k.x/magnitude*co,s,p.physical.x*k.y/magnitude*co);
        }
    }
    let jac=(1.0+differential.x)*(1.0+differential.z)-differential.y*differential.y;
    let slope=vec2(gradient.x*(1.0+differential.z)-gradient.y*differential.y,
        gradient.y*(1.0+differential.x)-gradient.x*differential.y)/jac;
    return Field(vec4(-offset.x,h,-offset.y,jac),vec4(slope,0.0,0.0),
        vec4(velocity,velocity.y-dot(slope,velocity.xz)));
}
@compute @workgroup_size(64)
fn long_waves(@builtin(global_invocation_id) id: vec3<u32>) {
    let i=id.x;
    if i<p.time.z { long_field[i]=sample_long(queries[i],1e20); return; }
    let cache=i-p.time.z; if cache>=8u*128u*128u { return; }
    let level=cache/(128u*128u); let xy=cache%(128u*128u); let info=p.levels[level];
    let point=info.xy+(vec2(f32(xy%128u),f32(xy/128u))+0.5)*info.z;
    long_field[i]=sample_long(point,info.w);
}
