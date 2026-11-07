// Stackless traversal of the top-down caster raster and maximum-depth parents.
// Traverse positive grid indices, reflecting integer boundaries for a negative
// direction. Evaluate plane times in original coordinates so reflection cannot
// round a grazing origin onto another side of an edge.
@group(0) @binding(6) var horizon_parents: texture_2d<f32>;

const HORIZON_GRID: i32 = 1024;
const HORIZON_ROOT: u32 = 10u;
// Below this speed, even the full map reach moves less than 4e-14 cells.
const HORIZON_STATIONARY_SPEED: f32 = 1e-18;
const HORIZON_PLANE_TOLERANCE_M: f32 = 0.02;

fn horizon_axis(p: f32, velocity: f32, inverse: f32, reach: f32) -> vec2<f32> {
    if velocity == 0.0 {
        return select(vec2(1.0,0.0),vec2(0.0,reach),p>=0.0 && p<f32(HORIZON_GRID));
    }
    let a = -p*inverse;
    let b = (f32(HORIZON_GRID)-p)*inverse;
    return vec2(min(a,b),max(a,b));
}

fn horizon_plane_time(plane: vec2<f32>, origin: vec2<f32>, inverse: vec2<f32>) -> vec2<f32> {
    let boundary = select(plane,vec2(f32(HORIZON_GRID))-plane,inverse<vec2(0.0));
    return (boundary-origin)*inverse;
}

fn horizon_cell_at(origin: vec2<f32>, velocity: vec2<f32>, inverse: vec2<f32>, t: f32) -> vec2<i32> {
    let base = floor(fma(velocity,vec2(t),origin));
    let cell = select(base,vec2(f32(HORIZON_GRID-1))-base,velocity<vec2(0.0));
    let lo = horizon_plane_time(cell,origin,inverse);
    let hi = horizon_plane_time(cell+vec2(1.0),origin,inverse);
    let moving = velocity!=vec2(0.0);
    return vec2<i32>(cell)-vec2<i32>(moving & (lo>vec2(t)))+
        vec2<i32>(moving & (hi<=vec2(t)));
}

fn horizon_visibility(receiver: ShadowReceiver) -> f32 {
    let horizon = shadows.horizon;
    let l = light.direction_to_light.xyz;
    let flat = length(l.xz);
    if horizon.x <= 0.0 || flat < 1e-4 { return 1.0; }
    let elevation = atan2(l.y,flat);
    let radius = atan(SUN_TAN_RADIUS);
    let lower = tan(elevation-radius);
    let upper = tan(min(elevation+radius,1.5));
    let min_distance = 2.0*horizon.w;
    if lower >= 0.0 && shadows.height_support.x-receiver.position.y <= lower*min_distance {
        return 1.0;
    }
    let reach = 1.0/max(horizon.z,1e-20);
    let m = shadows.height_view_proj;
    let clip = m*vec4(receiver.position.x,0.0,receiver.position.z,1.0);
    let azimuth = l.xz/flat;
    let clip_delta = m*vec4(azimuth.x,0.0,azimuth.y,0.0);
    let receiver_cell = vec2(clip.x+1.0,1.0-clip.y)*(0.5*f32(HORIZON_GRID));
    let grid_velocity = vec2(clip_delta.x,-clip_delta.y)*(0.5*f32(HORIZON_GRID));
    let stationary = abs(grid_velocity)<vec2(HORIZON_STATIONARY_SPEED);
    let velocity = select(grid_velocity,vec2(0.0),stationary);
    // A negative, almost-cardinal direction entering exactly on a plane
    // belongs to the lower cell. An exactly stationary axis stays half-open.
    let lower_cell = stationary & (grid_velocity<vec2(0.0)) &
        (receiver_cell==floor(receiver_cell));
    let origin = select(receiver_cell,receiver_cell-vec2(0.5),lower_cell);
    let flip = velocity<vec2(0.0);
    let inverse = 1.0/select(velocity,vec2(1.0),velocity==vec2(0.0));
    let x = horizon_axis(origin.x,velocity.x,inverse.x,reach);
    let y = horizon_axis(origin.y,velocity.y,inverse.y,reach);
    var start = max(min_distance,max(x.x,y.x));
    let end = min(reach,min(x.y,y.y));
    if start >= end { return 1.0; }

    let scale = 1.0/horizon.x;
    let receiver_depth = fma(horizon.x,receiver.position.y,horizon.y);
    let normal = cross(receiver.dx,receiver.dy);
    let has_plane = abs(normal.y)>max(length(normal)*1e-4,1e-20);
    let gradient = -normal.xz/select(1.0,normal.y,has_plane);
    let projected_gradient = m*vec4(gradient.x,0.0,gradient.y,0.0);
    // The snapped map is orthonormal up to scale, so A inverse-transpose
    // equals A times texel-metres squared for the world-to-cell transform A.
    let gradient_cells = vec2(projected_gradient.x,-projected_gradient.y)*
        (0.5*f32(HORIZON_GRID)*horizon.w*horizon.w);
    let plane_slope = dot(gradient,azimuth);
    var best = lower;
    var cell = clamp(horizon_cell_at(origin,velocity,inverse,start),vec2(0),vec2(HORIZON_GRID-1));
    var level = HORIZON_ROOT;
    loop {
        let coordinate = cell >> vec2(level);
        let high = (coordinate+vec2(1))<<vec2(level);
        let exits = select(horizon_plane_time(vec2<f32>(high),origin,inverse),
                           vec2(reach),velocity==vec2(0.0));
        let exit = min(end,min(exits.x,exits.y));
        let reflected = vec2((HORIZON_GRID >> level)-1)-coordinate;
        let index = select(coordinate,reflected,flip);
        var depth: f32;
        if level == 0u {
            depth = textureLoad(shadow_maps,index,2,0);
        } else {
            depth = textureLoad(horizon_parents,index,i32(level)-1).x;
        }
        if depth > 0.0 && exit > start {
            let rise = (depth-receiver_depth)*scale;
            // Positive slopes reach their maximum at entry, negative at exit.
            let bound = rise/select(exit,start,rise>=0.0);
            if bound > best {
                if level > 0u {
                    level -= 1u;
                    continue;
                }
                var leaf = bound;
                if has_plane && plane_slope < bound {
                    let offset = vec2<f32>(index)+vec2(0.5)-receiver_cell;
                    let residual = abs(rise-dot(gradient_cells,offset));
                    // A matching sample represents the receiver plane, whose
                    // slope does not depend on the clipped cell's endpoints.
                    // A separate caster retains its full conservative height.
                    let keep_flat = smoothstep(HORIZON_PLANE_TOLERANCE_M,
                        2.0*HORIZON_PLANE_TOLERANCE_M,residual);
                    leaf = mix(plane_slope,bound,keep_flat);
                }
                best = max(best,leaf);
                if best >= upper { break; }
            }
        }
        if exit >= end { break; }
        let next = select(clamp(horizon_cell_at(origin,velocity,inverse,exit),cell,high-vec2(1)),
                          high,exits<=vec2(exit));
        if any(next >= vec2(HORIZON_GRID)) { break; }
        // The highest changed bit selects the next sibling's level. At least
        // one axis crossed its tile's high plane, so the cell always advances.
        let change = (next.x ^ cell.x) | (next.y ^ cell.y);
        level = firstLeadingBit(u32(change));
        cell = next;
        start = max(start,exit);
    }
    if best == lower { return 1.0; }
    return disk_above((elevation-atan(best))/radius);
}
