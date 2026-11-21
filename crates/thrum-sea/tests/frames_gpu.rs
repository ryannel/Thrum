//! Frame lifecycle and continuous wind transition on Metal.
#[expect(
    dead_code,
    reason = "shared harness includes other independent sea oracles"
)]
mod support;

#[test]
fn frame_script_and_continuous_wind_change() {
    support::frame_script(false);
}
