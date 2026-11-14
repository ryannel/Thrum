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
fn history_at(p: vec2<f32>) -> vec4<f32> {
    let base = vec2<i32>(floor(p));
    let f = fract(p);
    return mix(mix(textureLoad(history,bounded(base),0),textureLoad(history,bounded(base+vec2(1,0)),0),f.x),
               mix(textureLoad(history,bounded(base+vec2(0,1)),0),textureLoad(history,bounded(base+vec2(1,1)),0),f.x),f.y);
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
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let at = bounded(p+vec2(x,y));
            let z = textureLoad(depth,at,0);
            if z > front { front = z; nearest = at; }
            let c = ycocg(textureLoad(colour,at,0).rgb);
            low = min(low,c); high = max(high,c);
        }
    }
    let current = textureLoad(colour,p,0).rgb;
    let velocity = textureLoad(motion,nearest,0).xy;
    let previous = vec2<f32>(p)+velocity;
    let uv = (vec2<f32>(p)+vec2(0.5)-params.jitter.xy)/vec2<f32>(extent);
    let world_h = params.inverse*vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),textureLoad(depth,p,0),1.0);
    let world = world_h/max(abs(world_h.w),1e-20)*sign(world_h.w);
    let linear_depth = abs((params.view*world).z);
    let expected = abs((params.previous_view*world).z);
    // Depth history is a jittered raster, unlike the resolved colour history.
    let depth_at = previous+params.jitter.zw-params.jitter.xy;
    let old_depth = textureLoad(previous_depth,bounded(vec2<i32>(round(depth_at))),0).r;
    let sky = textureLoad(depth,p,0) == 0.0;
    let depth_ok = (sky && old_depth == 0.0) || (!sky && abs(expected-old_depth) <= max(0.01,expected*0.01));
    var weight = 0.95*(1.0-textureLoad(reactive,p,0).r);
    if params.state.x == 0.0 || !depth_ok || any(previous < vec2(0.0)) || any(previous > vec2<f32>(extent)-vec2(1.0)) {
        weight = 0.0;
    }
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
    textureStore(output,p,vec4(resolved,1.0));
    textureStore(next_colour,p,vec4(resolved,exposure.values.x));
    textureStore(next_depth,p,vec4(select(linear_depth,0.0,sky)));
}
