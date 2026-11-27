    // Ripples pull a longer optical path through the aerated surface where
    // the resolved crest is lifting. Both inputs are continuous across cells.
    let crest_path = smoothstep(0.004, 0.036, abs(wake.w))
        * smoothstep(0.001, 0.011, wake.x)
        * smoothstep(0.008, 0.10, paths.edge_depth);
    let path_scatter = mix(0.16, 0.46, whitewater.x) * (1.0 - 0.70 * whitewater.y);
    light += incident * water.scatter.rgb * (1.0 - fresnel) * (crest_path * path_scatter);
    // Dense pockets of fresh air at the impact scatter close to white.
    // Diffuse bubbles farther down the pool retain the water's pale tint.
    let turning = smoothstep(0.021, 0.18, abs(wake.w));
    let bubbles = smoothstep(0.14, 0.65, whitewater.x) * (0.54 + 0.27 * turning + 0.19 * whitewater.y);
    let path = 1.0 / (0.37 + 0.63 * nv);
    let boil = 1.0 - exp(-1.85 * bubbles * path);
    light = mix(light, incident * vec3(0.84, 0.90, 0.89) / 3.14159265, boil);
