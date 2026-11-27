    // Ripples pull a longer optical path through the aerated surface where
    // the resolved crest is lifting. Both inputs are continuous across cells.
    let crest_path = smoothstep(0.004, 0.036, abs(wake.w))
        * smoothstep(0.001, 0.011, wake.x)
        * smoothstep(0.008, 0.10, paths.edge_depth);
    let path_scatter = mix(0.16, 0.46, whitewater.x) * (1.0 - 0.70 * whitewater.y);
    light += incident * water.scatter.rgb * (1.0 - fresnel) * (crest_path * path_scatter);
    // Dense pockets of fresh air at the impact scatter close to white.
    // Diffuse bubbles farther down the pool retain the water's pale tint.
    let boil = smoothstep(0.31, 0.72, whitewater.x) * (0.49 + 0.35 * whitewater.y);
    light = mix(light, incident * vec3(0.84, 0.90, 0.89) / 3.14159265, boil);
