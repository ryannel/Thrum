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

struct ChargedBytes {
    bytes: Box<[u8]>,
    _charge: PublicationCharge,
}

/// Immutable payload bytes with one pool charge for every shared owner.
/// A clone shares the bytes and charge without allocating or copying them.
#[derive(Clone)]
pub struct PublicationBytes(Arc<ChargedBytes>);

/// A closed, fixed-layout value for a future admitted game profile. Inline
/// bytes have no hidden Rust owner; each optional immutable payload already
/// carries its own pool charge. Profile size limits and authority validation
/// are separate from this storage primitive.
#[derive(Debug, Eq, PartialEq)]
pub struct PublicationRecord<const INLINE: usize, const BLOBS: usize> {
    inline: [u8; INLINE],
    blobs: [Option<PublicationBytes>; BLOBS],
}

mod sealed {
    pub trait ClosedValue {}
}

/// Engine-owned state that can be checked before charged authority admission.
/// This boundary is sealed so a game type cannot claim an unchecked owner is
/// charged. It does not by itself establish a finite whole-tick profile.
pub trait ClosedPublicationValue: Clone + sealed::ClosedValue {
    /// Whether every retained payload owner belongs to this process pool.
    fn belongs_to_pool(&self, pool: &PublicationPool) -> bool;
}

/// A record attempted to retain a payload from another publication pool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationRecordError {
    /// Every owned payload in one record must belong to the selected pool.
    WrongPool,
}

impl<const INLINE: usize, const BLOBS: usize> PublicationRecord<INLINE, BLOBS> {
    /// Build a closed value, checking the provenance of every payload owner.
    pub fn try_new(
        pool: &PublicationPool,
        inline: [u8; INLINE],
        blobs: [Option<PublicationBytes>; BLOBS],
    ) -> Result<Self, PublicationRecordError> {
        if blobs
            .iter()
            .flatten()
            .any(|payload| !payload.belongs_to(pool))
        {
            return Err(PublicationRecordError::WrongPool);
        }
        Ok(Self { inline, blobs })
    }

    /// Fixed inline bytes; game-facing typed decoding does not change storage.
    pub fn inline(&self) -> &[u8; INLINE] {
        &self.inline
    }

    /// Borrow one immutable payload owner for sharing into a new record.
    pub fn payload_owner(&self, index: usize) -> Option<&PublicationBytes> {
        self.blobs.get(index)?.as_ref()
    }

    /// Check provenance before admitting this record to another owner.
    pub fn belongs_to(&self, pool: &PublicationPool) -> bool {
        self.blobs
            .iter()
            .flatten()
            .all(|payload| payload.belongs_to(pool))
    }
}

impl<const INLINE: usize, const BLOBS: usize> Clone for PublicationRecord<INLINE, BLOBS> {
    fn clone(&self) -> Self {
        Self {
            inline: self.inline,
            blobs: self.blobs.clone(),
        }
    }
}

impl<const INLINE: usize, const BLOBS: usize> sealed::ClosedValue
    for PublicationRecord<INLINE, BLOBS>
{
}

impl<const INLINE: usize, const BLOBS: usize> ClosedPublicationValue
    for PublicationRecord<INLINE, BLOBS>
{
    fn belongs_to_pool(&self, pool: &PublicationPool) -> bool {
        self.belongs_to(pool)
    }
}

impl PublicationBytes {
    fn belongs_to(&self, pool: &PublicationPool) -> bool {
        Arc::ptr_eq(&self.0._charge.state, &pool.state)
    }
    /// Reserve the logical payload and owner bytes before copying from a
    /// caller-owned source. The source and allocator metadata are not charged.
    pub fn try_copy_from_slice(
        pool: &PublicationPool,
        source: &[u8],
    ) -> Result<Self, PublicationCapacity> {
        let mut reservation = pool.try_reserve(Self::required_charge(source.len())?)?;
        Self::try_copy_from_reservation(&mut reservation, source)
    }

    /// Transfer an existing admission reservation to a retained byte owner.
    /// Insufficient reserved capacity rejects before copying the source.
    pub fn try_copy_from_reservation(
        reservation: &mut PublicationReservation,
        source: &[u8],
    ) -> Result<Self, PublicationCapacity> {
        let charge = reservation.charge(Self::required_charge(source.len())?)?;
        let payload = Box::<[u8]>::from(source);
        Ok(Self(Arc::new(ChargedBytes {
            bytes: payload,
            _charge: charge,
        })))
    }

    /// Logical bytes required to retain a payload of `len` bytes, including
    /// its shared owner. Callers can sum this before reserving a complete tick.
    pub fn required_charge(len: usize) -> Result<usize, PublicationCapacity> {
        len.checked_add(size_of::<ChargedBytes>())
            .and_then(|total| total.checked_add(2 * size_of::<usize>()))
            .ok_or(PublicationCapacity)
    }

    /// Borrow the retained immutable bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.0.bytes
    }
}

impl std::fmt::Debug for PublicationBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PublicationBytes")
            .field("len", &self.as_slice().len())
            .finish()
    }
}

