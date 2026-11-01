//! Process-shared reservation for immutable publication allocations.
//!
//! Reservation and ownership charges are distinct: a reservation is made
//! before a world step; committed allocations keep their charge until the
//! final owner releases them. The pool does not infer game-owned heap usage.

use std::{
    alloc::Layout,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::world::StableId;

struct PoolState {
    limit: usize,
    used: AtomicUsize,
}

/// A process-shared logical byte budget for published immutable data.
#[derive(Clone)]
pub struct PublicationPool {
    state: Arc<PoolState>,
}

impl PublicationPool {
    pub(crate) fn same_pool(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }
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

struct ColumnStorage<T> {
    values: Vec<T>,
    limit: usize,
    _charge: PublicationCharge,
}

/// Shared, fixed-fill publication storage. Only its builder can push values;
/// clones share one charged owner and never duplicate the vector buffer.
pub(crate) struct PublicationColumn<T> {
    inner: Option<Arc<ColumnStorage<T>>>,
}

/// A uniquely owned column keeps its buffer charge until iteration and any
/// unconsumed values are dropped. Moved records need a separately admitted
/// destination, while their immutable payload handles retain their own charge.
pub(crate) struct ChargedColumnIntoIter<T> {
    values: std::vec::IntoIter<T>,
    _charge: Option<PublicationCharge>,
}

impl<T> Iterator for ChargedColumnIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.values.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.values.size_hint()
    }
}

impl<T> ExactSizeIterator for ChargedColumnIntoIter<T> {}

impl<T> Clone for PublicationColumn<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> PublicationColumn<T> {
    pub(crate) fn try_into_unique_iter(self) -> Result<ChargedColumnIntoIter<T>, Self> {
        let Some(inner) = self.inner else {
            return Ok(ChargedColumnIntoIter {
                values: Vec::new().into_iter(),
                _charge: None,
            });
        };
        match Arc::try_unwrap(inner) {
            Ok(ColumnStorage {
                values, _charge, ..
            }) => Ok(ChargedColumnIntoIter {
                values: values.into_iter(),
                _charge: Some(_charge),
            }),
            Err(inner) => Err(Self { inner: Some(inner) }),
        }
    }

    pub(crate) fn belongs_to(&self, pool: &PublicationPool) -> bool {
        self.inner
            .as_ref()
            .is_none_or(|owner| Arc::ptr_eq(&owner._charge.state, &pool.state))
    }

    pub(crate) fn required_charge(capacity: usize) -> Result<usize, PublicationCapacity> {
        if capacity == 0 {
            return Ok(0);
        }
        Layout::array::<T>(capacity)
            .map_err(|_| PublicationCapacity)?
            .size()
            .checked_add(size_of::<ColumnStorage<T>>())
            .and_then(|bytes| bytes.checked_add(2 * size_of::<usize>()))
            .ok_or(PublicationCapacity)
    }

    pub(crate) fn try_with_capacity(
        reservation: &mut PublicationReservation,
        capacity: usize,
    ) -> Result<Self, PublicationCapacity> {
        if capacity == 0 {
            return Ok(Self { inner: None });
        }
        let charge = reservation.charge(Self::required_charge(capacity)?)?;
        Ok(Self {
            inner: Some(Arc::new(ColumnStorage {
                values: Vec::with_capacity(capacity),
                limit: capacity,
                _charge: charge,
            })),
        })
    }

    /// Take a vector whose requested storage was covered by a pre-edit
    /// reservation. The logical limit is the request bound, not allocator
    /// surplus in `capacity()`.
    pub(crate) fn try_from_reserved_vec(
        reservation: &mut PublicationReservation,
        values: Vec<T>,
        requested_limit: usize,
    ) -> Result<Self, PublicationCapacity> {
        assert!(values.len() <= requested_limit, "column preflight bound");
        if values.is_empty() {
            return Ok(Self { inner: None });
        }
        let charge = reservation.charge(Self::required_charge(requested_limit)?)?;
        Ok(Self {
            inner: Some(Arc::new(ColumnStorage {
                values,
                limit: requested_limit,
                _charge: charge,
            })),
        })
    }

    pub(crate) fn push(&mut self, value: T) {
        let inner = self.inner.as_mut().expect("nonempty reserved column");
        let storage = Arc::get_mut(inner).expect("column builder has one owner");
        assert!(storage.values.len() < storage.limit, "column fill limit");
        storage.values.push(value);
    }

    pub(crate) fn as_slice(&self) -> &[T] {
        self.inner.as_ref().map_or(&[], |inner| &inner.values)
    }

    pub(crate) fn sort_unstable_by_key<K: Ord>(&mut self, mut key: impl FnMut(&T) -> K) {
        if let Some(inner) = &mut self.inner {
            Arc::get_mut(inner)
                .expect("column builder has one owner")
                .values
                .sort_unstable_by_key(|value| key(value));
        }
    }
}

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

impl sealed::ClosedValue for () {}

/// Engine-owned state that can be checked before charged authority admission.
/// This boundary is sealed so a game type cannot claim an unchecked owner is
/// charged. It does not by itself establish a finite whole-tick profile.
pub trait ClosedPublicationValue: Clone + sealed::ClosedValue {
    /// Check fixed type shape once, even when the initial world is empty.
    fn validate_shape(limits: ClosedValueLimits) -> Result<(), ClosedValueError>;
    /// Check every retained payload owner against this pool and byte limit.
    fn validate_for_pool(
        &self,
        pool: &PublicationPool,
        limits: ClosedValueLimits,
    ) -> Result<(), ClosedValueError>;
}

