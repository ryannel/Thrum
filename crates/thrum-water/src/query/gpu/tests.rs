use super::{Answer, CascadeSample, Component, Interaction, Point, Settings};
use thrum_test_support::{record, records};

// A wave component is a nested record in the foam and query settings, so the reflection compares its size and
// holds no scalar kind for it. Its own members are checked below.
impl records::Kind for Component {
    const KIND: Option<records::RecordScalar> = None;
}
// So is a cascade sample in the answer.
impl records::Kind for CascadeSample {
    const KIND: Option<records::RecordScalar> = None;
}

#[test]
fn query_records_match_the_shader_layout() {
    let source = Interaction::sample_source(0, 5, 6, true)
        + include_str!("../../gains.wgsl")
        + include_str!("../../column.wgsl")
        + include_str!("../sample.wgsl");
    records::named(
        "water query shader",
        &source,
        &[
            record!(
                Point as "QueryPoint",
                position,
                radius,
                dt,
                level_current_bed,
                slope,
                body,
                has_bed,
                gains,
                dent,
            ),
            record!(
                Component, vector, amplitude, phase, omega, inverse_k, padding
            ),
            record!(
                Settings,
                origin,
                count,
                components,
                lengths_chop,
                heights,
                swash_margin,
            ),
            record!(CascadeSample, height, slope, differential, velocity),
            record!(
                Answer,
                height_slope_rate,
                velocity_depth,
                layers,
                current_bed,
                body_flags,
                cascades,
                padding,
                wake,
            ),
        ],
    );
}
