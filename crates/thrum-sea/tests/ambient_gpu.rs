//! Metal ambient field conformance.
mod support;
#[test]
fn metal_ambient_matches_independent_model() {
    support::ambient(false, "ambient");
}

#[test]
#[ignore = "standalone cost observer; run explicitly under the GPU lock after numerical qualification"]
fn isolated_pass_cost() {
    let mut h = support::Harness::new(false);
    h.isolated_cost(&support::spectrum(20.0), &support::points());
    h.save("cost-only");
}
