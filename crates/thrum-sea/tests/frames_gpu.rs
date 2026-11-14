//! Frame lifecycle and continuous wind transition on Metal.
mod support;

#[test]
fn frame_script_and_continuous_wind_change() {
    support::ambient(false, "frames-ambient");
}
