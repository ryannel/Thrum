//! Public error inventory and wrapped-source contract.

use std::error::Error;
use thrum_sea::AmbientError;

fn assert_error<T: Error>() {}

#[test]
fn public_errors_implement_error() {
    assert_error::<AmbientError>();
}

#[test]
fn model_refusal_is_the_source() {
    let error = AmbientError::LongWaves(thrum_sea_model::SeaError::InvalidInput);
    assert!(error.source().expect("model source").is::<thrum_sea_model::SeaError>());
}
