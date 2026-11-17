//! Public error inventory and wrapped-source contract.

fn assert_error<T: std::error::Error>() {}

#[test]
fn public_errors_implement_error() {
    assert_error::<thrum_controls::ControlError>();
    #[cfg(target_os = "macos")]
    assert_error::<thrum_controls::macos::PanelError>();
}
