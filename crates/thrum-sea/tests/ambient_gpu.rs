//! Metal ambient field conformance.
mod support;
#[test]
fn metal_ambient_matches_independent_model() {
    support::ambient(false, "ambient");
}
