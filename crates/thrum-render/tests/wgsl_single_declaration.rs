//! Count the source files declaring bound structs, independent of composition count.

use std::collections::{BTreeMap, BTreeSet};

use thrum_test_support::records;

const SOURCE: &str = "// thrum-wgsl-source: ";

fn check<'a>(sources: impl IntoIterator<Item = (&'a str, &'a str)>) {
    let mut declarations = BTreeMap::<_, BTreeSet<_>>::new();
    for (shader, source) in sources {
        let module = records::parse(shader, source);
        for ty in records::bound_structs(&module) {
            let name = module.types[ty].name.as_deref().expect("bound struct name");
            let offset = module
                .types
                .get_span(ty)
                .to_range()
                .expect("type source span")
                .start;
            let file = source[..offset]
                .lines()
                .rev()
                .take_while(|line| *line != "// thrum-wgsl-end")
                .find_map(|line| line.strip_prefix(SOURCE))
                .unwrap_or_else(|| panic!("{shader}: {name} needs a WGSL source file"));
            declarations
                .entry(name.to_owned())
                .or_default()
                .insert(file);
        }
    }
    for (name, files) in declarations {
        assert_eq!(
            files.len(),
            1,
            "{name} is declared in multiple source files: {files:?}"
        );
    }
}

#[test]
fn bound_records_have_one_declaration() {
    let sources = thrum_render::composed_sources();
    check(
        sources
            .iter()
            .map(|(name, source)| (*name, source.as_ref())),
    );
}

const FIRST: &str = "// thrum-wgsl-source: first.wgsl\nstruct Record { value: vec4<f32> }\n@group(0) @binding(0) var<uniform> input: Record;";
const SECOND: &str = "// thrum-wgsl-source: second.wgsl\nstruct Record { value: vec4<f32> }\n@group(0) @binding(0) var<uniform> input: Record;";
const LOCAL: &str = "// thrum-wgsl-source: local.wgsl\nstruct Record { unrelated: u32 }";

#[test]
fn repeated_composition_and_an_unbound_namesake_are_allowed() {
    check([
        ("one", FIRST),
        ("another composition", FIRST),
        ("unbound", LOCAL),
    ]);
}

#[test]
#[should_panic(expected = "Record is declared in multiple source files")]
fn copied_bound_declaration_is_rejected() {
    check([("one", FIRST), ("two", SECOND)]);
}
