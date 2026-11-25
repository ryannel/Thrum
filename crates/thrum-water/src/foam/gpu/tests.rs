use super::{Foam, Settings};
use thrum_test_support::{record, records};

// A hull is a nested record, so the reflection compares its size and holds no scalar kind for it.
impl records::Kind for super::HullRecord {
    const KIND: Option<records::RecordScalar> = None;
}

#[test]
fn foam_settings_match_the_shader_layout() {
    let source = include_str!("../../field.wgsl").to_owned() + &Foam::source(true);
    records::named(
        "foam sampling shader",
        &source,
        &[record!(
            Settings as "FoamSettings",
            origins,
            banks,
            time_wind,
            camera,
            jacobian_variance,
            long_bounds,
            hulls,
            components,
        )],
    );
}
