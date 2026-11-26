@group(0) @binding(1) var<storage, read> points: array<QueryPoint>;

fn query_point(index: u32) -> QueryPoint { return points[index]; }

fn query_carriage(_index: u32, point: QueryPoint) -> QueryCarriage {
    return local_query_carriage(point);
}
