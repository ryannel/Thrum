@group(0) @binding(0) var colour: texture_2d<f32>;
@group(0) @binding(1) var depth: texture_depth_2d;
@group(0) @binding(2) var motion: texture_2d<f32>;
@group(0) @binding(3) var reactive: texture_2d<f32>;
@group(0) @binding(4) var history: texture_2d<f32>;
@group(0) @binding(5) var previous_depth: texture_2d<f32>;
@group(0) @binding(6) var output: texture_storage_2d<rgba16float, write>;
@group(0) @binding(7) var next_colour: texture_storage_2d<rgba16float, write>;
@group(0) @binding(8) var next_depth: texture_storage_2d<r32float, write>;
@group(0) @binding(9) var<uniform> params: ReconstructionParams;
@group(0) @binding(10) var<uniform> exposure: ExposureSample;
@group(0) @binding(11) var<uniform> previous_exposure: ExposureSample;

fn ycocg(c: vec3<f32>) -> vec3<f32> {
    return vec3(dot(c,vec3(0.25,0.5,0.25)),(c.r-c.b)*0.5,(-c.r+2.0*c.g-c.b)*0.25);
}
fn rgb(c: vec3<f32>) -> vec3<f32> { return vec3(c.x+c.y-c.z,c.x+c.z,c.x-c.y-c.z); }
fn bounded(p: vec2<i32>, extent: vec2<u32>) -> vec2<i32> {
    return clamp(p,vec2(0),vec2<i32>(extent)-vec2(1));
}
fn reconstruction_filter(distance: f32) -> f32 {
    // Mitchell-Netravali, B = C = 1/3, support two pixels. The source
    // neighbourhood is already loaded for clipping and velocity dilation.
    let x = abs(distance);
    if x < 1.0 { return ((7.0*x-12.0)*x*x+16.0/3.0)/6.0; }
    if x < 2.0 { return (((-7.0/3.0*x+12.0)*x-20.0)*x+32.0/3.0)/6.0; }
    return 0.0;
}
fn history_filter(distance: f32) -> f32 {
    // Interpolating cubic Hermite (Catmull-Rom). Unlike bilinear reprojection,
    // repeated fractional translations retain resolved spatial detail.
    let x = abs(distance);
    if x < 1.0 { return (1.5*x-2.5)*x*x+1.0; }
    if x < 2.0 { return ((-0.5*x+2.5)*x-4.0)*x+2.0; }
    return 0.0;
}
fn history_at(p: vec2<f32>) -> vec4<f32> {
    let base = vec2<i32>(floor(p));
    let f = fract(p);
    let centre = textureLoad(history,bounded(base,textureDimensions(history)),0);
    var difference = vec4(0.0);
    for (var y = -1; y <= 2; y++) {
        let wy = history_filter(f32(y)-f.y);
        for (var x = -1; x <= 2; x++) {
            let weight = history_filter(f32(x)-f.x)*wy;
            difference += (textureLoad(history,bounded(base+vec2(x,y),textureDimensions(history)),0)-centre)*weight;
        }
    }
    // Exact constant preservation also prevents a storage-scale drift. Existing
    // YCoCg neighbourhood clipping bounds the cubic's negative-lobe overshoot.
    return centre+difference;
}
fn depth_at(p: vec2<f32>, expected: f32, extent: vec2<u32>) -> vec4<f32> {
    let base = vec2<i32>(floor(p));
    let a = textureLoad(previous_depth,bounded(base,extent),0).r;
    let b = textureLoad(previous_depth,bounded(base+vec2(1,0),extent),0).r;
    let c = textureLoad(previous_depth,bounded(base+vec2(0,1),extent),0).r;
    let d = textureLoad(previous_depth,bounded(base+vec2(1,1),extent),0).r;
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
    let tolerance = max(0.01,expected*0.01);
    // At a discontinuity, the matching taps support only their share of the
    // footprint. One matching tap must not fully admit mixed colour history.
    let support = dot(weights,select(vec4(0.0),vec4(1.0),errors <= vec4(tolerance)));
    return vec4(low,high,select(projected,0.0,high == 0.0),support);
}
fn depth_confidence(old: vec4<f32>, expected: f32, sky: bool) -> f32 {
    if sky { return select(0.0,1.0,old.y == 0.0); }
    if old.x <= 0.0 { return 0.0; }
    let tolerance = max(0.01,expected*0.01);
    // A projected plane has continuous reciprocal depth even when no corner
    // has the queried depth. Preserve its full temporal support.
    return select(old.w,1.0,abs(expected-old.z) <= tolerance);
}
fn history_round(value: f32, seed: u32) -> f32 {
    // Unbiased quantisation keeps a long temporal average from sticking one
    // half-float ULP away from its mean. Exactly representable values stay exact.
    let bounded_value = clamp(value,0.0,MAX_FINITE_HALF);
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
fn world_at(p: vec2<i32>, depth_value: f32) -> vec4<f32> {
    let uv = (vec2<f32>(p)+vec2(0.5)-params.jitter.xy)/params.state.zw;
    let world_h = params.inverse*vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),depth_value,1.0);
    return world_h/max(abs(world_h.w),1e-20)*sign(world_h.w);
}

