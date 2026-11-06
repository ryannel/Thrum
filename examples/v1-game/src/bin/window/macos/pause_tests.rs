use super::{PauseIntent, WorldLifecycle};

#[test]
fn only_nonrepeated_press_changes_intent() {
    let mut intent = PauseIntent::default();
    assert!(!intent.needs_reply());
    assert_eq!(intent.next_request(), None);
    assert!(!intent.key(false, false));
    assert!(!intent.key(true, true));
    assert_eq!(intent.next_request(), None);
    assert!(intent.key(true, false));
    assert_eq!(intent.next_request(), Some(WorldLifecycle::Pause));
    assert_eq!(intent.label(), "Pausing (P)");
    assert!(!intent.key(true, true));
    assert!(!intent.key(false, false));
    assert_eq!(intent.next_request(), Some(WorldLifecycle::Pause));
}

#[test]
fn pending_pause_preserves_a_later_resume_and_busy_preserves_intent() {
    let mut intent = PauseIntent::default();
    intent.key(true, false);
    // Busy does not acknowledge or accept the request; the intent remains due.
    assert_eq!(intent.next_request(), Some(WorldLifecycle::Pause));
    assert_eq!(intent.next_request(), Some(WorldLifecycle::Pause));
    intent.accepted(WorldLifecycle::Pause);
    assert!(intent.needs_reply());
    assert_eq!(intent.next_request(), None);
    intent.key(true, false);
    assert_eq!(intent.next_request(), None);
    intent.acknowledge(WorldLifecycle::Pause).unwrap();
    assert!(!intent.needs_reply());
    assert_eq!(intent.next_request(), Some(WorldLifecycle::Resume));
    intent.accepted(WorldLifecycle::Resume);
    assert_eq!(intent.label(), "Resuming (P)");
    intent.acknowledge(WorldLifecycle::Resume).unwrap();
    assert_eq!(intent.next_request(), None);
    assert_eq!(intent.label(), "Running (P)");
}

#[test]
fn rapid_toggles_and_invalid_ack_leave_the_accepted_request_intact() {
    let mut intent = PauseIntent::default();
    intent.key(true, false);
    intent.accepted(WorldLifecycle::Pause);
    for _ in 0..2 {
        intent.key(true, false);
    }
    assert!(intent.acknowledge(WorldLifecycle::Resume).is_err());
    assert!(intent.needs_reply());
    assert_eq!(intent.next_request(), None);
    intent.acknowledge(WorldLifecycle::Pause).unwrap();
    assert_eq!(intent.label(), "Paused (P)");
    assert_eq!(intent.next_request(), None);
    assert!(intent.acknowledge(WorldLifecycle::Pause).is_err());
    assert_eq!(intent.label(), "Paused (P)");
}
