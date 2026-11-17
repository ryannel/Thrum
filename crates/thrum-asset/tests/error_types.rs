//! Public error inventory and wrapped-source contract.

use std::error::Error;
use thrum_asset::AssetError;

fn assert_error<T: Error>() {}

#[test]
fn public_errors_implement_error() {
    assert_error::<AssetError>();
}

#[test]
fn io_refusal_retains_its_source() {
    let error = AssetError::Io(std::io::Error::from(std::io::ErrorKind::NotFound));
    assert!(error.source().expect("I/O source").is::<std::io::Error>());
}
