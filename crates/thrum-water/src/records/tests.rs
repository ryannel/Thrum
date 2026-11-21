use super::Parameters;
use thrum_test_support::{record, records};

/// One struct declaration from a shader that does not parse alone.
fn declaration(shader: &str, name: &str) -> String {
    let body = shader
        .split_once(&format!("struct {name} "))
        .expect("record")
        .1
        .split_once('}')
        .expect("struct closes")
        .0;
    format!("struct {name} {body}}}")
}

#[test]
fn lent_wave_records_match_the_seas_host_layouts() {
    let layouts = thrum_sea::records::layouts();
    let field: Vec<_> = layouts.into_iter().filter(|l| l.name == "Field").collect();
    assert_eq!(field.len(), 1, "sea host record Field");
    records::named(
        "water surface Field",
        &declaration(include_str!("../surface.wgsl"), "Field"),
        &field,
    );
}
#[test]
fn gpu_parameter_size_and_member_offsets_match_the_actual_shader() {
    records::named(
        "water field Parameters",
        &declaration(include_str!("../field.wgsl"), "Parameters"),
        &[record!(
            Parameters,
            level_dt_reset,
            gains,
            lengths,
            cache,
            long_covariance,
            scatter,
            heights
        )],
    );
}
