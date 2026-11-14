struct Params {
    inverse: mat4x4<f32>,
    previous_view: mat4x4<f32>,
    view: mat4x4<f32>,
    state: vec4<f32>, // valid, reserved, width, height
    jitter: vec4<f32>,
}
@group(0) @binding(0) var colour: texture_2d<f32>;
@group(0) @binding(1) var depth: texture_depth_2d;
@group(0) @binding(2) var motion: texture_2d<f32>;
@group(0) @binding(3) var reactive: texture_2d<f32>;
@group(0) @binding(4) var history: texture_2d<f32>;
@group(0) @binding(5) var previous_depth: texture_2d<f32>;
@group(0) @binding(6) var output: texture_storage_2d<rgba16float, write>;
@group(0) @binding(7) var next_colour: texture_storage_2d<rgba16float, write>;
@group(0) @binding(8) var next_depth: texture_storage_2d<r32float, write>;
@group(0) @binding(9) var<uniform> params: Params;
struct Exposure { values: vec4<f32>, metadata: vec4<u32> }
@group(0) @binding(10) var<uniform> exposure: Exposure;
@group(0) @binding(11) var<uniform> previous_exposure: Exposure;

fn ycocg(c: vec3<f32>) -> vec3<f32> {
    return vec3(dot(c,vec3(0.25,0.5,0.25)),(c.r-c.b)*0.5,(-c.r+2.0*c.g-c.b)*0.25);
}
fn rgb(c: vec3<f32>) -> vec3<f32> { return vec3(c.x+c.y-c.z,c.x+c.z,c.x-c.y-c.z); }
fn bounded(p: vec2<i32>) -> vec2<i32> {
    return clamp(p,vec2(0),vec2<i32>(textureDimensions(colour))-vec2(1));
}
fn reconstruction_filter(distance: f32) -> f32 {
    // Mitchell-Netravali, B = C = 1/3, support two pixels. The source
    // neighbourhood is already loaded for clipping and velocity dilation.
    let x = abs(distance);
    if x < 1.0 { return ((7.0*x-12.0)*x*x+16.0/3.0)/6.0; }
    if x < 2.0 { return (((-7.0/3.0*x+12.0)*x-20.0)*x+32.0/3.0)/6.0; }
    return 0.0;
}
fn clip_history(value: vec3<f32>, low: vec3<f32>, high: vec3<f32>) -> vec3<f32> {
    if all(value >= low) && all(value <= high) { return value; }
    let centre = (low+high)*0.5;
    let radius = max((high-low)*0.5,vec3(1e-20));
    let offset = value-centre;
    let normalized = abs(offset/radius);
    return centre+offset/max(1.0,max(max(normalized.x,normalized.y),normalized.z));
}
fn history_at(p: vec2<f32>) -> vec4<f32> {
    let base = vec2<i32>(floor(p));
    let f = fract(p);
    return mix(mix(textureLoad(history,bounded(base),0),textureLoad(history,bounded(base+vec2(1,0)),0),f.x),
               mix(textureLoad(history,bounded(base+vec2(0,1)),0),textureLoad(history,bounded(base+vec2(1,1)),0),f.x),f.y);
}
fn depth_at(p: vec2<f32>, expected: f32) -> vec4<f32> {
    let base = vec2<i32>(floor(p));
    let a = textureLoad(previous_depth,bounded(base),0).r;
    let b = textureLoad(previous_depth,bounded(base+vec2(1,0)),0).r;
    let c = textureLoad(previous_depth,bounded(base+vec2(0,1)),0).r;
    let d = textureLoad(previous_depth,bounded(base+vec2(1,1)),0).r;
    let depths = vec4(a,b,c,d);
    let f = fract(p);
    let weights = vec4((1.0-f.x)*(1.0-f.y),f.x*(1.0-f.y),(1.0-f.x)*f.y,f.x*f.y);
    let low = min(min(a,b),min(c,d));
    let high = max(max(a,b),max(c,d));
    // Reciprocal view depth is affine on a projected plane. Interpolate it
    // instead of accepting every depth between unrelated foreground/background
    // taps; the latter admits stale occluder colour at a disocclusion.
    let projected = 1.0/max(dot(weights,vec4(1.0)/max(depths,vec4(1e-20))),1e-20);
    let errors = abs(depths-vec4(expected));
    // At a discontinuity, a matching surface tap is also a valid witness.
    // Interpolation alone invents a depth between surfaces and rejects both.
    return vec4(low,high,select(projected,0.0,high == 0.0),min(min(errors.x,errors.y),min(errors.z,errors.w)));
}
fn depth_matches(old: vec4<f32>, expected: f32, sky: bool) -> bool {
    let tolerance = max(0.01,expected*0.01);
    return (sky && old.y == 0.0) || (!sky && old.x > 0.0 && min(abs(expected-old.z),old.w) <= tolerance);
}
fn history_round(value: f32, seed: u32) -> f32 {
    // Unbiased quantisation keeps a long temporal average from sticking one
    // half-float ULP away from its mean. Exactly representable values stay exact.
    let bounded_value = clamp(value,0.0,65504.0);
    let nearest_bits = pack2x16float(vec2(bounded_value,0.0)) & 65535u;
    let nearest_value = unpack2x16float(nearest_bits).x;
    if nearest_value == bounded_value { return nearest_value; }
    let lower_bits = select(nearest_bits,nearest_bits-1u,nearest_value > bounded_value);
    let lower = unpack2x16float(lower_bits).x;
    let upper = unpack2x16float(lower_bits+1u).x;
    var hash = seed;
    hash = (hash ^ (hash >> 16u))*2246822519u;
    hash = (hash ^ (hash >> 13u))*3266489917u;
    hash = hash ^ (hash >> 16u);
    let random = f32(hash >> 8u)*(1.0/16777216.0);
    return select(lower,upper,random < (bounded_value-lower)/(upper-lower));
}
@compute @workgroup_size(8,8)
fn reconstruct(@builtin(global_invocation_id) id: vec3<u32>) {
    let extent = textureDimensions(colour);
    if any(id.xy >= extent) { return; }
    let p = vec2<i32>(id.xy);
    var nearest = p;
    var front = textureLoad(depth,p,0);
    var low = vec3(1e30);
    var high = vec3(-1e30);
    let centre = textureLoad(colour,p,0).rgb;
    let centre_ycocg = ycocg(centre);
    var first_moment = vec3(0.0);
    var second_moment = vec3(0.0);
    var source = vec3(0.0);
    var source_weight = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let at = bounded(p+vec2(x,y));
            let z = textureLoad(depth,at,0);
            if z > front { front = z; nearest = at; }
            let sample = textureLoad(colour,at,0).rgb;
            let offset = vec2<f32>(f32(x),f32(y))-params.jitter.xy;
            let filter_weight = reconstruction_filter(offset.x)*reconstruction_filter(offset.y);
            source += (sample-centre)*filter_weight;
            source_weight += filter_weight;
            let c = ycocg(sample);
            low = min(low,c); high = max(high,c);
            let difference = c-centre_ycocg;
            first_moment += difference;
            second_moment += difference*difference;
        }
    }
    // The jittered raster samples p-jitter. Reconstruct at the output pixel
    // centre, including when disocclusion leaves no usable temporal history.
    let filtered = max(vec3(0.0),centre+source/source_weight);
    // A neighbourhood outlier must not give a stale history colour the whole
    // min/max range. Centre-relative moments preserve constant colours exactly.
    let delta_mean = first_moment/9.0;
    let sigma = sqrt(max(vec3(0.0),second_moment/9.0-delta_mean*delta_mean));
    let mean = centre_ycocg+delta_mean;
    low = max(low,mean-sigma);
    high = min(high,mean+sigma);
    let velocity = textureLoad(motion,nearest,0).xy;
    let previous = vec2<f32>(p)+velocity;
    let uv = (vec2<f32>(p)+vec2(0.5)-params.jitter.xy)/vec2<f32>(extent);
    let world_h = params.inverse*vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),textureLoad(depth,p,0),1.0);
    let world = world_h/max(abs(world_h.w),1e-20)*sign(world_h.w);
    let linear_depth = abs((params.view*world).z);
    let expected = abs((params.previous_view*world).z);
    // Depth history is a jittered raster, unlike the resolved colour history.
    let old_depth = depth_at(previous+params.jitter.zw-params.jitter.xy,expected);
    let sky = textureLoad(depth,p,0) == 0.0;
    let depth_ok = depth_matches(old_depth,expected,sky);
    let nominal_history_weight = 0.97;
    var weight = nominal_history_weight*(1.0-textureLoad(reactive,p,0).r);
    if params.state.x == 0.0 || !depth_ok || any(previous < vec2(0.0)) || any(previous > vec2<f32>(extent)-vec2(1.0)) {
        weight = 0.0;
    }
    // With usable history, retain the raw jittered sample for the temporal box
    // estimate. Spatial reconstruction supplies pixels without that support;
    // applying it again to converged history broadens the reference footprint.
    let current = mix(filtered,centre,weight/nominal_history_weight);
    let retained = history_at(previous);
    let rescaled = retained.rgb*(exposure.values.x/max(previous_exposure.values.x,1e-20));
    let retained_ycocg = ycocg(rescaled);
    let bounded_ycocg = clip_history(retained_ycocg,low,high);
    // Preserve the exact value when clipping is unnecessary. A YCoCg round trip
    // and weighted sum can otherwise lose a half-precision ULP on every frame.
    let clipped = select(rgb(bounded_ycocg),rescaled,all(retained_ycocg == bounded_ycocg));
    let luminance_scale = max(exposure.values.x*exposure.values.y,1e-20);
    let a = (1.0-weight)/(1.0+max(0.0,ycocg(current).x/luminance_scale));
    let b = weight/(1.0+max(0.0,ycocg(clipped).x/luminance_scale));
    let resolved = max(vec3(0.0),current+(clipped-current)*(b/max(a+b,1e-20)));
    textureStore(output,p,vec4(resolved,1.0));
    let seed = id.x+id.y*extent.x+bitcast<u32>(params.jitter.x)*1664525u+bitcast<u32>(params.jitter.y)*1013904223u;
    let retained_colour = vec3(history_round(resolved.r,seed),history_round(resolved.g,seed+374761393u),history_round(resolved.b,seed+668265263u));
    textureStore(next_colour,p,vec4(retained_colour,exposure.values.x));
    textureStore(next_depth,p,vec4(select(linear_depth,0.0,sky)));
}
