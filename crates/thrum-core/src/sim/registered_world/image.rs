use crate::profile::{Digest, StateDecode, StateDecodeError as DecodeError, StateReader};

use super::{DenseDirectory, IdIndex, RegisteredWorld, Slot, StableId, StorageColumn, tick_index};
use crate::sim::{ComponentSchemaId, WorldImageError};

pub(in crate::sim) struct DecodedWorld {
    slots: Vec<Slot>,
    free: Vec<u32>,
    ids: IdIndex,
    live: usize,
    directory: Option<DenseDirectory>,
    columns: Vec<Box<dyn StorageColumn>>,
    change_ticks: Vec<u64>,
}

fn charge<T>(reader: &mut StateReader<'_>, count: usize) -> Result<(), DecodeError> {
    reader.charge_allocation(
        count
            .checked_mul(size_of::<T>())
            .ok_or(DecodeError::Limit)?,
    )
}

impl RegisteredWorld {
    pub(in crate::sim) fn image_schema(&self) -> Result<(), ComponentSchemaId> {
        self.registry
            .entries
            .iter()
            .try_for_each(|entry| entry.profile.as_ref().map(|_| ()).ok_or(entry.schema))
    }

    pub(in crate::sim) fn encode_image(&self, sink: &mut Digest<'_>) {
        self.encode_metadata(sink);
        for index in self.schema_order() {
            self.encode_column(index, sink)
                .expect("checked image schema");
        }
    }

    pub(in crate::sim) fn decode_image(
        &self,
        reader: &mut StateReader<'_>,
        tick: u64,
    ) -> Result<DecodedWorld, WorldImageError> {
        let capacity = self.capacity();
        if u64::decode(reader)? != u64::try_from(capacity).expect("world capacity fits u64") {
            return Err(DecodeError::Invalid.into());
        }
        if capacity > reader.remaining() / 33 {
            return Err(DecodeError::Truncated.into());
        }
        charge::<Slot>(reader, capacity)?;
        charge::<Option<(StableId, u32)>>(reader, self.ids.buckets.len())?;
        let mut world = DecodedWorld {
            slots: Vec::with_capacity(capacity),
            free: Vec::new(),
            ids: IdIndex::new(capacity),
            live: 0,
            directory: None,
            columns: Vec::new(),
            change_ticks: Vec::new(),
        };
        world.decode_slots(reader, tick, capacity)?;
        world.decode_free(reader)?;
        world.decode_identity_index(reader)?;
        if bool::decode(reader)? != self.directory.is_some() {
            return Err(DecodeError::Invalid.into());
        }
        if self.directory.is_some() {
            world.directory = Some(DenseDirectory::decode(reader, capacity)?);
        }
        self.decode_columns(reader, tick, &mut world)?;
        world.validate_directory(self.activity_component)?;
        Ok(world)
    }

    fn decode_columns(
        &self,
        reader: &mut StateReader<'_>,
        tick: u64,
        decoded: &mut DecodedWorld,
    ) -> Result<(), WorldImageError> {
        let count = self.component_count();
        let capacity = self.capacity();
        charge::<usize>(reader, count)?;
        charge::<Box<dyn StorageColumn>>(reader, count)?;
        charge::<u64>(reader, self.change_ticks.len())?;
        for entry in &self.registry.entries {
            let profile = entry
                .profile
                .as_ref()
                .ok_or(WorldImageError::Schema(entry.schema))?;
            let bytes =
                (profile.storage_bytes)(capacity, entry.indexed).ok_or(DecodeError::Limit)?;
            reader.charge_allocation(bytes)?;
        }
        decoded.columns = self
            .registry
            .entries
            .iter()
            .map(|entry| (entry.storage)(capacity))
            .collect();
        decoded.change_ticks = vec![0; self.change_ticks.len()];
        for index in self.schema_order() {
            let entry = &self.registry.entries[index];
            let profile = entry.profile.as_ref().expect("checked image schema");
            if u128::decode(reader)? != entry.schema.0 {
                return Err(DecodeError::Invalid.into());
            }
            let start = tick_index(index, 0, capacity);
            for changed in &mut decoded.change_ticks[start..start + capacity] {
                *changed = u64::decode(reader)?;
                if *changed > tick {
                    return Err(DecodeError::Invalid.into());
                }
            }
            for (slot, state) in decoded.slots.iter().enumerate() {
                if bool::decode(reader)? {
                    if state.stable.is_none() {
                        return Err(DecodeError::Invalid.into());
                    }
                    let row = decoded
                        .directory
                        .as_ref()
                        .map_or(slot, |directory| directory.row_of(slot));
                    (profile.decode)(decoded.columns[index].as_mut(), row, reader)?;
                }
            }
        }
        Ok(())
    }

