//! One platform surface lease shared by the UIKit shell and the render owner.
//!
//! Only the shell installs or removes a surface, and only with `try_lock`, so
//! UIKit never waits for the render owner. The render owner locks the slot to
//! configure and for one bounded acquire → submit → present section. Every
//! invalidation advances the epoch first, which closes admission for requests
//! made under an older epoch before any surface is removed.

use std::sync::{
    Mutex, PoisonError, TryLockError,
    atomic::{AtomicU64, Ordering},
};

/// The surface held by the slot, with the configuration the owner applied.
pub(crate) struct Leased<S> {
    pub(crate) surface: S,
    /// Epoch and extent of the last configuration, if any.
    pub(crate) configured: Option<(u64, [u32; 2])>,
}

/// Result of the shell's nonblocking removal attempt.
pub(crate) enum Take<S> {
    /// The surface was removed; the caller drops it on its own thread.
    Taken(S),
    /// No surface was installed.
    Empty,
    /// The owner is inside a configure or acquire/present section. The epoch
    /// already rejects that section's drawable; retry after its reply.
    Busy,
}

pub(crate) struct SurfaceLease<S> {
    epoch: AtomicU64,
    slot: Mutex<Option<Leased<S>>>,
}

impl<S> SurfaceLease<S> {
    pub(crate) fn new() -> Self {
        Self {
            epoch: AtomicU64::new(1),
            slot: Mutex::new(None),
        }
    }

    /// The epoch a new request is made under.
    pub(crate) fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::Acquire)
    }

    /// Close admission for every request made under an older epoch.
    pub(crate) fn invalidate(&self) -> u64 {
        self.epoch.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Shell only: install a surface under a new epoch without waiting.
    /// # Errors
    /// Returns the surface if the owner holds the slot or one is installed.
    pub(crate) fn try_install(&self, surface: S) -> Result<u64, S> {
        let mut slot = match self.slot.try_lock() {
            Ok(slot) => slot,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => return Err(surface),
        };
        if slot.is_some() {
            return Err(surface);
        }
        let epoch = self.invalidate();
        *slot = Some(Leased {
            surface,
            configured: None,
        });
        Ok(epoch)
    }

    /// Shell only: remove the surface without waiting. Call `invalidate` first.
    pub(crate) fn try_take(&self) -> Take<S> {
        let mut slot = match self.slot.try_lock() {
            Ok(slot) => slot,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => return Take::Busy,
        };
        slot.take()
            .map_or(Take::Empty, |leased| Take::Taken(leased.surface))
    }

    /// Owner only: run one section on the installed surface if `epoch` is
    /// still current. `None` means no lease or an old epoch.
    pub(crate) fn section<R>(
        &self,
        epoch: u64,
        run: impl FnOnce(&mut Leased<S>) -> R,
    ) -> Option<R> {
        let mut slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        if self.epoch() != epoch {
            return None;
        }
        slot.as_mut().map(run)
    }

    /// Owner only, inside a section: whether `epoch` may still present.
    pub(crate) fn is_current(&self, epoch: u64) -> bool {
        self.epoch() == epoch
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier, mpsc};

    use super::{SurfaceLease, Take};

    #[test]
    fn install_take_and_reinstall_advance_the_epoch() {
        let lease = SurfaceLease::new();
        let Ok(first) = lease.try_install("a") else {
            panic!("empty slot");
        };
        assert_eq!(lease.epoch(), first);
        assert!(
            matches!(lease.try_install("b"), Err("b")),
            "one lease at a time"
        );
        let revoked = lease.invalidate();
        assert!(revoked > first);
        assert!(matches!(lease.try_take(), Take::Taken("a")));
        assert!(matches!(lease.try_take(), Take::Empty));
        let Ok(second) = lease.try_install("b") else {
            panic!("released slot");
        };
        assert!(second > revoked);
    }

    #[test]
    fn an_old_epoch_cannot_enter_a_section() {
        let lease = SurfaceLease::new();
        let Ok(epoch) = lease.try_install(7_u32) else {
            panic!("empty slot");
        };
        assert_eq!(lease.section(epoch, |leased| leased.surface), Some(7));
        assert_eq!(
            lease.section(epoch, |leased| leased.configured.replace((epoch, [1, 1]))),
            Some(None),
            "a new lease starts unconfigured"
        );
        lease.invalidate();
        assert_eq!(lease.section(epoch, |leased| leased.surface), None);
        assert_eq!(
            lease.section(lease.epoch(), |leased| leased.surface),
            Some(7)
        );
    }

    #[test]
    fn the_shell_never_waits_for_a_held_section_and_the_section_sees_revocation() {
        let lease = Arc::new(SurfaceLease::new());
        let Ok(epoch) = lease.try_install(1_u32) else {
            panic!("empty slot");
        };
        let inside = Arc::new(Barrier::new(2));
        let (revoked_tx, revoked_rx) = mpsc::channel();
        let owner = {
            let lease = Arc::clone(&lease);
            let inside = Arc::clone(&inside);
            std::thread::spawn(move || {
                lease.section(epoch, |_| {
                    inside.wait();
                    revoked_rx.recv().expect("shell revokes");
                    lease.is_current(epoch)
                })
            })
        };
        inside.wait();
        lease.invalidate();
        assert!(matches!(lease.try_take(), Take::Busy), "busy, not blocked");
        revoked_tx.send(()).expect("owner waits");
        assert_eq!(
            owner.join().expect("owner"),
            Some(false),
            "old epoch cannot present"
        );
        assert!(
            matches!(lease.try_take(), Take::Taken(1)),
            "deferred release"
        );
    }
}
