    let body_index = surface_body(input.body);
    if map.bodies[body_index].layers.x == 3u {
        let sediment = hollows_optical_type(body_index, input.rest);
        let ray = refract(-v, n, 1.0 / 1.333);
        let water_path = paths.edge_depth / max(0.08, -ray.y);
        let scattered = 1.0 - exp(-sediment.w * water_path);
        let sediment_light = incident * sediment.rgb / 3.14159265;
        // The common optics already contain the clear-water path. Sediment
        // attenuates the transmitted bed and returns light along that path.
        light += (1.0 - fresnel) * paths.transmission * scattered * (sediment_light - paths.behind);
    }
