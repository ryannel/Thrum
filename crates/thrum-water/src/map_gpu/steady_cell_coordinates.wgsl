    let inside = all(grid >= vec2(0.0)) && all(grid <= last);
    var cell: Cell;
    cell.inside = inside;
    if !inside { return cell; }
    let first = vec2<i32>(min(floor(grid), last - vec2(1.0)));
    let uv = grid - vec2<f32>(first);
