//! Process-shared reservation for immutable publication allocations.
//!
//! Reservation and ownership charges are distinct: a reservation is made
//! before a world step; committed allocations keep their charge until the
//! final owner releases them. The pool does not infer game-owned heap usage.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct PoolState {
    limit: usize,
    used: AtomicUsize,
}

/// A process-shared logical byte budget for published immutable data.
#[derive(Clone)]
pub struct PublicationPool {
    state: Arc<PoolState>,
}

/// An admission failure before a publication allocation is made.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicationCapacity;

/// Capacity held for one proposed publication. Unused capacity returns to the
/// pool when this value drops.
#[must_use = "keep the reservation until publication allocations are charged or cancelled"]
pub struct PublicationReservation {
    state: Arc<PoolState>,
    remaining: usize,
}

/// Capacity attached to one unique allocation. Sharing its owner must share
/// this charge rather than creating a second one.
#[must_use = "attach the charge to the allocation owner until its final release"]
pub struct PublicationCharge {
    state: Arc<PoolState>,
    bytes: usize,
}

impl PublicationPool {
    /// Construct a logical budget. This is not an OS resident-byte limit.
    pub fn new(limit: usize) -> Self {
        Self {
            state: Arc::new(PoolState {
                limit,
                used: AtomicUsize::new(0),
            }),
        }
    }

    /// Total admitted capacity, including unspent reservations.
    pub fn used_bytes(&self) -> usize {
        self.state.used.load(Ordering::Acquire)
    }

    /// Configured logical limit.
    pub fn limit_bytes(&self) -> usize {
        self.state.limit
    }

    /// Atomically reserve peak space before a world mutation. On failure, no
    /// capacity changes hands. A zero-byte reservation is valid.
    pub fn try_reserve(&self, bytes: usize) -> Result<PublicationReservation, PublicationCapacity> {
        let mut used = self.state.used.load(Ordering::Acquire);
        loop {
            let next = used.checked_add(bytes).ok_or(PublicationCapacity)?;
            if next > self.state.limit {
                return Err(PublicationCapacity);
            }
            match self.state.used.compare_exchange_weak(
                used,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Ok(PublicationReservation {
                        state: self.state.clone(),
                        remaining: bytes,
                    });
                }
                Err(current) => used = current,
            }
        }
    }
}

impl PublicationReservation {
    /// Capacity still available for unique allocations in this publication.
    pub fn remaining_bytes(&self) -> usize {
        self.remaining
    }

    /// Transfer part of this reservation to one allocation owner. The sum of
    /// unspent reservations and live charges remains unchanged here.
    pub fn charge(&mut self, bytes: usize) -> Result<PublicationCharge, PublicationCapacity> {
        self.remaining = self
            .remaining
            .checked_sub(bytes)
            .ok_or(PublicationCapacity)?;
        Ok(PublicationCharge {
            state: self.state.clone(),
            bytes,
        })
    }
}

impl Drop for PublicationReservation {
    fn drop(&mut self) {
        self.state.used.fetch_sub(self.remaining, Ordering::Release);
    }
}

impl Drop for PublicationCharge {
    fn drop(&mut self) {
        self.state.used.fetch_sub(self.bytes, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Barrier},
        thread,
    };

    use super::*;

    #[test]
    fn reservations_and_unique_owners_release_exactly_once() {
        let pool = PublicationPool::new(100);
        let mut first = pool.try_reserve(80).unwrap();
        assert_eq!(pool.used_bytes(), 80);
        let charge = Arc::new(first.charge(30).unwrap());
        assert_eq!(first.remaining_bytes(), 50);
        assert_eq!(pool.try_reserve(21).err(), Some(PublicationCapacity));
        drop(first);
        assert_eq!(pool.used_bytes(), 30);
        let second = pool.try_reserve(70).unwrap();
        let held = charge.clone();
        drop(charge);
        assert_eq!(pool.used_bytes(), 100);
        drop(second);
        assert_eq!(pool.used_bytes(), 30);
        drop(held);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn concurrent_admission_never_exceeds_the_shared_limit() {
        let pool = PublicationPool::new(10);
        let held = pool.try_reserve(7).unwrap();
        let other = pool.clone();
        let rejected = thread::spawn(move || other.try_reserve(4).err());
        assert_eq!(rejected.join().unwrap(), Some(PublicationCapacity));
        assert_eq!(pool.used_bytes(), 7);
        drop(held);
        assert!(pool.try_reserve(10).is_ok());
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn competing_reservations_admit_only_one_owner() {
        let pool = PublicationPool::new(10);
        let start = Arc::new(Barrier::new(3));
        let attempted = Arc::new(Barrier::new(3));
        let release = Arc::new(Barrier::new(3));
        let mut workers = Vec::new();
        for _ in 0..2 {
            let pool = pool.clone();
            let start = start.clone();
            let attempted = attempted.clone();
            let release = release.clone();
            workers.push(thread::spawn(move || {
                start.wait();
                let reservation = pool.try_reserve(7);
                attempted.wait();
                release.wait();
                reservation.is_ok()
            }));
        }
        start.wait();
        attempted.wait();
        let admitted_bytes = pool.used_bytes();
        release.wait();
        assert_eq!(admitted_bytes, 7);
        assert_eq!(
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .filter(|admitted| *admitted)
                .count(),
            1
        );
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn overflow_zero_and_failed_transfer_preserve_capacity() {
        let pool = PublicationPool::new(usize::MAX);
        let held = pool.try_reserve(usize::MAX - 1).unwrap();
        assert_eq!(pool.try_reserve(2).err(), Some(PublicationCapacity));
        drop(held);
        let mut zero = pool.try_reserve(0).unwrap();
        let zero_charge = zero.charge(0).unwrap();
        assert_eq!(pool.used_bytes(), 0);
        drop(zero_charge);
        drop(zero);
        let mut reservation = pool.try_reserve(5).unwrap();
        assert_eq!(reservation.charge(6).err(), Some(PublicationCapacity));
        assert_eq!(reservation.remaining_bytes(), 5);
        let charge = reservation.charge(5).unwrap();
        drop(reservation);
        assert_eq!(pool.used_bytes(), 5);
        drop(charge);
        assert_eq!(pool.used_bytes(), 0);
    }
}
