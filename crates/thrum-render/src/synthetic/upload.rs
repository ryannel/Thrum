//! Reused instance packing and sparse queue writes for the diagnostic.

use std::ops::Range;

use crate::abi::{self, InstanceRecord};

use super::{InstanceUploadReport, RenderError};

const MAX_PARTIAL_WRITES: usize = 64;

struct UploadRun {
    source: Range<usize>,
    destination: u32,
}

pub(super) struct PackedInstances {
    pub(super) bytes: Vec<u8>,
    physical_to_slot: Vec<u32>,
    slot_to_physical: Vec<u32>,
    active_count: u32,
}

impl PackedInstances {
    pub(super) fn new(
        instances: &[InstanceRecord],
        material_count: u32,
    ) -> Result<Self, RenderError> {
        let count = u32::try_from(instances.len()).map_err(|_| RenderError::SceneCapacity)?;
        let mut physical_to_slot = Vec::with_capacity(instances.len());
        for active in [true, false] {
            for (slot, instance) in instances.iter().enumerate() {
                if (instance.ids[3] != 0) == active {
                    physical_to_slot
                        .push(u32::try_from(slot).map_err(|_| RenderError::SceneCapacity)?);
                }
            }
        }
        let active_count = u32::try_from(
            instances
                .iter()
                .filter(|instance| instance.ids[3] != 0)
                .count(),
        )
        .map_err(|_| RenderError::SceneCapacity)?;
        let mut slot_to_physical = vec![0; instances.len()];
        let mut bytes = Vec::with_capacity(instances.len() * abi::INSTANCE_BYTES);
        for (physical, &slot) in physical_to_slot.iter().enumerate() {
            slot_to_physical[slot as usize] =
                u32::try_from(physical).map_err(|_| RenderError::SceneCapacity)?;
            bytes.extend_from_slice(&pack_synthetic_instance(
                &instances[slot as usize],
                material_count,
                slot,
            )?);
        }
        debug_assert_eq!(physical_to_slot.len(), count as usize);
        Ok(Self {
            bytes,
            physical_to_slot,
            slot_to_physical,
            active_count,
        })
    }
}

pub(super) struct InstanceUpload {
    buffer: wgpu::Buffer,
    packed: Vec<u8>,
    staged: Vec<u8>,
    staged_shadow: Vec<bool>,
    staged_two_sided: Vec<bool>,
    valid_slots: Vec<u32>,
    dirty_physical: Vec<u32>,
    runs: Vec<UploadRun>,
    material_count: u32,
    instance_count: u32,
    physical_to_slot: Vec<u32>,
    slot_to_physical: Vec<u32>,
    active_slots: Vec<bool>,
    active_count: u32,
    shadow_clamped: Vec<bool>,
    active_shadow_clamps: usize,
    two_sided: Vec<bool>,
    active_two_sided: usize,
}

impl InstanceUpload {
    pub(super) fn new(
        buffer: wgpu::Buffer,
        initial: PackedInstances,
        instances: &[InstanceRecord],
        material_count: u32,
        instance_count: u32,
    ) -> Self {
        let count = instance_count as usize;
        let mut active_slots = vec![false; count];
        for &slot in &initial.physical_to_slot[..initial.active_count as usize] {
            active_slots[slot as usize] = true;
        }
        let two_sided: Vec<bool> = instances
            .iter()
            .map(|instance| instance.ids[3] != 0 && needs_two_sided(instance))
            .collect();
        let active_two_sided = two_sided.iter().filter(|&&value| value).count();
        Self {
            buffer,
            staged: Vec::with_capacity(initial.bytes.len()),
            staged_shadow: Vec::with_capacity(count),
            staged_two_sided: Vec::with_capacity(count),
            packed: initial.bytes,
            valid_slots: Vec::with_capacity(count),
            dirty_physical: Vec::with_capacity(count * 3),
            runs: Vec::with_capacity(MAX_PARTIAL_WRITES),
            material_count,
            instance_count,
            physical_to_slot: initial.physical_to_slot,
            slot_to_physical: initial.slot_to_physical,
            active_slots,
            active_count: initial.active_count,
            shadow_clamped: vec![false; count],
            active_shadow_clamps: 0,
            two_sided,
            active_two_sided,
        }
    }

