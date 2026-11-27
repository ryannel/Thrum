    let flow = surface_flow(spacing, row, motion);
    f.displacement.y += flow.x;
    f.velocity.y += flow.y;
