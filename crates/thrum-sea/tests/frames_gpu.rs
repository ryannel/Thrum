//! Frame lifecycle and continuous wind transition on Metal.
#[expect(
    dead_code,
    reason = "the shared cost observer is exercised separately by ambient_gpu"
)]
mod support;

#[test]
fn frame_script_and_continuous_wind_change() {
    support::ambient(false, "frames-ambient");
}