@compute @workgroup_size(RECONSTRUCTION_WORKGROUP_EDGE, RECONSTRUCTION_WORKGROUP_EDGE)
fn reconstruct(@builtin(global_invocation_id) id: vec3<u32>) {
    let output_extent = textureDimensions(output);
    if any(id.xy >= output_extent) { return; }
    let extent = vec2<u32>(params.state.zw);
    let same_grid = all(extent == output_extent);
    let output_pixel = vec2<f32>(id.xy);
    let render_per_output = vec2<f32>(extent)/vec2<f32>(output_extent);
    let input_pixel = select((output_pixel+0.5)*render_per_output-0.5,output_pixel,same_grid);
    let p = bounded(vec2<i32>(floor(input_pixel+0.5)),extent);
    let phase_offset = input_pixel-vec2<f32>(p);
    var nearest = p;
    var front = textureLoad(depth,p,0);
    var low = vec3(1e30);
    var high = vec3(-1e30);
    let centre = textureLoad(colour,p,0).rgb;
    var source = vec3(0.0);
    var source_weight = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let at = bounded(p+vec2(x,y),extent);
            let z = textureLoad(depth,at,0);
            if z > front { front = z; nearest = at; }
            let sample = textureLoad(colour,at,0).rgb;
            let offset = vec2<f32>(f32(x),f32(y))-params.jitter.xy-phase_offset;
            let filter_weight = reconstruction_filter(offset.x)*reconstruction_filter(offset.y);
            source += (sample-centre)*filter_weight;
            source_weight += filter_weight;
            let c = ycocg(sample);
            low = min(low,c); high = max(high,c);
        }
    }
    // The jittered raster samples p-jitter. Reconstruct at the output pixel
    // centre, including when disocclusion leaves no usable temporal history.
    let filtered = max(vec3(0.0),centre+source/source_weight);
    let velocity = textureLoad(motion,nearest,0).xy;
    let previous = output_pixel+velocity/render_per_output;
    let source_depth = textureLoad(depth,p,0);
    let world = world_at(p,source_depth);
    let expected = abs((params.previous_view*world).z);
    // Colour history is resolved on the output grid. Depth history is a jittered raster
    // on the previous drawn grid. Reproject the source depth sample in its own grid.
    let previous_render = vec2<f32>(p)+velocity-params.jitter.xy;
    let depth_ratio = params.previous_extent.xy/vec2<f32>(extent);
    let depth_pixel = (previous_render+0.5)*depth_ratio-0.5+params.jitter.zw;
    let native_depth_pixel = previous+params.jitter.zw-params.jitter.xy;
    let same_depth_grid = same_grid && all(params.previous_extent.xy == vec2<f32>(extent));
    let old_depth = depth_at(select(depth_pixel,native_depth_pixel,same_depth_grid),expected,vec2<u32>(params.previous_extent.xy));
    let sky = source_depth == 0.0;
    let support = depth_confidence(old_depth,expected,sky);
    let nominal_history_weight = 0.97;
    var weight = nominal_history_weight*support*(1.0-textureLoad(reactive,p,0).r);
    if params.state.x == 0.0 || any(previous < vec2(0.0)) || any(previous > vec2<f32>(output_extent)-vec2(1.0)) {
        weight = 0.0;
    }
    // With usable history, retain the raw jittered sample for the temporal box
    // estimate. Spatial reconstruction supplies pixels without that support;
    // applying it again to converged history broadens the reference footprint.
    let current = mix(filtered,centre,select(0.0,weight/nominal_history_weight,same_grid));
    let retained = history_at(previous);
    let rescaled = retained.rgb*(exposure.values.x/max(previous_exposure.values.x,1e-20));
    let retained_ycocg = ycocg(rescaled);
    let bounded_ycocg = clamp(retained_ycocg,low,high);
    // Preserve the exact value when clipping is unnecessary. A YCoCg round trip
    // and weighted sum can otherwise lose a half-precision ULP on every frame.
    let clipped = select(rgb(bounded_ycocg),rescaled,all(retained_ycocg == bounded_ycocg));
    let luminance_scale = max(exposure.values.x*exposure.values.y,1e-20);
    let a = (1.0-weight)/(1.0+max(0.0,ycocg(current).x/luminance_scale));
    let b = weight/(1.0+max(0.0,ycocg(clipped).x/luminance_scale));
    let resolved = max(vec3(0.0),current+(clipped-current)*(b/max(a+b,1e-20)));
    textureStore(output,vec2<i32>(id.xy),vec4(resolved,1.0));
    let seed = id.x+id.y*output_extent.x+bitcast<u32>(params.jitter.x)*1664525u+bitcast<u32>(params.jitter.y)*1013904223u;
    let retained_colour = vec3(history_round(resolved.r,seed),history_round(resolved.g,seed+374761393u),history_round(resolved.b,seed+668265263u));
    textureStore(next_colour,vec2<i32>(id.xy),vec4(retained_colour,exposure.values.x));
    // Each invocation owns one output pixel and, inside the input rectangle,
    // one depth texel. The rest of the retained depth allocation is unreadable.
    if all(id.xy < extent) {
        let raster = vec2<i32>(id.xy);
        let z = textureLoad(depth,raster,0);
        let linear_depth = abs((params.view*world_at(raster,z)).z);
        textureStore(next_depth,raster,vec4(select(linear_depth,0.0,z == 0.0)));
    }
}