impl PartialEq for PublicationBytes {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for PublicationBytes {}

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
        if bytes == 0 {
            return Ok(PublicationReservation {
                state: self.state.clone(),
                remaining: 0,
            });
        }
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
    pub(crate) fn pool(&self) -> PublicationPool {
        PublicationPool {
            state: self.state.clone(),
        }
    }

    pub(crate) fn belongs_to(&self, pool: &PublicationPool) -> bool {
        Arc::ptr_eq(&self.state, &pool.state)
    }

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
        if self.remaining != 0 {
            self.state.used.fetch_sub(self.remaining, Ordering::Release);
        }
    }
}

impl Drop for PublicationCharge {
    fn drop(&mut self) {
        if self.bytes != 0 {
            self.state.used.fetch_sub(self.bytes, Ordering::Release);
        }
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

        let full = PublicationPool::new(5);
        let occupied = full.try_reserve(5).unwrap();
        let empty = full.try_reserve(0).unwrap();
        assert_eq!(full.used_bytes(), 5);
        drop(empty);
        assert_eq!(full.used_bytes(), 5);
        drop(occupied);
        assert_eq!(full.used_bytes(), 0);
    }

    #[test]
    fn immutable_payload_rejects_before_copy_and_releases_after_final_clone() {
        let header = PublicationBytes::required_charge(0).unwrap();
        assert_eq!(PublicationBytes::required_charge(4), Ok(header + 4));
        assert_eq!(
            PublicationBytes::required_charge(usize::MAX),
            Err(PublicationCapacity)
        );
        let pool = PublicationPool::new(header + 4);
        assert!(matches!(
            PublicationBytes::try_copy_from_slice(&pool, b"12345"),
            Err(PublicationCapacity)
        ));
        assert_eq!(pool.used_bytes(), 0);
        let mut source = *b"1234";
        let first = PublicationBytes::try_copy_from_slice(&pool, &source).unwrap();
        source[0] = b'X';
        assert_eq!(&source, b"X234");
        let held = first.clone();
        assert_eq!(first.as_slice(), b"1234");
        assert_eq!(first, held);
        assert_eq!(pool.used_bytes(), header + 4);
        drop(first);
        assert_eq!(pool.used_bytes(), header + 4);
        assert!(matches!(
            PublicationBytes::try_copy_from_slice(&pool, b""),
            Err(PublicationCapacity)
        ));
        drop(held);
        assert_eq!(pool.used_bytes(), 0);
        let empty = PublicationBytes::try_copy_from_slice(&pool, b"").unwrap();
        assert_eq!(empty.as_slice(), b"");
        assert_eq!(pool.used_bytes(), header);
    }

    #[test]
    fn closed_record_rejects_foreign_payload_and_shares_charge_until_final_clone() {
        let first = PublicationPool::new(10_000);
        let foreign = PublicationPool::new(10_000);
        let payload = PublicationBytes::try_copy_from_slice(&first, &[7; 512]).unwrap();
        let bytes = first.used_bytes();
        assert_eq!(
            PublicationRecord::<8, 1>::try_new(
                &foreign,
                5_u64.to_le_bytes(),
                [Some(payload.clone())]
            ),
            Err(PublicationRecordError::WrongPool)
        );
        assert_eq!(first.used_bytes(), bytes);
        let record = PublicationRecord::<8, 1>::try_new(
            &first,
            5_u64.to_le_bytes(),
            [Some(payload.clone())],
        )
        .unwrap();
        assert_eq!(record.inline(), &5_u64.to_le_bytes());
        assert_eq!(record.payload_owner(0).unwrap().as_slice(), &[7; 512]);
        assert!(record.belongs_to(&first));
        assert!(!record.belongs_to(&foreign));
        let reader = record.clone();
        drop(payload);
        drop(record);
        assert_eq!(first.used_bytes(), bytes);
        drop(reader);
        assert_eq!(first.used_bytes(), 0);

        let fixed = PublicationRecord::<8, 0>::try_new(&foreign, 8_u64.to_le_bytes(), []).unwrap();
        assert!(fixed.belongs_to(&first));
    }

    #[test]
    fn immutable_payload_can_take_a_whole_tick_reservation_without_reserving_twice() {
        let header = PublicationBytes::required_charge(0).unwrap();
        let pool = PublicationPool::new(header + 8);
        let mut reservation = pool.try_reserve(header + 8).unwrap();
        assert!(matches!(
            PublicationBytes::try_copy_from_reservation(&mut reservation, b"123456789"),
            Err(PublicationCapacity)
        ));
        assert_eq!(reservation.remaining_bytes(), header + 8);
        let first = PublicationBytes::try_copy_from_reservation(&mut reservation, b"1234").unwrap();
        assert_eq!(reservation.remaining_bytes(), 4);
        assert_eq!(pool.used_bytes(), header + 8);
        drop(reservation);
        assert_eq!(pool.used_bytes(), header + 4);
        drop(first);
        assert_eq!(pool.used_bytes(), 0);
    }
}
