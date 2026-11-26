fn cloud_stop_distance(pixel: vec2<u32>, camera: vec3<f32>) -> f32 {
    let depth = textureLoad(scene_depth, vec2<i32>(pixel), 0);
    if depth == 0.0 { return cloud.heights.w; }
    let uv = (vec2<f32>(pixel) + 0.5) / vec2<f32>(cloud.render_extent);
    let hit = cloud.inverse_raster * vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), depth, 1.0);
    return length(hit.xyz / hit.w - camera);
}

struct CloudStop {
    transport: CloudTransport,
    front: f32,
    opacity: f32,
}

struct CloudTracePart {
    completed: CloudTransport,
    current: CloudTransport,
    weighted: f32,
    opacity: f32,
    part_weighted: f32,
    part_mass: f32,
}

fn cloud_stop(part: CloudTracePart, camera: vec3<f32>, ray: vec3<f32>, uv: vec2<f32>) -> CloudStop {
    let front_distance = part.weighted / max(part.opacity, 1e-10);
    let total = cloud_then(part.completed, cloud_air_part(part.current, part.part_weighted,
        part.part_mass, camera, ray, uv));
    return CloudStop(total, front_distance, part.opacity);
}

// A stop inside a march step uses the same coefficient sample as the full step,
// over only the distance in front of the surface. The live march is unchanged.
fn cloud_partial_stop(part: CloudTracePart, sigma: f32, radiance: vec3<f32>, start: f32, width: f32,
                      camera: vec3<f32>, ray: vec3<f32>, uv: vec2<f32>) -> CloudStop {
    var clipped = part;
    if width > 0.0 {
        let segment = cloud_beer(sigma, radiance, width);
        let local = part.current.transmittance * cloud_removed(segment.optical_depth);
        let amount = part.completed.transmittance * local;
        let midpoint = start + width * 0.5;
        clipped.weighted += amount * midpoint;
        clipped.opacity += amount;
        clipped.part_weighted += local * midpoint;
        clipped.part_mass += local;
        clipped.current = cloud_then(part.current, segment);
    }
    return cloud_stop(clipped, camera, ray, uv);
}

fn cloud_write_stop(texel: vec2<i32>, slot: i32, stop: CloudStop, samples: u32, distance: f32) {
    textureStore(transport, texel, slot,
                 vec4(stop.transport.scattering * environment_storage_scale(), stop.transport.transmittance));
    // Both distances are kilometres to remain representable in fp16.
    textureStore(front, texel, slot, vec4(stop.front * 0.001, stop.opacity, f32(samples), distance));
}
