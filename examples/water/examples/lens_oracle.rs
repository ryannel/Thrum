//! Independent model expectations for the browser's fixed lens fixture.
use serde_json::json;
use water_scene::lens_probe::{BROWSER_DATUM, Case, EXTENT, NAMES, STEP};

#[expect(
    clippy::print_stdout,
    reason = "the browser proof reads the oracle over stdout"
)]
fn main() {
    let cases: serde_json::Map<_, _> = NAMES
        .into_iter()
        .chain([
            "carried-crest",
            "carried-gain",
            "carried-weight",
            "trough-wet",
        ])
        .map(|name| {
            (
                name.to_owned(),
                json!(Case::new(name, BROWSER_DATUM).browser_expected()),
            )
        })
        .collect();
    println!(
        "{}",
        json!({ "extent": EXTENT, "cases": cases, "step_ns": STEP.as_nanos() })
    );
}
