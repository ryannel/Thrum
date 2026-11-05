//! Reused instance packing and sparse queue writes for the diagnostic.

use std::ops::Range;

use crate::abi::{self, InstanceRecord};

use super::{InstanceUploadReport, RenderError};

const MAX_PARTIAL_WRITES: usize = 64;

struct UploadRun {
    source: Range<usize>,
    destination: u32,
}

pub(super) struct InstanceUpload {
    buffer: wgpu::Buffer,
    packed: Vec<u8>,
    staged: Vec<u8>,
    staged_shadow: Vec<bool>,
    actual: Vec<u32>,
    runs: Vec<UploadRun>,
    material_count: u32,
    instance_count: u32,
    shadow_clamped: Vec<bool>,
    active_shadow_clamps: usize,
}

impl InstanceUpload {
    pub(super) fn new(
        buffer: wgpu::Buffer,
        packed: Vec<u8>,
        material_count: u32,
        instance_count: u32,
    ) -> Self {
        let count = instance_count as usize;
        Self {
            buffer,
            staged: Vec::with_capacity(packed.len()),
            staged_shadow: Vec::with_capacity(count),
            packed,
            actual: Vec::with_capacity(count),
            runs: Vec::with_capacity(MAX_PARTIAL_WRITES),
            material_count,
            instance_count,
            shadow_clamped: vec![false; count],
            active_shadow_clamps: 0,
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
        self.actual.clear();
        let mut previous_slot = None;
        let mut dropped_invalid = 0;
        for &slot in changed_slots {
            if slot >= self.instance_count || previous_slot.is_some_and(|old| slot <= old) {
                return Err(RenderError::InvalidInstanceUpdate);
            }
            previous_slot = Some(slot);
            let index = slot as usize;
            let Ok(bytes) = pack_synthetic_instance(&instances[index], self.material_count) else {
                dropped_invalid += 1;
                continue;
            };
            let start = index * abi::INSTANCE_BYTES;
            if self.packed[start..start + abi::INSTANCE_BYTES] != bytes {
                let clamped = shadow_depth_clamped(&instances[index]);
                self.staged.extend_from_slice(&bytes);
                self.staged_shadow.push(clamped);
                self.actual.push(slot);
            }
        }
        let changed_rows = self.actual.len();
        if changed_rows == 0 {
            return Ok(InstanceUploadReport {
                changed_rows,
                dropped_invalid,
                clamped_shadow_casters: self.active_shadow_clamps,
                writes: 0,
                bytes: 0,
            });
        }
        let too_many_runs = build_runs(&self.actual, &mut self.runs);
        let full = too_many_runs || changed_rows > instances.len() / 2;
        // Slot ordering is valid and malformed presentation rows were dropped.
        // No GPU write or baseline mutation occurs on a rejected update.
        if full {
            apply_to_baseline(&mut self.packed, &self.staged, &self.actual);
            queue.write_buffer(&self.buffer, 0, &self.packed);
        } else {
            for run in &self.runs {
                let start = run.source.start * abi::INSTANCE_BYTES;
                let end = run.source.end * abi::INSTANCE_BYTES;
                queue.write_buffer(
                    &self.buffer,
                    u64::from(run.destination) * abi::INSTANCE_BYTES as u64,
                    &self.staged[start..end],
                );
            }
            apply_to_baseline(&mut self.packed, &self.staged, &self.actual);
        }
        for (&slot, &clamped) in self.actual.iter().zip(&self.staged_shadow) {
            let index = slot as usize;
            self.active_shadow_clamps -= usize::from(self.shadow_clamped[index]);
            self.active_shadow_clamps += usize::from(clamped);
            self.shadow_clamped[index] = clamped;
        }
        Ok(InstanceUploadReport {
            changed_rows,
            dropped_invalid,
            clamped_shadow_casters: self.active_shadow_clamps,
            writes: if full { 1 } else { self.runs.len() },
            bytes: if full {
                self.packed.len()
            } else {
                self.staged.len()
            },
        })
    }
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

fn apply_to_baseline(packed: &mut [u8], staged: &[u8], actual: &[u32]) {
    let (rows, remainder) = staged.as_chunks::<{ abi::INSTANCE_BYTES }>();
    debug_assert_eq!(remainder, []);
    for (&slot, row) in actual.iter().zip(rows) {
        let start = slot as usize * abi::INSTANCE_BYTES;
        packed[start..start + abi::INSTANCE_BYTES].copy_from_slice(row);
    }
}

pub(super) fn pack_synthetic_instance(
    instance: &InstanceRecord,
    material_count: u32,
) -> Result<[u8; abi::INSTANCE_BYTES], RenderError> {
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
