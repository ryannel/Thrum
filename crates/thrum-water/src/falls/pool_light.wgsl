    // A raised crest adds a short optical path through the receiving water.
    // Suspended air increases its scattering near the active plunge. Use the
    // resolved interaction height so the response travels and dies with it.
    let crest_path = smoothstep(0.001, 0.025, wake.x)
        * smoothstep(0.008, 0.10, paths.edge_depth);
    let path_scatter = mix(0.19, 0.64, whitewater.x) * (1.0 - 0.70 * whitewater.y);
    light += incident * water.scatter.rgb * (1.0 - fresnel) * (crest_path * path_scatter);
