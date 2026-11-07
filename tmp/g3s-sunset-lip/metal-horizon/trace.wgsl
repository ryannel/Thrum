struct Ray {
    origin_dir: vec4<f32>,
    height_sun_scale_bias: vec4<f32>,
    limbs_ceiling: vec4<f32>,
}

@group(0) @binding(0) var base: texture_depth_2d;
@group(0) @binding(1) var parents: texture_2d<f32>;
@group(0) @binding(2) var<storage, read> rays: array<Ray>;
@group(0) @binding(3) var<storage, read_write> output: array<vec4<f32>>;

const ROOT: u32 = 10u;
const START: f32 = 64.0;
const REACH: f32 = 32768.0;
const RADIUS: f32 = 0.00465421;

fn fraction(ray: Ray, slope: f32) -> f32 {
    let c = clamp((ray.height_sun_scale_bias.y-atan(slope))/RADIUS,-1.0,1.0);
    return 0.5+(c*sqrt(max(0.0,1.0-c*c))+asin(c))/3.14159265;
}

fn axis(p: f32, velocity: f32, inverse: f32, low: f32, high: f32) -> vec2<f32> {
    if velocity == 0.0 {
        // Half-open ownership matches nearest texel lookup along a grid face.
        return select(vec2(1.0,0.0),vec2(0.0,REACH),p>=low && p<high);
    }
    let a = (low-p)*inverse;
    let b = (high-p)*inverse;
    return vec2(min(a,b),max(a,b));
}

fn visit(ray: Ray) -> vec4<f32> {
    let direction = ray.origin_dir.zw;
    let inv = 1.0/select(direction,vec2(1.0),direction==vec2(0.0));
    let flip = vec2<i32>(direction<vec2(0.0));
    let lower = ray.limbs_ceiling.x;
    let upper = ray.limbs_ceiling.y;
    var cell = vec2<i32>(0);
    var level = ROOT;
    var best = lower;
    var tests = 0u;
    var reads = 0u;
    var leaves = 0u;
    loop {
        tests += 1u;
        let width = f32(1u<<level);
        let low = vec2<f32>(cell)*width;
        let x = axis(ray.origin_dir.x,direction.x,inv.x,low.x,low.x+width);
        let y = axis(ray.origin_dir.y,direction.y,inv.y,low.y,low.y+width);
        let start = max(START,max(x.x,y.x));
        let end = min(REACH,min(x.y,y.y));
        if start < end {
            var depth: f32;
            if level == 0u {
                depth = textureLoad(base,cell,0);
            } else {
                depth = textureLoad(parents,cell,i32(level)-1).x;
            }
            reads += 1u;
            if depth > 0.0 {
                let rise = depth*ray.height_sun_scale_bias.z+
                    ray.height_sun_scale_bias.w-ray.height_sun_scale_bias.x;
                let bound = rise/select(end,start,rise>=0.0);
                if bound > best {
                    if level > 0u {
                        cell = cell*2+flip;
                        level -= 1u;
                        continue;
                    }
                    leaves += 1u;
                    best = bound;
                    if best >= upper { break; }
                }
            }
        }
        // The cell coordinate carries its ancestry. Ordering the children by
        // travel direction visits intersected quadrants in ray order.
        loop {
            if level == ROOT { return vec4(fraction(ray,best),f32(reads),f32(tests),f32(leaves)); }
            let rank = (cell & vec2(1)) ^ flip;
            let next = rank.x+2*rank.y+1;
            if next < 4 {
                cell = (cell & vec2(-2)) + (vec2(next & 1,next >> 1) ^ flip);
                break;
            }
            cell >>= vec2(1u);
            level += 1u;
        }
    }
    return vec4(fraction(ray,best),f32(reads),f32(tests),f32(leaves));
}

fn tap(ray: Ray) -> vec4<f32> {
    var distance = START;
    let step = pow(REACH/START,1.0/95.0);
    let ceiling = ray.limbs_ceiling.z-ray.height_sun_scale_bias.x;
    let upper = ray.limbs_ceiling.y;
    var best = ray.limbs_ceiling.x;
    var reads = 0u;
    for (var i = 0u; i < 96u; i++) {
        if best >= upper || (best >= 0.0 && ceiling <= best*distance) { break; }
        let p = ray.origin_dir.xy + ray.origin_dir.zw*distance;
        if any(p<vec2(0.0)) || any(p>vec2(1024.0)) { break; }
        let depth = textureLoad(base,min(vec2<i32>(p),vec2(1023)),0);
        reads += 1u;
        if depth > 0.0 {
            let rise = depth*ray.height_sun_scale_bias.z+
                ray.height_sun_scale_bias.w-ray.height_sun_scale_bias.x-4.0;
            best = max(best,rise/distance);
        }
        distance *= step;
    }
    return vec4(fraction(ray,best),f32(reads),0.0,0.0);
}