    pub(super) fn seed_shadow_clamps(
        &mut self,
        instances: &[InstanceRecord],
        clamped: impl Fn(&InstanceRecord) -> bool,
    ) {
        self.active_shadow_clamps = 0;
        for (flag, instance) in self.shadow_clamped.iter_mut().zip(instances) {
            *flag = clamped(instance);
            self.active_shadow_clamps += usize::from(*flag);
        }
    }

    pub(super) fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub(super) fn active_count(&self) -> u32 {
        self.active_count
    }

    pub(super) fn two_sided_count(&self) -> usize {
        self.active_two_sided
    }

    /// Physical row order and its packed rows, for layout witnesses.
    #[cfg(test)]
    pub(super) fn physical_rows(&self) -> (&[u32], &[u8]) {
        (&self.physical_to_slot, &self.packed)
    }

    pub(super) fn replace(
        &mut self,
        queue: &wgpu::Queue,
        instances: &[InstanceRecord],
        changed_slots: &[u32],
        shadow_depth_clamped: impl Fn(&InstanceRecord) -> bool,
    ) -> Result<InstanceUploadReport, RenderError> {
        if instances.len() != self.instance_count as usize {
            return Err(RenderError::SceneCapacity);
        }
        self.staged.clear();
        self.staged_shadow.clear();
        self.staged_two_sided.clear();
        self.valid_slots.clear();
        self.dirty_physical.clear();
        let mut previous_slot = None;
        let mut dropped_invalid = 0;
        for &slot in changed_slots {
            if slot >= self.instance_count || previous_slot.is_some_and(|old| slot <= old) {
                return Err(RenderError::InvalidInstanceUpdate);
            }
            previous_slot = Some(slot);
            let index = slot as usize;
            let Ok(bytes) = pack_synthetic_instance(&instances[index], self.material_count, slot)
            else {
                dropped_invalid += 1;
                continue;
            };
            let start = self.slot_to_physical[index] as usize * abi::INSTANCE_BYTES;
            if self.packed[start..start + abi::INSTANCE_BYTES] != bytes {
                let clamped = shadow_depth_clamped(&instances[index]);
                self.staged.extend_from_slice(&bytes);
                self.staged_shadow.push(clamped);
                self.staged_two_sided
                    .push(instances[index].ids[3] != 0 && needs_two_sided(&instances[index]));
                self.valid_slots.push(slot);
            }
        }
        let changed_rows = self.valid_slots.len();
        if changed_rows == 0 {
            return Ok(InstanceUploadReport {
                changed_rows,
                dropped_invalid,
                clamped_shadow_casters: self.active_shadow_clamps,
                writes: 0,
                bytes: 0,
            });
        }
        let (rows, remainder) = self.staged.as_chunks::<{ abi::INSTANCE_BYTES }>();
        debug_assert_eq!(remainder, []);
        for (&slot, row) in self.valid_slots.iter().zip(rows) {
            let logical = slot as usize;
            let new_active = instances[logical].ids[3] != 0;
            let old_physical = self.slot_to_physical[logical];
            if new_active != self.active_slots[logical] {
                let boundary = if new_active {
                    let boundary = self.active_count;
                    self.active_count += 1;
                    boundary
                } else {
                    self.active_count -= 1;
                    self.active_count
                };
                swap_physical(
                    &mut self.packed,
                    &mut self.physical_to_slot,
                    &mut self.slot_to_physical,
                    &mut self.dirty_physical,
                    old_physical,
                    boundary,
                );
            }
            let physical = self.slot_to_physical[logical];
            let start = physical as usize * abi::INSTANCE_BYTES;
            self.packed[start..start + abi::INSTANCE_BYTES].copy_from_slice(row);
            self.active_slots[logical] = new_active;
            self.dirty_physical.push(physical);
        }
        // Stable all-active scenes preserve logical order. Avoid sorting the
        // common sparse-motion path; churn can disorder physical rows.
        if self
            .dirty_physical
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            self.dirty_physical.sort_unstable();
            self.dirty_physical.dedup();
        }
        let too_many_runs = build_runs(&self.dirty_physical, &mut self.runs);
        let full = too_many_runs || self.dirty_physical.len() > instances.len() / 2;
        if full {
            queue.write_buffer(&self.buffer, 0, &self.packed);
        } else {
            for run in &self.runs {
                let start = run.destination as usize * abi::INSTANCE_BYTES;
                let end = start + (run.source.end - run.source.start) * abi::INSTANCE_BYTES;
                queue.write_buffer(
                    &self.buffer,
                    u64::from(run.destination) * abi::INSTANCE_BYTES as u64,
                    &self.packed[start..end],
                );
            }
        }
        for (&slot, &clamped) in self.valid_slots.iter().zip(&self.staged_shadow) {
            let index = slot as usize;
            self.active_shadow_clamps -= usize::from(self.shadow_clamped[index]);
            self.active_shadow_clamps += usize::from(clamped);
            self.shadow_clamped[index] = clamped;
        }
        for (&slot, &two_sided) in self.valid_slots.iter().zip(&self.staged_two_sided) {
            let index = slot as usize;
            self.active_two_sided -= usize::from(self.two_sided[index]);
            self.active_two_sided += usize::from(two_sided);
            self.two_sided[index] = two_sided;
        }
        Ok(InstanceUploadReport {
            changed_rows,
            dropped_invalid,
            clamped_shadow_casters: self.active_shadow_clamps,
            writes: if full { 1 } else { self.runs.len() },
            bytes: if full {
                self.packed.len()
            } else {
                self.dirty_physical.len() * abi::INSTANCE_BYTES
            },
        })
    }
}

