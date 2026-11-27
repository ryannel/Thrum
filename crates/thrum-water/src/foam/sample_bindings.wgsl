struct FoamHull {
    location_axis: vec4<f32>,
    extent_rate: vec4<f32>,
    velocity: vec4<f32>,
}
struct FoamSettings {
    origins: array<vec4<i32>, 4>,
    banks: vec4<u32>,
    time_wind: vec4<f32>,
    camera: vec4<f32>,
    jacobian_variance: vec4<f32>,
    long_bounds: vec4<f32>,
    hulls: array<FoamHull, HULL_LIMIT>,
    components: array<Component, COMPONENT_LIMIT>,
}
struct FoamEndpoints {
    stocks: array<vec4<f32>, FOAM_STOCK_RECORDS>,
    coarse_deficit: array<f32, FOAM_HANDOFF_CELLS>,
}
@group(3) @binding(9) var<storage, read> foam_stocks: FoamEndpoints;
@group(3) @binding(10) var<uniform> foam_settings: FoamSettings;

