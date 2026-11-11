use super::{Completion, PassNames, PassTiming, SceneGpuTimings, TimingError};
use std::sync::Arc;

#[test]
fn completion_reports_failed_and_dropped_callbacks_and_reuses_the_owner() {
    let completion = Completion::new();
    let mut first = completion.arm();
    assert_eq!(completion.ready(), Ok(false));
    first.complete(false);
    assert_eq!(completion.ready(), Err(TimingError::MapFailed));
    let second = completion.arm();
    drop(first);
    assert_eq!(completion.ready(), Ok(false));
    drop(second);
    assert_eq!(completion.ready(), Err(TimingError::MapFailed));
    let mut third = completion.arm();
    third.complete(true);
    assert_eq!(completion.ready(), Ok(true));
}

#[test]
fn distinct_timing_owners_complete_independently() {
    let first = Completion::new();
    let second = Completion::new();
    let mut first_ticket = first.arm();
    let mut second_ticket = second.arm();
    second_ticket.complete(true);
    assert_eq!(first.ready(), Ok(false));
    assert_eq!(second.ready(), Ok(true));
    first_ticket.complete(false);
    assert_eq!(first.ready(), Err(TimingError::MapFailed));
    assert_eq!(second.ready(), Ok(true));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn completed_callback_drop_cannot_poison_concurrent_reuse() {
    use std::sync::Barrier;

    let completion = Completion::new();
    let mut previous = completion.arm();
    let boundary = Arc::new(Barrier::new(2));
    let callback_boundary = Arc::clone(&boundary);
    let callback = std::thread::spawn(move || {
        previous.complete(true);
        callback_boundary.wait();
        callback_boundary.wait();
        drop(previous);
    });
    boundary.wait();
    assert_eq!(completion.ready(), Ok(true));
    let mut current = completion.arm();
    boundary.wait();
    callback.join().expect("previous callback completed");
    assert_eq!(completion.ready(), Ok(false));
    current.complete(true);
    assert_eq!(completion.ready(), Ok(true));
}

#[test]
fn pass_names_reuse_slots_across_batches_and_replace_changed_roles() {
    let mut names = PassNames::with_capacity(2);
    assert_eq!(names.push("depth"), 0);
    assert_eq!(names.push("forward"), 1);
    let first = Arc::clone(&names.slots[0]);
    let second = Arc::clone(&names.slots[1]);
    names.clear();
    assert_eq!(names.push("depth"), 0);
    assert_eq!(names.push("forward"), 1);
    assert!(Arc::ptr_eq(&first, &names.slots[0]));
    assert!(Arc::ptr_eq(&second, &names.slots[1]));
    names.clear();
    assert_eq!(names.push("shadow"), 0);
    assert!(!Arc::ptr_eq(&first, &names.slots[0]));
    assert_eq!(names.slots[1].as_ref(), "forward");
}

#[test]
fn named_scene_timings_preserve_optional_pass_identity_and_unavailable_transfers() {
    let passes = [
        PassTiming {
            name: "synthetic forward opaque".into(),
            nanoseconds: 7.0,
        },
        PassTiming {
            name: "atmosphere stars".into(),
            nanoseconds: 3.0,
        },
        PassTiming {
            name: "synthetic tone encode".into(),
            nanoseconds: 0.0,
        },
        PassTiming {
            name: crate::present::PASS_NAME.into(),
            nanoseconds: 2.0,
        },
    ];
    let mut named = SceneGpuTimings::new(&passes);
    named.unmeasured(["contribution upload".to_owned()].into_iter());
    assert_eq!(named.get("synthetic tone encode"), Some(0.0));
    assert_eq!(named.get("synthetic cluster build"), None);
    assert_eq!(named.forward(), 10.0);
    assert_eq!(named.atmosphere(), 0.0, "stars belong to opaque work once");
    assert_eq!(named.sum(), 10.0, "surface stays separate");
    assert_eq!(named.values()["contribution upload"], None);
    assert_eq!(named.values()["timestamp query resolve/readback"], None);
    let mut reordered = passes.to_vec();
    reordered.reverse();
    assert_eq!(SceneGpuTimings::new(&reordered).forward(), named.forward());
    assert_eq!(SceneGpuTimings::new(&[]).get("atmosphere stars"), None);
}
