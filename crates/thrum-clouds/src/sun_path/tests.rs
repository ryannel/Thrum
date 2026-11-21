use super::Uniform;

#[test]
fn sun_shader_and_uniform_layout_validate() {
    for source in [super::SHADER, super::GROUND_REFERENCE_SHADER] {
        validate_uniform_layout(source);
    }
}

fn validate_uniform_layout(source: &str) {
    use wgpu::naga;
    let module = naga::front::wgsl::parse_str(source).expect("Sun path WGSL");
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("valid Sun compute entries");
    thrum_test_support::records::named(
        "Sun path shader",
        source,
        &[thrum_test_support::record!(
            Uniform as "Params",
            weather,
            origin,
            direction,
            physical,
            erosion,
            rows
        )],
    );
}
