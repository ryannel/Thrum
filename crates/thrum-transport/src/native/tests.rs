use super::transport_reason;
use crate::CloseReason;
use web_transport_quinn::{SessionError, WebTransportError};

#[test]
fn idle_timeout_survives_http3_capsule_and_stream_error_wrappers() {
    let capsule = SessionError::WebTransportError(WebTransportError::ReadError(
        quinn::ReadExactError::ReadError(quinn::ReadError::ConnectionLost(
            quinn::ConnectionError::TimedOut,
        )),
    ));
    assert_eq!(transport_reason(&capsule), CloseReason::TimedOut);
    let stream = web_transport_quinn::ReadExactError::ReadError(
        web_transport_quinn::ReadError::SessionError(capsule),
    );
    assert_eq!(transport_reason(&stream), CloseReason::TimedOut);
    assert_eq!(
        transport_reason(&SessionError::ConnectionError(
            quinn::ConnectionError::TimedOut
        )),
        CloseReason::TimedOut
    );
    assert_eq!(
        transport_reason(&SessionError::WebTransportError(WebTransportError::Closed(
            42,
            String::new()
        ))),
        CloseReason::Peer(42)
    );
    assert_eq!(
        transport_reason(&SessionError::WebTransportError(
            WebTransportError::UnknownSession
        )),
        CloseReason::Transport
    );
}
