struct BeaconVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) @interpolate(flat) face: u32,
}
@group(2) @binding(0) var<uniform> beacon: vec4<f32>;

@vertex fn beacon_vertex(@builtin(vertex_index) vertex: u32) -> BeaconVertex {
    let corners = array<u32, 36>(1u, 3u, 7u, 1u, 7u, 5u, 0u, 4u, 6u, 0u, 6u, 2u,
                                 2u, 6u, 7u, 2u, 7u, 3u, 0u, 1u, 5u, 0u, 5u, 4u,
                                 4u, 5u, 7u, 4u, 7u, 6u, 0u, 2u, 3u, 0u, 3u, 1u);
    let c = corners[vertex];
    let corner = vec3(f32(c & 1u), f32((c >> 1u) & 1u), f32((c >> 2u) & 1u)) - vec3(0.5);
    let world = beacon.xyz + corner * vec3(0.75, 1.35, 0.58);
    return BeaconVertex(view.view_proj * vec4(world, 1.0), world, vertex / 6u);
}

fn beacon_colour(input: BeaconVertex) -> vec4<f32> {
    let sides = array<f32, 6>(0.6, 0.4, 1.0, 0.2, 0.7, 0.43);
    let warm = vec3(175.0, 55.0, 8.0) * sides[input.face];
    let radiance = select(warm * 0.08, warm, input.world.y > 0.83);
    let colour = aerial_colour(radiance, input.world, input.position);
    return vec4(min(colour * scene_storage_scale(), vec3(65504.0)), 1.0);
}
