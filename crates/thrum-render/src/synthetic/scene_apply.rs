//! Map extracted physical world slots into the synthetic GPU instance table.

use crate::extract::RenderColumns;

use super::SyntheticScene;

/// Presentation rows accepted or dropped while updating the GPU source table.
#[derive(Clone, Copy, Debug, Default)]
pub struct SceneApplyReport {
    /// Rows whose current and previous positions differ.
    pub moving: usize,
    /// Rows that cannot be represented by this prepared scene.
    pub dropped_invalid: usize,
}

impl SyntheticScene {
    /// Apply a committed render extraction. Physical slots address the table;
    /// stable identities never serve as GPU array indices. The changed-slot
    /// buffer is reused by the caller for a bounded upload.
    /// # Panics
    /// Only if the prepared scene contains more physical slots than a `u32`
    /// index can represent.
    #[inline]
    pub fn apply_render_columns(
        &mut self,
        frame: &RenderColumns,
        changed_slots: &mut Vec<u32>,
    ) -> SceneApplyReport {
        changed_slots.clear();
        let mut report = SceneApplyReport::default();
        let material_count = self.materials.len();
        // The extraction worker may discard frames. Retire against the last
        // scene actually applied to the GPU, never its previous extraction.
        for (slot, instance) in self.instances.iter_mut().enumerate() {
            if instance.ids[3] != 0 && !frame.contains_slot(slot) {
                instance.ids[3] = 0;
                changed_slots.push(u32::try_from(slot).expect("bounded synthetic scene slots"));
            }
        }
        for row in 0..frame.len() {
            let Ok(index) = usize::try_from(frame.slots()[row]) else {
                report.dropped_invalid += 1;
                continue;
            };
            let Some(instance) = self.instances.get_mut(index) else {
                report.dropped_invalid += 1;
                continue;
            };
            let Some(material) = u32::try_from(frame.visuals()[row]).ok().filter(|material| {
                usize::try_from(*material).is_ok_and(|row| row < material_count)
            }) else {
                report.dropped_invalid += 1;
                continue;
            };
            let world_current = frame.current()[row];
            let previous = if frame.history_valid()[row] {
                frame.previous()[row]
            } else {
                world_current
            };
            // Zero marks an inactive GPU row. A nonzero wrapped token also
            // distinguishes consecutive physical-slot generations.
            let Some(generation) = frame
                .generation_at_slot(index)
                .filter(|generation| *generation != 0)
            else {
                report.dropped_invalid += 1;
                continue;
            };
            let source_generation = u32::try_from((generation - 1) % u64::from(u32::MAX) + 1)
                .expect("nonzero wrapped generation fits u32");
            if instance.current[3][..3] != world_current
                || instance.previous[3][..3] != previous
                || instance.ids[1] != material
                || instance.ids[3] != source_generation
            {
                instance.current[3][..3].copy_from_slice(&world_current);
                instance.previous[3][..3].copy_from_slice(&previous);
                instance.ids[1] = material;
                instance.ids[3] = source_generation;
                changed_slots.push(u32::try_from(index).expect("bounded scene slot fits u32"));
            }
            report.moving += usize::from((world_current[0] - previous[0]).abs() > 1e-6);
        }
        changed_slots.sort_unstable();
        report
    }
}
