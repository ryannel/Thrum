//! Exact server result frontier survives frame/journal retirement across gaps.

use thrum_network::retirement::{AuthorityRetirement, PeerKey, RetirementError};

fn key() -> PeerKey {
    PeerKey {
        match_id: *b"0123456789abcdef",
        peer_id: 11,
        connection_epoch: 42,
    }
}

#[test]
fn admitted_higher_identity_survives_independent_journal_reuse() {
    let mut retirement = AuthorityRetirement::new(key(), 6).unwrap();
    let reserved = retirement.reserved_capacity_bytes();
    let mut calls = Vec::new();
    retirement.stage(key(), &[8]).unwrap();
    assert_eq!(
        retirement.stage(key(), &[9]),
        Err(RetirementError::ReceiptOccupied)
    );
    assert_eq!(
        retirement.finish(key(), |identity, through| {
            assert_eq!(identity, key());
            calls.push(through);
            Ok::<_, ()>(())
        }),
        Ok(6)
    );
    assert!(calls.is_empty());
    assert_eq!(retirement.admitted_higher_count(), 1);

    // The physical sent journal can recycle a slot once result 8's frame is
    // admitted. The frontier owns the independent exact identity thereafter.
    let mut recycled_journal_slot = Some(8);
    assert_eq!(recycled_journal_slot.take(), Some(8));
    recycled_journal_slot = Some(99);
    assert_eq!(recycled_journal_slot, Some(99));

    retirement.stage(key(), &[9]).unwrap();
    assert_eq!(
        retirement.finish(key(), |_, through| {
            calls.push(through);
            Ok::<_, ()>(())
        }),
        Ok(6)
    );
    retirement.stage(key(), &[7]).unwrap();
    assert_eq!(
        retirement.finish(key(), |_, through| {
            calls.push(through);
            Ok::<_, ()>(())
        }),
        Ok(9)
    );
    assert_eq!(calls, [9]);
    assert_eq!(retirement.admitted_higher_count(), 0);
    assert_eq!(retirement.reserved_capacity_bytes(), reserved);
    retirement.stage(key(), &[8]).unwrap();
    let duplicate: Result<u64, RetirementError<()>> =
        retirement.finish(key(), |_, _| panic!("already retired"));
    assert_eq!(duplicate, Ok(9));
}

#[test]
fn authority_refusal_keeps_exact_receipt_for_retry() {
    let mut retirement = AuthorityRetirement::new(key(), 6).unwrap();
    retirement.stage(key(), &[7]).unwrap();
    assert_eq!(
        retirement.finish(key(), |_, _| Err("authority busy")),
        Err(RetirementError::Authority("authority busy"))
    );
    assert_eq!(retirement.retired_through(), 6);
    assert!(retirement.receipt_occupied());
    assert_eq!(
        retirement.stage(key(), &[8]),
        Err(RetirementError::ReceiptOccupied)
    );
    assert_eq!(
        retirement.finish(key(), |identity, through| {
            assert_eq!((identity, through), (key(), 7));
            Ok::<_, &str>(())
        }),
        Ok(7)
    );
    assert!(!retirement.receipt_occupied());
}

#[test]
fn unwound_authority_ack_poisons_frontier_and_receipt() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let mut retirement = AuthorityRetirement::new(key(), 6).unwrap();
    retirement.stage(key(), &[7]).unwrap();
    let caught = catch_unwind(AssertUnwindSafe(|| {
        let _: Result<u64, RetirementError<()>> =
            retirement.finish(key(), |_, _| panic!("uncertain authority ACK"));
    }));
    assert!(caught.is_err());
    assert!(retirement.receipt_occupied());
    assert_eq!(
        retirement.stage(key(), &[8]),
        Err(RetirementError::Poisoned)
    );
    assert_eq!(
        retirement.finish(key(), |_, _| Ok::<_, ()>(())),
        Err(RetirementError::Poisoned)
    );
}

#[test]
fn identity_and_finite_window_refuse_without_overwriting_receipt() {
    let mut retirement = AuthorityRetirement::new(key(), 6).unwrap();
    let wrong = PeerKey {
        connection_epoch: 43,
        ..key()
    };
    assert_eq!(
        retirement.stage(wrong, &[7]),
        Err(RetirementError::WrongIdentity)
    );
    for wrong in [
        PeerKey {
            match_id: *b"fedcba9876543210",
            ..key()
        },
        PeerKey {
            peer_id: 12,
            ..key()
        },
    ] {
        assert_eq!(
            retirement.stage(wrong, &[7]),
            Err(RetirementError::WrongIdentity)
        );
    }
    assert_eq!(
        retirement.stage(key(), &[0]),
        Err(RetirementError::InvalidIdentity)
    );
    assert_eq!(
        retirement.stage(key(), &[263]),
        Err(RetirementError::InvalidIdentity)
    );
    assert_eq!(
        retirement.stage(key(), &[7; 257]),
        Err(RetirementError::Capacity)
    );
    assert_eq!(retirement.retired_through(), 6);
    assert!(!retirement.receipt_occupied());
    assert_eq!(
        retirement.finish(key(), |_, _| Ok::<_, ()>(())),
        Err(RetirementError::NoReceipt)
    );

    retirement.stage(key(), &[7]).unwrap();
    assert_eq!(
        retirement.finish(wrong, |_, _| Ok::<_, ()>(())),
        Err(RetirementError::WrongIdentity)
    );
    assert!(retirement.receipt_occupied());
}

#[test]
fn callback_must_recheck_current_authority_incarnation() {
    let mut retirement = AuthorityRetirement::new(key(), 6).unwrap();
    retirement.stage(key(), &[7]).unwrap();
    let current_authority = PeerKey {
        connection_epoch: 43,
        ..key()
    };
    assert_eq!(
        retirement.finish(key(), |expected, _| {
            if expected != current_authority {
                Err("stale authority epoch")
            } else {
                Ok(())
            }
        }),
        Err(RetirementError::Authority("stale authority epoch"))
    );
    assert!(retirement.receipt_occupied());
    assert_eq!(retirement.retired_through(), 6);
}

#[test]
fn sequence_boundary_does_not_wrap() {
    let mut retirement = AuthorityRetirement::new(key(), u64::MAX - 1).unwrap();
    retirement.stage(key(), &[u64::MAX]).unwrap();
    assert_eq!(
        retirement.finish(key(), |_, through| {
            assert_eq!(through, u64::MAX);
            Ok::<_, ()>(())
        }),
        Ok(u64::MAX)
    );
    retirement.stage(key(), &[u64::MAX]).unwrap();
    let duplicate: Result<u64, RetirementError<()>> =
        retirement.finish(key(), |_, _| panic!("no new prefix"));
    assert_eq!(duplicate, Ok(u64::MAX));
}