@compute @workgroup_size(64)
fn tree(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&rays) { output[id.x] = visit(rays[id.x]); }
}

@compute @workgroup_size(64)
fn legacy(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&rays) { output[id.x] = tap(rays[id.x]); }
}

// Traverse in increasing grid indices while evaluating plane times in
// original coordinates. Reflecting the float origin itself can move a
// grazing ray onto another side of a plane near the map's outer edge.
fn plane_time(plane: vec2<f32>, origin: vec2<f32>, inv: vec2<f32>) -> vec2<f32> {
    return (select(plane,vec2(1024.0)-plane,inv<vec2(0.0))-origin)*inv;
}

fn cell_at(origin: vec2<f32>, velocity: vec2<f32>, inv: vec2<f32>, t: f32) -> vec2<i32> {
    let base = floor(fma(velocity,vec2(t),origin));
    let cell = select(base,vec2(1023.0)-base,velocity<vec2(0.0));
    let lo = plane_time(cell,origin,inv);
    let hi = plane_time(cell+vec2(1.0),origin,inv);
    let moving = velocity!=vec2(0.0);
    return vec2<i32>(cell)-vec2<i32>(moving & (lo>vec2(t)))+
        vec2<i32>(moving & (hi<=vec2(t)));
}

fn traverse(ray: Ray) -> vec4<f32> {
    let velocity = ray.origin_dir.zw;
    let flip = velocity<vec2(0.0);
    let origin = ray.origin_dir.xy;
    let inv = 1.0/select(velocity,vec2(1.0),velocity==vec2(0.0));
    let x = axis(origin.x,velocity.x,inv.x,0.0,1024.0);
    let y = axis(origin.y,velocity.y,inv.y,0.0,1024.0);
    var start = max(START,max(x.x,y.x));
    let end = min(REACH,min(x.y,y.y));
    var best = ray.limbs_ceiling.x;
    let upper = ray.limbs_ceiling.y;
    if start >= end { return vec4(fraction(ray,best),0.0,0.0,0.0); }
    var cell = clamp(cell_at(origin,velocity,inv,start),vec2(0),vec2(1023));
    var level = ROOT;
    var reads = 0u;
    var leaves = 0u;
    loop {
        let coordinate = cell >> vec2(level);
        let high = (coordinate+vec2(1))<<vec2(level);
        let exits = select(plane_time(vec2<f32>(high),origin,inv),vec2(REACH),velocity==vec2(0.0));
        let exit = min(end,min(exits.x,exits.y));
        let reflected = vec2(i32((1024u >> level)-1u))-coordinate;
        let index = select(coordinate,reflected,flip);
        var depth: f32;
        if level == 0u {
            depth = textureLoad(base,index,0);
        } else {
            depth = textureLoad(parents,index,i32(level)-1).x;
        }
        reads += 1u;
        if depth > 0.0 && exit > start {
            let rise = depth*ray.height_sun_scale_bias.z+
                ray.height_sun_scale_bias.w-ray.height_sun_scale_bias.x;
            let bound = rise/select(exit,start,rise>=0.0);
            if bound > best {
                if level > 0u {
                    level -= 1u;
                    continue;
                }
                leaves += 1u;
                best = bound;
                if best >= upper { break; }
            }
        }
        if exit >= end { break; }
        // The plane that ended this tile owns the cell transition. Use its
        // integer boundary directly and reproject only the other axis.
        let next = select(clamp(cell_at(origin,velocity,inv,exit),cell,high-vec2(1)),
                          high,exits<=vec2(exit));
        if any(next >= vec2(1024)) { break; }
        let change = (next.x ^ cell.x) | (next.y ^ cell.y);
        level = firstLeadingBit(u32(change));
        cell = next;
        start = max(start,exit);
    }
    return vec4(fraction(ray,best),f32(reads),f32(reads),f32(leaves));
}

@compute @workgroup_size(64)
fn dda(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x < arrayLength(&rays) { output[id.x] = traverse(rays[id.x]); }
}
