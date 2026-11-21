//! Every bound struct is discovered from the actual shader, not from a list of shader names.

fn check(source: &str) {
    let layouts = thrum_sea::records::layouts();
    assert!(!thrum_test_support::records::bound("sea", source, &layouts).is_empty());
}

#[test]
fn every_bound_struct_has_equal_host_layout() {
    check(thrum_sea::SHADER);
    check(thrum_sea::FILTERED_SHADER);
}

#[test]
#[should_panic(expected = "has no Rust Pod record")]
fn a_new_bound_record_cannot_escape_reflection() {
    check(&format!(
        "{}\nstruct Unregistered {{ value: vec4<f32> }}\n@group(2) @binding(0) var<uniform> new_record: Unregistered;",
        thrum_sea::SHADER
    ));
}

#[test]
#[should_panic(expected = "Params size")]
fn a_changed_bound_record_cannot_escape_reflection() {
    check(&thrum_sea::SHADER.replace(
        "lengths: vec4<f32>,",
        "lengths: vec4<f32>, extra: vec4<f32>,",
    ));
}
