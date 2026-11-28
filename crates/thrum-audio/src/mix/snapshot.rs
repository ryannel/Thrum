use std::{
    cell::UnsafeCell,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::{Listener, Volumes};

use super::EmitterVoice;

/// One reusable copy of the desired state. One-shots never enter this mailbox.
pub(crate) struct Snapshot {
    pub(crate) revision: u64,
    pub(crate) listener: Listener,
    pub(crate) volumes: Volumes,
    pub(crate) emitters: Box<[Option<EmitterVoice>]>,
}

impl Snapshot {
    fn new(emitters: usize) -> Self {
        Self {
            revision: 0,
            listener: Listener::default(),
            volumes: Volumes::default(),
            emitters: (0..emitters).map(|_| None).collect(),
        }
    }
}

const DIRTY: usize = 4;

struct Mailbox {
    slots: [UnsafeCell<Snapshot>; 3],
    middle: AtomicUsize,
}

#[expect(
    unsafe_code,
    reason = "the two endpoints exchange exclusive ownership of slots"
)]
// SAFETY: Each endpoint has exclusive access to its own slot. An AcqRel exchange with the
// unowned middle slot hands over ownership and publishes any writes to that slot.
unsafe impl Sync for Mailbox {}

pub(crate) struct Publisher {
    mailbox: Arc<Mailbox>,
    back: usize,
}

pub(crate) struct Subscriber {
    mailbox: Arc<Mailbox>,
    front: usize,
}

pub(crate) fn channel(emitters: usize) -> (Publisher, Subscriber) {
    let mailbox = Arc::new(Mailbox {
        slots: std::array::from_fn(|_| UnsafeCell::new(Snapshot::new(emitters))),
        middle: AtomicUsize::new(1),
    });
    (
        Publisher {
            mailbox: Arc::clone(&mailbox),
            back: 0,
        },
        Subscriber { mailbox, front: 2 },
    )
}

impl Publisher {
    #[expect(
        unsafe_code,
        reason = "only the publisher owns this slot until it publishes"
    )]
    pub(crate) fn back_mut(&mut self) -> &mut Snapshot {
        // SAFETY: Only this endpoint owns back, and the borrow ends before publish can run.
        unsafe { &mut *self.mailbox.slots[self.back].get() }
    }

    pub(crate) fn publish(&mut self) {
        self.back = self
            .mailbox
            .middle
            .swap(self.back | DIRTY, Ordering::AcqRel)
            & !DIRTY;
        // An unread snapshot reused by the publisher can release its sample leases here.
        self.back_mut().emitters.fill_with(|| None);
    }
}

impl Subscriber {
    #[expect(unsafe_code, reason = "only the subscriber owns the acquired snapshot")]
    pub(crate) fn latest(&mut self) -> Option<&mut Snapshot> {
        if self.mailbox.middle.load(Ordering::Acquire) & DIRTY == 0 {
            return None;
        }
        self.front = self.mailbox.middle.swap(self.front, Ordering::AcqRel) & !DIRTY;
        // SAFETY: After the exchange only the subscriber owns front. A new publish uses one
        // of the two other slots. The borrow ends before another call can exchange this slot.
        Some(unsafe { &mut *self.mailbox.slots[self.front].get() })
    }
}
