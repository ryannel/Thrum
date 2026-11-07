use super::Recovery;

#[test]
fn lost_profiles_are_explicit_and_never_create_present_calls_or_timings() {
    assert_eq!(super::missing_profiles(3, 3, false).unwrap(), 0);
    assert!(super::missing_profiles(3, 2, false).is_err());
    assert_eq!(super::missing_profiles(3, 2, true).unwrap(), 1);
    assert_eq!(super::missing_profiles(3, 0, true).unwrap(), 3);
    assert!(super::missing_profiles(2, 3, true).is_err());
}

#[test]
fn loss_is_single_flight_busy_retains_queue_and_retry_is_explicit() {
    let mut recovery = Recovery::new(1);
    assert!(recovery.ready());
    assert!(!recovery.lost(0).unwrap());
    assert!(recovery.lost(1).unwrap());
    assert_eq!(recovery.queued(), Some(2));
    assert!(!recovery.lost(1).unwrap());
    // A full worker slot leaves the same bounded request available for retry.
    assert_eq!(recovery.queued(), Some(2));
    recovery.accepted(2);
    assert!(!recovery.ready());
    assert!(!recovery.complete(3, 3, true));
    assert!(recovery.complete(2, 1, false));
    assert!(recovery.failed());
    assert_eq!(recovery.queued(), None);
    assert!(
        !recovery.lost(1).unwrap(),
        "failure cannot spin automatic retries"
    );
    recovery.retry().unwrap();
    assert_eq!(
        recovery.queued(),
        Some(3),
        "failed candidate generation is never reused"
    );
    recovery.accepted(3);
    assert!(recovery.complete(3, 3, true));
    assert!(recovery.ready());
    assert_eq!(recovery.active, 3);
    assert!(!recovery.lost(1).unwrap());
    assert!(!recovery.lost(2).unwrap());
    assert!(recovery.ready());
}

#[test]
fn candidate_loss_before_reply_cannot_publish_ready_or_restart_old_generation() {
    let mut recovery = Recovery::new(7);
    recovery.lost(7).unwrap();
    recovery.accepted(8);
    assert!(!recovery.lost(8).unwrap());
    assert!(recovery.complete(8, 8, true));
    assert_eq!(recovery.active, 8);
    assert!(recovery.failed());
    assert!(!recovery.lost(7).unwrap());
    assert!(!recovery.lost(8).unwrap());
    recovery.retry().unwrap();
    recovery.accepted(9);
    recovery.complete(9, 9, true);
    assert!(recovery.ready());
    assert_eq!(recovery.active, 9);
}

#[test]
fn generation_exhaustion_keeps_the_last_device_identity() {
    let mut recovery = Recovery::new(u64::MAX);
    assert!(recovery.lost(u64::MAX).is_err());
    assert_eq!(recovery.active, u64::MAX);
    assert_eq!(recovery.queued(), None);
}
