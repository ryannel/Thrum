//! Reflect the same WGSL recipes that the renderer uses to create its shader modules.

use std::{collections::BTreeSet, fs, path::Path};

use thrum_render::RecordLayout;
use thrum_test_support::records;
use wgpu::naga::valid::{Capabilities, ValidationFlags, Validator};

/// The kit's record check, then naga's validation of the whole composed shader, which the renderer's check does
/// beyond reflection.
fn check<'a>(source: &str, name: &str, layouts: &'a [RecordLayout]) -> BTreeSet<&'a str> {
    let records = records::bound(name, source, layouts);
    let module = records::parse(name, source);
    Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .unwrap_or_else(|err| panic!("{name}: {}", err.emit_to_string(source)));
    records
}

#[test]
fn all_production_buffer_structs_match_rust_records() {
    let layouts = thrum_render::record_layouts();
    let mut checked = BTreeSet::new();
    for (name, source) in thrum_render::composed_sources() {
        checked.extend(check(&source, name, &layouts));
    }
    assert_eq!(checked, layouts.iter().map(|layout| layout.name).collect());
    assert_eq!(
        checked.len(),
        layouts.len(),
        "duplicate host layout registration"
    );
    println!("Reflected {} host records", checked.len());
}

#[test]
fn production_shader_creation_goes_through_the_catalog() {
    fn check_directory(dir: &Path) {
        for entry in fs::read_dir(dir).expect("renderer source directory") {
            let path = entry.expect("renderer source entry").path();
            if path.is_dir() && path.file_name().is_some_and(|name| name != "tests") {
                check_directory(&path);
            }
            let Some(stem) = path.file_stem().and_then(|name| name.to_str()) else {
                continue;
            };
            if path.extension().is_some_and(|ext| ext == "rs")
                && stem != "shaders"
                && stem != "tests"
                && !stem.ends_with("_tests")
            {
                let source = fs::read_to_string(&path).expect("renderer source");
                assert!(
                    !source.contains("create_shader_module"),
                    "{} bypasses the reflected shader catalog",
                    path.display()
                );
            }
        }
    }
    check_directory(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
}

#[test]
#[should_panic(expected = "has no Rust Pod record")]
fn a_new_bound_struct_requires_a_host_record() {
    check(
        "struct Unknown { value: vec4<f32> }\n@group(2) @binding(0) var<uniform> probe: Unknown;",
        "probe",
        &thrum_render::record_layouts(),
    );
}

#[test]
#[should_panic(expected = "ViewRecord.padding scalar kind")]
fn a_same_size_type_change_fails() {
    let (_, source) = thrum_render::composed_sources()
        .into_iter()
        .find(|(name, _)| *name == "Scene")
        .expect("production scene source");
    let changed = source.replace("padding: vec2<u32>", "padding: vec2<i32>");
    assert_ne!(source, changed, "mutated the bound view declaration");
    check(&changed, "mutated scene", &thrum_render::record_layouts());
}
