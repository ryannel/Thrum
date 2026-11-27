}
@group(0) @binding(0) var<uniform> settings: Settings;
@group(0) @binding(2) var<storage, read> components: array<Component>;
@group(0) @binding(3) var<storage, read_write> answers: array<Answer>;
@group(0) @binding(4) var cascades: texture_2d_array<f32>;

fn column_component(index: u32) -> Component { return components[index]; }

struct QueryCarriage {
    motion: CascadeMotion,
    gradient: CarriageGradient,
}

fn carriage_gradient(point: QueryPoint) -> CarriageGradient {
    var gradient = CarriageGradient(COLUMN_IDENTITY, COLUMN_IDENTITY, vec2(0.0));
    let track = point.track;
    let derivative = point.track_gradient;
    if track.w > 0.0 {
        let axis = u32(track.w) - 1u;
        gradient.first[0][axis] = derivative.x;
        gradient.first[1][axis] = derivative.z;
        gradient.blend = derivative.yw;
    } else if track.w < 0.0 {
