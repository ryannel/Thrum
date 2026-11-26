    let origin = map.origin_spacing_margin.xy;
    let spacing = map.origin_spacing_margin.z;
    let end = fma(last, vec2(spacing), origin);
    let inside = all(point >= origin) && all(point <= end);
    var cell: Cell;
    cell.inside = inside;
    if !inside { return cell; }
    // The map covers the island. Subtract the metre-scale cell origin
    // before normalizing so the fractional coordinate keeps its precision.
    let first_guess = clamp(floor(grid), vec2(0.0), last - 1.0);
    let residual = point - fma(first_guess, vec2(spacing), origin);
    let advance = clamp(floor(residual / spacing), vec2(-1.0), vec2(1.0));
    let index = clamp(first_guess + advance, vec2(0.0), last - 1.0);
    let first = vec2<i32>(index);
    let uv = clamp((point - fma(index, vec2(spacing), origin)) / spacing,
        vec2(0.0), vec2(1.0));
