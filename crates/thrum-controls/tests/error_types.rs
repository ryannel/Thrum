//! Public error inventory and wrapped-source contract.

fn assert_error<T: std::error::Error>() {}

#[test]
fn public_errors_implement_error() {
    assert_error::<thrum_controls::ControlError>();
    #[cfg(target_os = "macos")]
    assert_error::<thrum_controls::macos::PanelError>();
}

#[test]
fn malformed_file_keeps_the_original_parser_error_and_clone_identity() {
    use std::error::Error;
    use thrum_controls::{ControlError, Controls};

    let mut controls = Controls::new();
    let revision = controls.revision();
    for input in ["{", "[1]", "null"] {
        let error = controls
            .apply_file(input)
            .expect_err("invalid control file");
        assert!(matches!(error, ControlError::InvalidFile(_)));
        let source = error.source().expect("JSON parser source");
        let parser = source
            .downcast_ref::<serde_json::Error>()
            .expect("original JSON error");
        assert!(parser.is_eof() || parser.is_data());
        assert_eq!(controls.revision(), revision);
        let cloned = error.clone();
        assert_eq!(cloned, error);
        assert!(std::ptr::eq(
            source,
            cloned.source().expect("cloned JSON source")
        ));
    }
}
