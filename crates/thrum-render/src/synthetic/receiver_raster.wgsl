@fragment fn receiver_fragment(input: VertexOutput) -> @location(0) u32 {
    let dx = dpdx(input.world_position);
    let dy = dpdy(input.world_position);
    let near = view_aux.near_far.x;
    let far = view_aux.near_far.y;
    let depth = near*far/(near+input.position.z*(far-near));
    let eligible = depth >= shadows.split_overlap.x-shadows.split_overlap.y &&
        (shadows.split_overlap.z <= 0.0 || depth < shadows.split_overlap.z);
    let geometric = normalize(cross(dx, dy));
    if !eligible || !all(abs(geometric) <= vec3(1.0)) { return 0u; }
    let receiver = ShadowReceiver(input.world_position, dx, dy, normalize(input.normal));
    let raw = sample_cascade_window(1u, receiver, vec3(0.0,2.0,2.0), true, input.position.xy);
    if raw.y < 0.0 { return 0u; }
    var oct = geometric.xy/(abs(geometric.x)+abs(geometric.y)+abs(geometric.z));
    if geometric.z < 0.0 {
        oct = (1.0-abs(oct.yx))*select(vec2(-1.0),vec2(1.0),oct>=vec2(0.0));
    }
    let n = vec2<u32>(round(clamp(oct*0.5+0.5,vec2(0.0),vec2(1.0))*255.0));
    return u32(round(clamp(raw.x,0.0,1.0)*4095.0)) | (n.x<<12u) | (n.y<<20u)
        | (1u<<28u) | select(0u,1u<<29u,raw.y>0.0);
}
