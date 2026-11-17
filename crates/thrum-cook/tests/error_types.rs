//! Public error inventory and wrapped-source contract.

use std::error::Error;
use thrum_cook::{CookError, OriginError, TextureEncodeError, TripwireError};

fn assert_error<T: Error>() {}

#[test]
fn public_errors_implement_error() {
    assert_error::<CookError>();
    assert_error::<OriginError>();
    assert_error::<TripwireError>();
    assert_error::<TextureEncodeError>();
}

#[test]
fn wrapped_cooker_errors_preserve_sources() {
    let error = CookError::Io(std::io::Error::from(std::io::ErrorKind::NotFound));
    assert!(error.source().expect("I/O source").is::<std::io::Error>());
    let error = CookError::Product(thrum_asset::AssetError::HashMismatch);
    assert!(
        error
            .source()
            .expect("product source")
            .is::<thrum_asset::AssetError>()
    );
    let error = TripwireError::History {
        path: "history.jsonl".into(),
        report: "passing run".into(),
        source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
    };
    assert!(
        error
            .source()
            .expect("history source")
            .is::<std::io::Error>()
    );
    let error = CookError::Watch(notify::Error::generic("watch unavailable"));
    assert!(error.source().expect("watch source").is::<notify::Error>());
}
