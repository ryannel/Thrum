//! Selected charged-column storage probe with a private logical fill limit.
//!
//! Rust guarantees `Vec::with_capacity(n)` requests storage for exactly `n`
//! elements; allocator-supplied surplus is not available through this API.
//! Reserve the requested bytes before allocating, then never grow or shrink
//! the vector. This standalone evidence fixture stays outside the engine
//! until qualified groups and their replica/host handoff are integrated.

#![cfg(test)]

use std::{alloc::Layout, sync::Arc};

use thrum_core::{PublicationCapacity, PublicationCharge, PublicationReservation};

struct ColumnStorage<T> {
    values: Vec<T>,
    limit: usize,
    _charge: PublicationCharge,
}

/// Shared owner for one charged publication column. Zero capacity has no
/// allocation or charge.
struct PublicationColumn<T> {
    inner: Option<Arc<ColumnStorage<T>>>,
}

/// A unique owner can move values out without an additional allocation.
struct UniquePublicationColumn<T> {
    values: std::vec::IntoIter<T>,
    _charge: Option<PublicationCharge>,
}

enum ColumnWriteError<T> {
    Full(T),
    Shared(T),
}

impl<T> Clone for PublicationColumn<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> PublicationColumn<T> {
    fn required_charge(capacity: usize) -> Result<usize, PublicationCapacity> {
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

    fn try_with_capacity(
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

    fn len(&self) -> usize {
        self.inner.as_ref().map_or(0, |inner| inner.values.len())
    }

    fn capacity_limit(&self) -> usize {
        self.inner.as_ref().map_or(0, |inner| inner.limit)
    }

    fn as_slice(&self) -> &[T] {
        self.inner.as_ref().map_or(&[], |inner| &inner.values)
    }

    fn try_push(&mut self, value: T) -> Result<(), ColumnWriteError<T>> {
        let Some(inner) = &mut self.inner else {
            return Err(ColumnWriteError::Full(value));
        };
        let Some(inner) = Arc::get_mut(inner) else {
            return Err(ColumnWriteError::Shared(value));
        };
        if inner.values.len() == inner.limit {
            return Err(ColumnWriteError::Full(value));
        }
        inner.values.push(value);
        Ok(())
    }

    fn into_unique(self) -> Result<UniquePublicationColumn<T>, Self> {
        let Some(inner) = self.inner else {
            return Ok(UniquePublicationColumn {
                values: Vec::new().into_iter(),
                _charge: None,
            });
        };
        match Arc::try_unwrap(inner) {
            Ok(inner) => Ok(UniquePublicationColumn {
                values: inner.values.into_iter(),
                _charge: Some(inner._charge),
            }),
            Err(inner) => Err(Self { inner: Some(inner) }),
        }
    }
}

impl<T> UniquePublicationColumn<T> {
    fn take_front(&mut self) -> Option<T> {
        self.values.next()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::{
            Barrier,
            atomic::{AtomicUsize, Ordering},
        },
        thread,
    };

    use thrum_core::PublicationPool;

    use super::*;

    struct CountDrop(Arc<AtomicUsize>);
    static ZST_DROPS: AtomicUsize = AtomicUsize::new(0);
    struct ZstDrop;

    impl Drop for ZstDrop {
        fn drop(&mut self) {
            ZST_DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }

    impl Drop for CountDrop {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn column_rejects_before_allocation_and_releases_after_last_reader() {
        let bytes = PublicationColumn::<u64>::required_charge(2).unwrap();
        assert_eq!(PublicationColumn::<u64>::required_charge(0), Ok(0));
        assert_eq!(
            PublicationColumn::<u64>::required_charge(usize::MAX),
            Err(PublicationCapacity)
        );
        let pool = PublicationPool::new(bytes);
        let mut too_small = pool.try_reserve(bytes - 1).unwrap();
        assert!(PublicationColumn::<u64>::try_with_capacity(&mut too_small, 2).is_err());
        assert_eq!(too_small.remaining_bytes(), bytes - 1);
        drop(too_small);
        assert_eq!(pool.used_bytes(), 0);

        let mut reservation = pool.try_reserve(bytes).unwrap();
        let mut column = PublicationColumn::<u64>::try_with_capacity(&mut reservation, 2).unwrap();
        drop(reservation);
        assert_eq!(column.capacity_limit(), 2);
        assert_eq!(column.len(), 0);
        column.try_push(3).ok().unwrap();
        column.try_push(4).ok().unwrap();
        assert_eq!(column.as_slice(), &[3, 4]);
        assert!(matches!(column.try_push(5), Err(ColumnWriteError::Full(5))));
        let reader = column.clone();
        assert!(matches!(
            column.try_push(6),
            Err(ColumnWriteError::Shared(6))
        ));
        drop(column);
        assert_eq!(pool.used_bytes(), bytes);
        assert_eq!(reader.as_slice(), &[3, 4]);
        drop(reader);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn partial_unwind_and_unique_transfer_drop_each_value_once() {
        let bytes = PublicationColumn::<CountDrop>::required_charge(3).unwrap();
        let pool = PublicationPool::new(bytes);
        let drops = Arc::new(AtomicUsize::new(0));
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut reservation = pool.try_reserve(bytes).unwrap();
            let mut column = PublicationColumn::try_with_capacity(&mut reservation, 3).unwrap();
            column.try_push(CountDrop(drops.clone())).ok().unwrap();
            panic!("cancel candidate while partly filled");
        }));
        assert!(result.is_err());
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert_eq!(pool.used_bytes(), 0);

        let mut reservation = pool.try_reserve(bytes).unwrap();
        let mut column = PublicationColumn::try_with_capacity(&mut reservation, 3).unwrap();
        for _ in 0..3 {
            column.try_push(CountDrop(drops.clone())).ok().unwrap();
        }
        drop(reservation);
        let reader = column.clone();
        let column = match column.into_unique() {
            Err(shared) => shared,
            Ok(_) => panic!("shared column cannot move values"),
        };
        drop(reader);
        let mut unique = match column.into_unique() {
            Ok(unique) => unique,
            Err(_) => panic!("sole owner must transfer"),
        };
        let first = unique.take_front().unwrap();
        drop(first);
        assert_eq!(drops.load(Ordering::Relaxed), 2);
        assert_eq!(pool.used_bytes(), bytes);
        drop(unique);
        assert_eq!(drops.load(Ordering::Relaxed), 4);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn empty_and_zero_sized_columns_keep_a_finite_logical_limit() {
        let empty_pool = PublicationPool::new(0);
        let mut empty_reservation = empty_pool.try_reserve(0).unwrap();
        let mut empty =
            PublicationColumn::<ZstDrop>::try_with_capacity(&mut empty_reservation, 0).unwrap();
        assert_eq!(empty.len(), 0);
        assert_eq!(empty.capacity_limit(), 0);
        assert!(empty.as_slice().is_empty());
        let reader = empty.clone();
        assert!(matches!(
            empty.try_push(ZstDrop),
            Err(ColumnWriteError::Full(_))
        ));
        drop(reader);
        let mut unique = empty.into_unique().ok().unwrap();
        assert!(unique.take_front().is_none());
        drop(unique);
        assert_eq!(empty_pool.used_bytes(), 0);

        let bytes = PublicationColumn::<ZstDrop>::required_charge(3).unwrap();
        let pool = PublicationPool::new(bytes);
        let mut reservation = pool.try_reserve(bytes).unwrap();
        let mut column =
            PublicationColumn::<ZstDrop>::try_with_capacity(&mut reservation, 3).unwrap();
        let before = ZST_DROPS.load(Ordering::Relaxed);
        for _ in 0..3 {
            column.try_push(ZstDrop).ok().unwrap();
        }
        assert_eq!(column.len(), 3);
        assert_eq!(column.capacity_limit(), 3);
        assert!(matches!(
            column.try_push(ZstDrop),
            Err(ColumnWriteError::Full(_))
        ));
        drop(reservation);
        let mut unique = column.into_unique().ok().unwrap();
        while let Some(value) = unique.take_front() {
            drop(value);
            assert_eq!(pool.used_bytes(), bytes);
        }
        drop(unique);
        assert_eq!(pool.used_bytes(), 0);
        assert_eq!(ZST_DROPS.load(Ordering::Relaxed) - before, 4);
    }

    #[test]
    fn concurrent_shared_readers_keep_one_charge_until_final_release() {
        let bytes = PublicationColumn::<u64>::required_charge(1).unwrap();
        let pool = PublicationPool::new(bytes);
        let mut reservation = pool.try_reserve(bytes).unwrap();
        let mut column = PublicationColumn::<u64>::try_with_capacity(&mut reservation, 1).unwrap();
        column.try_push(42).ok().unwrap();
        drop(reservation);
        let barrier = Arc::new(Barrier::new(3));
        let readers: Vec<_> = (0..2)
            .map(|_| {
                let reader = column.clone();
                let other_pool = pool.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    assert_eq!(reader.as_slice(), &[42]);
                    assert!(other_pool.try_reserve(1).is_err());
                    drop(reader);
                })
            })
            .collect();
        barrier.wait();
        for reader in readers {
            reader.join().unwrap();
        }
        assert_eq!(pool.used_bytes(), bytes);
        drop(column);
        assert_eq!(pool.used_bytes(), 0);
        drop(pool.try_reserve(bytes).unwrap());
    }

    #[test]
    #[ignore = "run explicitly in release mode to measure local clone cost"]
    fn local_clone_cost_probe() {
        use std::{hint::black_box, time::Instant};

        fn sample(mut clone_once: impl FnMut()) -> (u128, u128) {
            const BATCH: usize = 256;
            for _ in 0..10 {
                for _ in 0..BATCH {
                    clone_once();
                }
            }
            let mut times = Vec::with_capacity(100);
            for _ in 0..100 {
                let start = Instant::now();
                for _ in 0..BATCH {
                    clone_once();
                }
                times.push(start.elapsed().as_nanos());
            }
            times.sort_unstable();
            (times[49], times[94])
        }

        for count in [1, 4_096] {
            let pool = PublicationPool::new(usize::MAX);
            let bytes = PublicationColumn::<u64>::required_charge(count).unwrap();
            let mut reservation = pool.try_reserve(bytes).unwrap();
            let mut column = PublicationColumn::try_with_capacity(&mut reservation, count).unwrap();
            let mut plain = Vec::with_capacity(count);
            for value in 0..count as u64 {
                column.try_push(value).ok().unwrap();
                plain.push(value);
            }
            drop(reservation);
            let deep = sample(|| {
                black_box(black_box(&plain).clone());
            });
            let shared = sample(|| {
                black_box(black_box(&column).clone());
            });
            println!(
                "column/{count}: clone_batch_256_ns[p50/p95] deep_vec={}/{} shared_owner={}/{}",
                deep.0, deep.1, shared.0, shared.1
            );
        }
    }
}
