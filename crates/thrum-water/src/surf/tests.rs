use super::{ShoreBody, SurfGpu, SurfRecord};
use crate::map_gpu;
use thrum_test_support::{record, records};

impl records::Kind for ShoreBody {
    const KIND: Option<records::RecordScalar> = None;
}

const COMPONENT: &str = include_str!("../column/records.wgsl");

#[test]
fn surf_records_match_the_cache_and_query() {
    let source = map_gpu::declarations_at(0)
        + COMPONENT
        + &SurfGpu::source(0, 7)
        + "fn surf_component(_i: u32) -> Component { var c: Component; return c; }";
    records::named(
        "prepared surf shader",
        &source,
        &[
            record!(ShoreBody, stages, base, count),
            record!(SurfRecord, controls, bounds, bodies),
        ],
    );
    let cache = map_gpu::declarations_at(0)
        + COMPONENT
        + &SurfGpu::source(0, 7)
        + include_str!("cache.wgsl");
    validate(&cache);
    let query = map_gpu::declarations_at(1)
        + COMPONENT
        + &SurfGpu::source(1, 7)
        + include_str!("../column.wgsl")
        + &crate::interaction::Interaction::sample_source(0, 5, 6, true)
        + include_str!("../query/sample.wgsl")
        + include_str!("cut.wgsl");
    validate(&query);
}

fn validate(source: &str) {
    use wgpu::naga::{
        front::wgsl,
        valid::{Capabilities, ValidationFlags, Validator},
    };
    let module =
        wgsl::parse_str(source).unwrap_or_else(|error| panic!("{}", error.emit_to_string(source)));
    Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(source)));
}
