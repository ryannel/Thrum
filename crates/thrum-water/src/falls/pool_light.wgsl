    // A convex crest lengthens the optical path through the receiving water.
    // Resolve its curvature in world coordinates before adding scatter.
    let rest_jacobian = dx.x * dy.y - dx.y * dy.x;
    let concavity = -(dy.y * wake_dx.x - dx.y * wake_dy.x
        + dx.x * wake_dy.y - dy.x * wake_dx.y) * sign(rest_jacobian)
        / max(abs(rest_jacobian), 1e-8);
    let crest_path = smoothstep(0.0012, 0.012, concavity)
        * smoothstep(0.0007, 0.005, wake.x)
        * smoothstep(0.008, 0.10, paths.edge_depth);
    let path_scatter = mix(0.42, 0.70, whitewater.x) * (1.0 - 0.70 * whitewater.y);
    light += incident * water.scatter.rgb * (1.0 - fresnel) * (crest_path * path_scatter);
