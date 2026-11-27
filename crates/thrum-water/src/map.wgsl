@group(WATER_MAP_GROUP) @binding(5) var bed_map: texture_2d_array<f32>;
@group(WATER_MAP_GROUP) @binding(6) var<uniform> map: MapRecord;

struct Cell {
    corners: array<vec2<i32>, 4>,
    weights: vec4<f32>,
    heights: vec4<f32>,
    uv: vec2<f32>,
    bed: f32,
    inside: bool,
}
struct MapRow {
    water: vec3<f32>, // level, current in x/z
    gains: vec4<f32>,
}
struct MapAnswer {
    body: u32,
    row: MapRow,
    bed: f32,
    has_bed: bool,
}
const NO_BODY = 0xffffffffu;
const CORNERS = array(vec2(0, 0), vec2(1, 0), vec2(0, 1), vec2(1, 1));
fn map_cell(point: vec2<f32>) -> Cell {
    let last = vec2<f32>(map.size_count_sea.xy - vec2(1u));
    let grid = (point - map.origin_spacing_margin.xy) / map.origin_spacing_margin.z;
