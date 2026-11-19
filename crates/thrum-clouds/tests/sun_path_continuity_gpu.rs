//! Sun-elevation and field-border continuity of actual GPU products.
mod support;

#[test]
#[cfg(target_os = "macos")]
fn broken_sky_factor_has_no_floor_or_border_switch() {
    support::conformance(
        "broken_sky_factor_has_no_floor_or_border_switch",
        false,
        true,
    );
}
