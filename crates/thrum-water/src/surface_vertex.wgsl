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
    loss_without_long: f32,
    current: vec2<f32>,
    track: vec4<f32>,
}
struct VertexOut {
    @builtin(position) clip:vec4<f32>, @location(0) world:vec3<f32>,
    @location(1) rest:vec2<f32>, @location(2) level:f32,
    @location(3) current:vec4<f32>, @location(4) previous:vec4<f32>,
    @location(5) @interpolate(flat) body:u32,
    @location(6) surf_weight:f32,
    @location(7) surf_normal_variance:vec4<f32>,
    @location(8) breaking_face_weight:f32,
}

fn sea_surf_normal(index:u32, local:vec2<f32>, spacing:f32)->vec3<f32> {
    // Neighbours stay inside this ring's full grid. Its inner hole skips
    // indices only, so the cache also holds the nodes either side of the seam.
    let grid_radius = sea_grid_radius(spacing);
    let row = 2u*grid_radius+1u;
    let radius = f32(grid_radius);
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
