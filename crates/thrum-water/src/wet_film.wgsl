fn film_world(pixel: vec2<f32>, depth: f32) -> vec3<f32> {
    let uv = pixel * view_aux.viewport_inv_viewport.zw;
    let clip = vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), depth, 1.0);
    let homogeneous = view_aux.inverse_view_proj * clip;
    return homogeneous.xyz / max(homogeneous.w, 1e-9);
}

fn film_tangent(pixel: vec2<f32>, ground: vec3<f32>, offset: vec2<f32>) -> vec3<f32> {
    let nearby = pixel + offset;
    let depth = textureLoad(shore_depth, vec2<i32>(nearby), 0);
    let diff = film_world(nearby, depth) - ground;
    if depth < 1e-7 || length(diff) > 0.12 { return vec3(0.0); }
    return diff;
}

fn film_normal(pixel: vec2<f32>, ground: vec3<f32>) -> vec3<f32> {
    let tx = film_tangent(pixel, ground, vec2(1.0, 0.0));
    let ty = film_tangent(pixel, ground, vec2(0.0, 1.0));
    let plane = cross(tx, ty);
    if dot(plane, plane) < 1e-16 { return vec3(0.0, 1.0, 0.0); }
    return normalize(plane * select(-1.0, 1.0, plane.y >= 0.0));
}

fn film_footprint(ground: vec3<f32>, normal: vec3<f32>) -> f32 {
    let eye = view.camera_xyz_time.xyz - ground;
    let range = length(eye);
    let incidence = abs(dot(eye / max(range, 1e-4), normal));
    return range * view_aux.viewport_inv_viewport.w / max(incidence, 0.08);
}

fn standing_film(pixel: vec2<f32>, ground: vec3<f32>, normal: vec3<f32>, wet: GroundWetness) -> Output {
    let appearance = wet_load(wet_class_start(wet.class_index) + 1u);
    let r = appearance.y;
    let v = normalize(view.camera_xyz_time.xyz - ground);
    let still = mix(vec3(0.0, 1.0, 0.0), normal, smoothstep(0.055, 0.24, length(normal.xz)));
    let footprint = distance(view.camera_xyz_time.xyz, ground) * view_aux.viewport_inv_viewport.w;
    let ripple = film_rings(ground, normal, footprint);
    let n = normalize(still - vec3(ripple.x, 0.0, ripple.y));
    let ray = reflect(-v, n);
    let facing = clamp(dot(v, n), 0.0, 1.0);
    let fresnel = water_fresnel(facing);
    let sky = environment_sky_radiance(ray);
    let reach = min(optics.reach_slope_scene.x, 24.0);
    // Near normal incidence the resolved environment supplies the sheen.
    // Reserve scenery rays for the grazing lobe, where their silhouette reads.
    let grazing = 1.0 - smoothstep(0.39, 0.76, facing);
    let clarity = grazing * (1.0 - 0.85 * r) * smoothstep(0.005, 0.09, ray.y);
    var light = sky;
    if clarity > 0.0 && optics.reach_slope_scene.z > 0.0 {
        let origin = ground + vec3(0.0, 0.009, 0.0);
        // The opaque bed is the film's own surface. Use the upward side
        // test to find scenery above it, including the wall and roof.
        let scenery = OpticsBody(vec4(0.0), vec4(0.0));
        let hit = trace(origin, ray, vec2(0.06, reach), scenery, false, vec2(0.0), 1.0);
        let along = dot(hit.point - origin, ray);
        let border = scene_edge(optics_uv(hit.pixel));
        let scene = ReflectionLayer(hit, along, clarity * hit.edge * border * reach_weight(along, reach));
        light = reflect_layer(scene, sky, 1.0);
    }
    let uv = pixel * view_aux.viewport_inv_viewport.zw;
    let air = environment_aerial(ground, uv);
    let colour = (light * air.transmission + air.scatter) * environment_storage_scale();
    return Output(vec4(colour, wet.cover * fresnel * 0.73), vec2(0.0), 0.0);
}
