//! Public error inventory and wrapped-source contract.

use std::error::Error;
use thrum_network::{
    envelope::EnvelopeError, future_input::FutureInputError, input_batch::InputCodecError,
    state_ack::AckError, state_delta::DeltaError,
};

fn assert_error<T: Error>() {}

#[test]
fn public_errors_implement_error() {
    assert_error::<EnvelopeError>();
    assert_error::<FutureInputError>();
    assert_error::<InputCodecError>();
    assert_error::<AckError>();
    assert_error::<DeltaError>();
}

#[test]
fn invalid_payload_retains_the_input_error() {
    let error = FutureInputError::InvalidPayload(thrum_core::sim::InputFrameError::InvalidAction);
    assert!(
        error
            .source()
            .expect("input source")
            .is::<thrum_core::sim::InputFrameError>()
    );
}