impl ClosedPublicationValue for () {
    fn validate_shape(_limits: ClosedValueLimits) -> Result<(), ClosedValueError> {
        Ok(())
    }

    fn validate_for_pool(
        &self,
        _pool: &PublicationPool,
        _limits: ClosedValueLimits,
    ) -> Result<(), ClosedValueError> {
        Ok(())
    }
}

macro_rules! closed_scalar {
    ($ty:ty) => {
        impl sealed::ClosedValue for $ty {}

        impl ClosedPublicationValue for $ty {
            fn validate_shape(limits: ClosedValueLimits) -> Result<(), ClosedValueError> {
                if size_of::<Self>() > limits.max_inline_bytes {
                    Err(ClosedValueError::InlineLimit)
                } else {
                    Ok(())
                }
            }

            fn validate_for_pool(
                &self,
                _pool: &PublicationPool,
                _limits: ClosedValueLimits,
            ) -> Result<(), ClosedValueError> {
                Ok(())
            }
        }
    };
}

closed_scalar!(u8);
closed_scalar!(StableId);

/// Per-value limits selected for one closed-state authority profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClosedValueLimits {
    /// Maximum fixed inline bytes in one state record.
    pub max_inline_bytes: usize,
    /// Maximum optional payload slots in one state record.
    pub max_payload_slots: usize,
    /// Maximum bytes in one immutable payload owner.
    pub max_payload_bytes: usize,
}

/// A closed state cannot enter the selected authority profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClosedValueError {
    /// A payload was charged to another process pool.
    WrongPool,
    /// Fixed inline storage exceeds the selected maximum.
    InlineLimit,
    /// Payload handle slots exceed the selected maximum.
    PayloadSlotLimit,
    /// One immutable payload exceeds the selected maximum.
    PayloadByteLimit,
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
    fn validate_shape(limits: ClosedValueLimits) -> Result<(), ClosedValueError> {
        if INLINE > limits.max_inline_bytes {
            return Err(ClosedValueError::InlineLimit);
        }
        if BLOBS > limits.max_payload_slots {
            return Err(ClosedValueError::PayloadSlotLimit);
        }
        Ok(())
    }

    fn validate_for_pool(
        &self,
        pool: &PublicationPool,
        limits: ClosedValueLimits,
    ) -> Result<(), ClosedValueError> {
        for payload in self.blobs.iter().flatten() {
            if !payload.belongs_to(pool) {
                return Err(ClosedValueError::WrongPool);
            }
            if payload.as_slice().len() > limits.max_payload_bytes {
                return Err(ClosedValueError::PayloadByteLimit);
            }
        }
        Ok(())
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

impl PublicationCharge {
    pub(crate) fn belongs_to(&self, reservation: &PublicationReservation) -> bool {
        Arc::ptr_eq(&self.state, &reservation.state)
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
    fn unique_column_moves_values_and_holds_charge_through_iterator_drop() {
        let bytes = PublicationColumn::<u8>::required_charge(2).unwrap();
        let pool = PublicationPool::new(bytes);
        let mut reservation = pool.try_reserve(bytes).unwrap();
        let mut column: PublicationColumn<u8> =
            PublicationColumn::try_with_capacity(&mut reservation, 2).unwrap();
        column.push(7);
        column.push(9);
        drop(reservation);
        let shared_reader = column.clone();
        let column = match column.try_into_unique_iter() {
            Ok(_) => panic!("shared column must remain shared"),
            Err(column) => column,
        };
        drop(shared_reader);
        let mut values = column.try_into_unique_iter().ok().unwrap();
        assert_eq!(values.next(), Some(7));
        assert_eq!(pool.used_bytes(), bytes);
        drop(values);
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
    fn closed_record_profile_checks_shape_and_payload_length() {
        let pool = PublicationPool::new(10_000);
        let limits = ClosedValueLimits {
            max_inline_bytes: 8,
            max_payload_slots: 1,
            max_payload_bytes: 32,
        };
        let large_inline = PublicationRecord::<16, 0>::try_new(&pool, [0; 16], []).unwrap();
        assert_eq!(
            PublicationRecord::<16, 0>::validate_shape(limits),
            Err(ClosedValueError::InlineLimit)
        );
        let many_slots = PublicationRecord::<8, 2>::try_new(&pool, [0; 8], [None, None]).unwrap();
        assert_eq!(
            PublicationRecord::<8, 2>::validate_shape(limits),
            Err(ClosedValueError::PayloadSlotLimit)
        );
        assert!(large_inline.validate_for_pool(&pool, limits).is_ok());
        assert!(many_slots.validate_for_pool(&pool, limits).is_ok());
        let payload = PublicationBytes::try_copy_from_slice(&pool, &[1; 33]).unwrap();
        let large_payload =
            PublicationRecord::<8, 1>::try_new(&pool, [0; 8], [Some(payload)]).unwrap();
        assert_eq!(
            large_payload.validate_for_pool(&pool, limits),
            Err(ClosedValueError::PayloadByteLimit)
        );
        assert_eq!(
            large_payload.validate_for_pool(
                &PublicationPool::new(10_000),
                ClosedValueLimits {
                    max_payload_bytes: 33,
                    ..limits
                }
            ),
            Err(ClosedValueError::WrongPool)
        );
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
