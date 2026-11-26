    let world = vec3(vertex.x, levels.x, vertex.y);
    let current = view.unjittered_view_proj * vec4(world, 1.0);
    let previous = view.previous_view_proj * vec4(vertex.x, levels.y, vertex.y, 1.0);
