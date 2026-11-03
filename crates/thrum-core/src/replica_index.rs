//! Finite confirmed-object index for a required local replica.
//! The pinned hashbrown layout is reserved before allocation; a separate live
//! limit prevents subsequent growth even after deletion tombstones accumulate.

use std::{alloc::Layout, collections::hash_map::RandomState};

use hashbrown::HashMap;

use crate::{PublicationCapacity, PublicationCharge, PublicationPool, StableId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IndexError {
    Full,
    Duplicate,
    Missing,
}

pub(crate) struct ChargedReplicaIndex<T> {
    // Drop the table before its charge. No method exposes reserve, shrink,
    // clone or unrestricted mutable access to the map.
    table: HashMap<StableId, T, RandomState>,
    _charge: PublicationCharge,
    max_live: usize,
    allocation_bytes: usize,
}

impl<T> ChargedReplicaIndex<T> {
    /// Upper bound for hashbrown 0.17.1's requested table allocation with
    /// `2 * max_live` entries reserved. The exact dependency pin and a
    /// post-allocation assertion make a layout change explicit.
    pub(crate) fn required_peak(max_live: usize) -> Result<usize, PublicationCapacity> {
        let requested = max_live.checked_mul(2).ok_or(PublicationCapacity)?;
        if requested == 0 {
            return Ok(0);
        }
        let buckets = if requested < 15 {
            if requested < 4 {
                4_usize
            } else if requested < 8 {
                8
            } else {
                16
            }
        } else {
            requested
                .checked_mul(8)
                .and_then(|value| value.checked_div(7))
                .and_then(usize::checked_next_power_of_two)
                .ok_or(PublicationCapacity)?
        };
        let entry = Layout::new::<(StableId, T)>();
        let control_alignment = entry.align().max(16);
        let entries = buckets
            .checked_mul(entry.size())
            .ok_or(PublicationCapacity)?;
        let control_offset = entries
            .checked_add(control_alignment - 1)
            .map(|value| value & !(control_alignment - 1))
            .ok_or(PublicationCapacity)?;
        let bytes = control_offset
            .checked_add(buckets)
            .and_then(|value| value.checked_add(16))
            .ok_or(PublicationCapacity)?;
        if bytes > isize::MAX as usize - (control_alignment - 1) {
            return Err(PublicationCapacity);
        }
        Ok(bytes)
    }

    pub(crate) fn try_new(
        pool: &PublicationPool,
        max_live: usize,
    ) -> Result<Self, PublicationCapacity> {
        let requested = max_live.checked_mul(2).ok_or(PublicationCapacity)?;
        let mut reservation = pool.try_reserve(Self::required_peak(max_live)?)?;
        let mut table = HashMap::with_hasher(RandomState::new());
        table
            .try_reserve(requested)
            .map_err(|_| PublicationCapacity)?;
        let allocation_bytes = table.allocation_size();
        assert!(
            allocation_bytes <= Self::required_peak(max_live)?,
            "pinned hashbrown table exceeded its preallocation bound"
        );
        let charge = reservation.charge(allocation_bytes)?;
        Ok(Self {
            table,
            _charge: charge,
            max_live,
            allocation_bytes,
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.table.len()
    }

    pub(crate) fn max_live(&self) -> usize {
        self.max_live
    }

    pub(crate) fn allocation_bytes(&self) -> usize {
        self.allocation_bytes
    }

    pub(crate) fn get(&self, id: StableId) -> Option<&T> {
        self.table.get(&id)
    }

    pub(crate) fn contains(&self, id: StableId) -> bool {
        self.table.contains_key(&id)
    }

    pub(crate) fn can_add(&self, max_spawns: usize) -> bool {
        self.len()
            .checked_add(max_spawns)
            .is_some_and(|peak| peak <= self.max_live)
    }

    pub(crate) fn insert_new(&mut self, id: StableId, value: T) -> Result<(), IndexError> {
        if self.table.contains_key(&id) {
            return Err(IndexError::Duplicate);
        }
        if self.table.len() == self.max_live {
            return Err(IndexError::Full);
        }
        assert!(self.table.insert(id, value).is_none());
        self.assert_stable_allocation();
        Ok(())
    }

    pub(crate) fn replace(&mut self, id: StableId, value: T) -> Result<T, IndexError> {
        let slot = self.table.get_mut(&id).ok_or(IndexError::Missing)?;
        Ok(std::mem::replace(slot, value))
    }

    pub(crate) fn remove(&mut self, id: StableId) -> Result<T, IndexError> {
        let value = self.table.remove(&id).ok_or(IndexError::Missing)?;
        self.assert_stable_allocation();
        Ok(value)
    }

    fn assert_stable_allocation(&self) {
        assert_eq!(
            self.table.allocation_size(),
            self.allocation_bytes,
            "qualified replica table changed its allocation"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    #[repr(align(32))]
    struct Aligned([u8; 32]);

    #[test]
    fn pinned_table_layout_stays_within_precharged_bound() {
        fn check<T>() {
            for max_live in (0..=64).chain([127, 128, 129, 4_096, 16_384]) {
                let bound = ChargedReplicaIndex::<T>::required_peak(max_live).unwrap();
                let pool = PublicationPool::new(bound);
                let index = ChargedReplicaIndex::<T>::try_new(&pool, max_live).unwrap();
                assert!(index.allocation_bytes() <= bound);
                assert_eq!(pool.used_bytes(), index.allocation_bytes());
                drop(index);
                assert_eq!(pool.used_bytes(), 0);
            }
        }
        check::<()>();
        check::<u8>();
        check::<[u8; 32]>();
        check::<Aligned>();
        assert_eq!(size_of::<Aligned>(), 32);
        assert_eq!((Aligned([0; 32])).0.len(), 32);
    }

    #[test]
    fn reservation_precedes_table_allocation_and_releases_with_owner() {
        let peak = ChargedReplicaIndex::<[u8; 32]>::required_peak(64).unwrap();
        let refused = PublicationPool::new(peak - 1);
        assert!(ChargedReplicaIndex::<[u8; 32]>::try_new(&refused, 64).is_err());
        assert_eq!(refused.used_bytes(), 0);

        let pool = PublicationPool::new(peak);
        let mut index: ChargedReplicaIndex<[u8; 32]> =
            ChargedReplicaIndex::try_new(&pool, 64).unwrap();
        assert_eq!(pool.used_bytes(), index.allocation_bytes());
        assert_eq!(index.max_live(), 64);
        assert!(index.can_add(64));
        index.insert_new(StableId(1), [1; 32]).unwrap();
        assert_eq!(index.get(StableId(1)), Some(&[1; 32]));
        assert_eq!(index.replace(StableId(1), [2; 32]), Ok([1; 32]));
        assert_eq!(index.get(StableId(1)), Some(&[2; 32]));
        assert_eq!(index.remove(StableId(1)), Ok([2; 32]));
        assert_eq!(pool.used_bytes(), index.allocation_bytes());
        drop(index);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn maximum_live_count_prevents_tombstone_growth_through_long_churn() {
        let pool = PublicationPool::new(usize::MAX);
        let mut index = ChargedReplicaIndex::try_new(&pool, 28).unwrap();
        let bytes = index.allocation_bytes();
        let mut live = VecDeque::new();
        for id in 1..=28_u128 {
            index
                .insert_new(
                    StableId(id),
                    u64::try_from(id).expect("fixture ID fits u64"),
                )
                .unwrap();
            live.push_back(id);
        }
        assert!(!index.can_add(1));
        assert_eq!(index.insert_new(StableId(29), 29), Err(IndexError::Full));
        assert_eq!(
            index.insert_new(StableId(1), 99),
            Err(IndexError::Duplicate)
        );
        for step in 0..100_000_u128 {
            let old = live.pop_front().unwrap();
            assert_eq!(
                index.remove(StableId(old)),
                Ok(u64::try_from(old).expect("fixture ID fits u64"))
            );
            let new = step + 29;
            index
                .insert_new(
                    StableId(new),
                    u64::try_from(new).expect("fixture ID fits u64"),
                )
                .unwrap();
            live.push_back(new);
            if step % 1_000 == 0 {
                assert_eq!(index.len(), 28);
                assert_eq!(index.allocation_bytes(), bytes);
                assert_eq!(pool.used_bytes(), bytes);
                for id in &live {
                    assert_eq!(
                        index.get(StableId(*id)),
                        Some(&(u64::try_from(*id).expect("fixture ID fits u64")))
                    );
                }
            }
        }
        drop(index);
        assert_eq!(pool.used_bytes(), 0);
    }
}