fn needs_two_sided(instance: &InstanceRecord) -> bool {
    let m = &instance.current;
    let (a, b, c) = (
        [f64::from(m[0][0]), f64::from(m[0][1]), f64::from(m[0][2])],
        [f64::from(m[1][0]), f64::from(m[1][1]), f64::from(m[1][2])],
        [f64::from(m[2][0]), f64::from(m[2][1]), f64::from(m[2][2])],
    );
    let determinant = a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0]);
    determinant <= 1e-12
}

fn swap_physical(
    packed: &mut [u8],
    physical_to_slot: &mut [u32],
    slot_to_physical: &mut [u32],
    dirty_physical: &mut Vec<u32>,
    first: u32,
    second: u32,
) {
    if first == second {
        return;
    }
    let a = first as usize;
    let b = second as usize;
    let slot_a = physical_to_slot[a];
    let slot_b = physical_to_slot[b];
    physical_to_slot.swap(a, b);
    slot_to_physical[slot_a as usize] = second;
    slot_to_physical[slot_b as usize] = first;
    let start_a = a * abi::INSTANCE_BYTES;
    let start_b = b * abi::INSTANCE_BYTES;
    for byte in 0..abi::INSTANCE_BYTES {
        packed.swap(start_a + byte, start_b + byte);
    }
    dirty_physical.extend([first, second]);
}

fn build_runs(actual: &[u32], runs: &mut Vec<UploadRun>) -> bool {
    runs.clear();
    let mut first = 0;
    let mut too_many = false;
    for index in 1..=actual.len() {
        if index == actual.len() || actual[index] != actual[index - 1] + 1 {
            if runs.len() < MAX_PARTIAL_WRITES {
                runs.push(UploadRun {
                    source: first..index,
                    destination: actual[first],
                });
            } else {
                too_many = true;
            }
            first = index;
        }
    }
    too_many
}

pub(super) fn pack_synthetic_instance(
    instance: &InstanceRecord,
    material_count: u32,
    logical_slot: u32,
) -> Result<[u8; abi::INSTANCE_BYTES], RenderError> {
    if instance.ids[0] != logical_slot {
        return Err(RenderError::InvalidInstanceIdentity);
    }
    if instance.ids[2] != 0 {
        return Err(RenderError::UnsupportedSyntheticMaterial);
    }
    // The bounded cube evaluator needs finite nondegenerate normal axes.
    // General affine/singular-transform policy belongs to cooked geometry.
    if instance.normal.iter().any(|column| {
        let length2 = column[..3]
            .iter()
            .map(|v| f64::from(*v).powi(2))
            .sum::<f64>();
        !(1e-12..=1e12).contains(&length2)
    }) || instance
        .current
        .iter()
        .flatten()
        .any(|v| !v.is_finite() || v.abs() > 1e6)
        || instance.current[0][3] != 0.0
        || instance.current[1][3] != 0.0
        || instance.current[2][3] != 0.0
        || instance.current[3][3].to_bits() != 1.0_f32.to_bits()
    {
        return Err(RenderError::InvalidLighting);
    }
    Ok(instance.pack(material_count)?)
}

#[cfg(test)]
mod tests;