    pub(in crate::sim) fn install_image(&mut self, decoded: DecodedWorld) {
        self.slots = decoded.slots;
        self.free = decoded.free;
        self.ids = decoded.ids;
        self.live = decoded.live;
        self.directory = decoded.directory;
        self.columns = decoded.columns;
        self.change_ticks = decoded.change_ticks;
        self.begin_tick();
    }
}

impl DecodedWorld {
    fn decode_slots(
        &mut self,
        reader: &mut StateReader<'_>,
        tick: u64,
        capacity: usize,
    ) -> Result<(), DecodeError> {
        for index in 0..capacity {
            let live = bool::decode(reader)?;
            let generation = u64::decode(reader)?;
            let id = StableId(u128::decode(reader)?);
            let structural_tick = u64::decode(reader)?;
            if generation == 0 || structural_tick > tick || (!live && id.0 != 0) {
                return Err(DecodeError::Invalid);
            }
            if live {
                if self.ids.get(id).is_some() {
                    return Err(DecodeError::Invalid);
                }
                self.ids
                    .insert(id, u32::try_from(index).expect("world capacity fits u32"));
                self.live += 1;
            }
            self.slots.push(Slot {
                generation,
                stable: live.then_some(id),
                structural_tick,
            });
        }
        Ok(())
    }

    fn decode_free(&mut self, reader: &mut StateReader<'_>) -> Result<(), DecodeError> {
        let capacity = self.slots.len();
        let count = usize::try_from(u64::decode(reader)?).map_err(|_| DecodeError::Invalid)?;
        if count > capacity - self.live {
            return Err(DecodeError::Invalid);
        }
        // Keep the original spare capacity for subsequent spawn and despawn.
        charge::<u32>(reader, capacity)?;
        charge::<bool>(reader, capacity)?;
        self.free = Vec::with_capacity(capacity);
        let mut seen = vec![false; capacity];
        for _ in 0..count {
            let slot = u32::decode(reader)?;
            let index = slot as usize;
            if self
                .slots
                .get(index)
                .is_none_or(|entry| entry.stable.is_some())
                || seen[index]
            {
                return Err(DecodeError::Invalid);
            }
            seen[index] = true;
            self.free.push(slot);
        }
        if self
            .slots
            .iter()
            .zip(seen)
            .any(|(slot, free)| !free && slot.stable.is_none() && slot.generation != u64::MAX)
        {
            return Err(DecodeError::Invalid);
        }
        Ok(())
    }

    fn decode_identity_index(&self, reader: &mut StateReader<'_>) -> Result<(), DecodeError> {
        for slot in &self.slots {
            let found = bool::decode(reader)?;
            let index = u32::decode(reader)?;
            let expected = slot.stable.and_then(|id| self.ids.get(id));
            if found != expected.is_some() || index != expected.unwrap_or(0) {
                return Err(DecodeError::Invalid);
            }
        }
        Ok(())
    }

    fn validate_directory(&self, activity: Option<usize>) -> Result<(), DecodeError> {
        if let Some(directory) = &self.directory {
            let words = self.columns[activity.expect("directory has an activity component")]
                .active_presence()
                .expect("activity component is indexed");
            for row in 0..self.slots.len() {
                let indexed = words[row / 64] & (1 << (row % 64)) != 0;
                if indexed != (row < directory.active_len()) {
                    return Err(DecodeError::Invalid);
                }
            }
        }
        Ok(())
    }
}
