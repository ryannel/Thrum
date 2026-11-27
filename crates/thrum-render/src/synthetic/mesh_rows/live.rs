//! Host-owned allocation, bounded recording and submit-only retirement.

use std::sync::Arc;

use super::{MeshRowError, MeshRowGeometry, MeshRowStatus, MeshRows, ROW_WRITE_BYTES};
use crate::synthetic::mesh_arena::{ArenaLimits, MeshArena, MeshId, MeshRow};
use crate::synthetic::raytrace::MeshStructure;

#[derive(Default)]
struct Space {
    end: u32,
    free: Vec<std::ops::Range<u32>>,
}

impl Space {
    fn take(&mut self, count: u32, limit: u32) -> Result<u32, MeshRowError> {
        if let Some(index) = self.free.iter().position(|r| r.end - r.start >= count) {
            let start = self.free[index].start;
            self.free[index].start += count;
            if self.free[index].is_empty() {
                self.free.remove(index);
            }
            return Ok(start);
        }
        let start = self.end;
        self.end = start
            .checked_add(count)
            .filter(|end| *end <= limit)
            .ok_or(MeshRowError::Capacity)?;
        Ok(start)
    }

    fn release(&mut self, start: u32, count: u32) {
        self.free.push(start..start + count);
        self.free.sort_unstable_by_key(|r| r.start);
        let mut merged: Vec<std::ops::Range<u32>> = Vec::new();
        for range in self.free.drain(..) {
            if let Some(last) = merged.last_mut()
                && last.end == range.start
            {
                last.end = range.end;
            } else {
                merged.push(range);
            }
        }
        self.free = merged;
    }
}

struct Replacement {
    id: MeshId,
    geometry: MeshRowGeometry,
    row: MeshRow,
    old: Option<MeshRow>,
    ray: Option<MeshStructure>,
    build_ray: bool,
}

pub(in crate::synthetic) struct LiveRows {
    current: Vec<Arc<crate::synthetic::SyntheticMesh>>,
    updates: MeshRows,
    vertices: Space,
    indices: Space,
    limits: ArenaLimits,
    generation: Option<u64>,
    serial: Option<u64>,
    rows: Vec<Replacement>,
    cursor: u64,
    recorded: u64,
    switching: bool,
}

impl LiveRows {
    pub(in crate::synthetic) fn same_reader(&self, other: &Self) -> bool {
        self.updates == other.updates
    }

    pub(in crate::synthetic) fn same_geometry(&self, other: &Self) -> bool {
        self.same_reader(other)
            && self.current.len() == other.current.len()
            && self
                .current
                .iter()
                .zip(&other.current)
                .all(|(a, b)| Arc::ptr_eq(a, b))
    }

    pub(in crate::synthetic) fn observe(&self, elapsed: std::time::Duration) {
        if let Ok(mut state) = self.updates.0.lock() {
            let report = &mut state.report;
            report.last_write_bytes = self.recorded;
            report.max_write_bytes = report.max_write_bytes.max(self.recorded);
            report.high_water = [self.vertices.end, self.indices.end];
            report.reserved_bytes =
                u64::from(self.limits.vertices) * 48 + u64::from(self.limits.indices) * 4;
            let times = if self.recorded == 0 {
                &mut report.steady_times
            } else {
                &mut report.write_times
            };
            if times.len() < 1024 {
                times.push(elapsed);
            }
        }
    }
    pub(in crate::synthetic) fn new(updates: MeshRows, arena: &mut MeshArena) -> Self {
        let (vertices, indices, limits) = arena.extents();
        Self {
            current: arena.take_geometry(),
            updates,
            vertices: Space {
                end: vertices,
                ..Space::default()
            },
            indices: Space {
                end: indices,
                ..Space::default()
            },
            limits,
            generation: None,
            serial: None,
            rows: Vec::new(),
            cursor: 0,
            recorded: 0,
            switching: false,
        }
    }

    fn retire(&mut self, row: MeshRow) {
        self.vertices.release(row.base_vertex, row.vertex_count);
        self.indices.release(row.first_index, row.index_count);
    }

    fn abandon(&mut self) {
        for replacement in std::mem::take(&mut self.rows) {
            self.retire(replacement.row);
        }
        self.cursor = 0;
        self.recorded = 0;
    }

    fn allocate(
        &mut self,
        index: usize,
        geometry: MeshRowGeometry,
    ) -> Result<Replacement, MeshRowError> {
        let id = MeshId::selected(index)?;
        let vertex_count =
            u32::try_from(geometry.mesh.vertices.len()).map_err(|_| MeshRowError::Capacity)?;
        let index_count =
            u32::try_from(geometry.mesh.indices.len()).map_err(|_| MeshRowError::Capacity)?;
        let base_vertex = self.vertices.take(vertex_count, self.limits.vertices)?;
        let first_index = match self.indices.take(index_count, self.limits.indices) {
            Ok(index) => index,
            Err(error) => {
                self.vertices.release(base_vertex, vertex_count);
                return Err(error);
            }
        };
        Ok(Replacement {
            id,
            row: MeshRow {
                first_index,
                index_count,
                base_vertex,
                vertex_count,
                bounds: geometry.bounds,
                cast_width: geometry.cast_width,
            },
            geometry,
            old: None,
            ray: None,
            build_ray: false,
        })
    }

    /// Replay discarded writes. Cursor and retirement change only in submitted.
    pub(in crate::synthetic) fn record(
        &mut self,
        arena: &mut MeshArena,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<bool, MeshRowError> {
        self.recorded = 0;
        self.switching = false;
        for row in &mut self.rows {
            row.build_ray = false;
        }
        let updates = self.updates.clone();
        let mut state = updates.0.lock().map_err(|_| MeshRowError::UnknownMesh)?;
        let Some(publication) = state.pending.as_mut() else {
            self.abandon();
            self.generation = None;
            return Ok(false);
        };
        if publication.status == MeshRowStatus::Committed {
            return Ok(false);
        }
        if self.serial != Some(publication.serial) {
            self.abandon();
            self.serial = Some(publication.serial);
            self.generation = Some(publication.generation);
            for (index, geometry) in &publication.rows {
                if self
                    .current
                    .get(*index)
                    .is_some_and(|current| Arc::ptr_eq(current, &geometry.mesh))
                {
                    continue;
                }
                match self.allocate(*index, geometry.clone()) {
                    Ok(row) if arena.row(row.id).is_some() => self.rows.push(row),
                    Ok(row) => {
                        self.retire(row.row);
                        self.abandon();
                        publication.status = MeshRowStatus::Refused {
                            row: *index,
                            reason: MeshRowError::UnknownMesh,
                        };
                        return Ok(false);
                    }
                    Err(error) => {
                        self.abandon();
                        publication.status = MeshRowStatus::Refused {
                            row: *index,
                            reason: error,
                        };
                        return Ok(false);
                    }
                }
            }
        }
        if matches!(publication.status, MeshRowStatus::Refused { .. }) {
            return Ok(false);
        }
        if publication.offered {
            for replacement in &mut self.rows {
                let old = arena.switch(replacement.id, replacement.row)?;
                replacement.old.get_or_insert(old);
                if let Some(structure) = replacement.ray.take() {
                    arena.install_mesh_structure(replacement.id, structure);
                }
            }
            self.switching = true;
            return Ok(true);
        }
        let buffers = arena.buffers().ok_or(MeshRowError::Capacity)?;
        let mut offset = 0;
        for replacement in &mut self.rows {
            for (bytes, buffer, destination) in [
                (
                    &replacement.geometry.vertices,
                    &buffers.vertices,
                    u64::from(replacement.row.base_vertex) * 48,
                ),
                (
                    &replacement.geometry.indices,
                    &buffers.indices,
                    u64::from(replacement.row.first_index) * 4,
                ),
            ] {
                let start = self.cursor.saturating_sub(offset).min(bytes.len() as u64);
                let count = (bytes.len() as u64 - start).min(ROW_WRITE_BYTES - self.recorded);
                if count != 0 {
                    queue.write_buffer(
                        buffer,
                        destination + start,
                        &bytes[usize::try_from(start).map_err(|_| MeshRowError::Capacity)?
                            ..usize::try_from(start + count)
                                .map_err(|_| MeshRowError::Capacity)?],
                    );
                    self.recorded += count;
                }
                offset += bytes.len() as u64;
            }
            if arena.ray_demand() && self.cursor < offset && self.cursor + self.recorded >= offset {
                if replacement.ray.is_none() {
                    replacement.ray = Some(
                        MeshStructure::new(device, replacement.row)
                            .map_err(|_| MeshRowError::Capacity)?,
                    );
                }
                replacement.build_ray = true;
            }
        }
        Ok(false)
    }

    pub(in crate::synthetic) fn pending_ray_builds(&self) -> impl Iterator<Item = &MeshStructure> {
        self.rows
            .iter()
            .filter(|row| row.build_ray)
            .filter_map(|row| row.ray.as_ref())
    }

    pub(in crate::synthetic) fn submitted(&mut self) {
        let updates = self.updates.clone();
        let Ok(mut state) = updates.0.lock() else {
            return;
        };
        if state
            .pending
            .as_ref()
            .is_some_and(|p| Some(p.serial) == self.serial)
        {
            state.report.submitted_bytes += self.recorded;
        }
        let Some(publication) = state
            .pending
            .as_mut()
            .filter(|p| Some(p.serial) == self.serial)
        else {
            return;
        };
        if self.switching {
            publication.status = MeshRowStatus::Committed;
            let changes = publication.rows.clone();
            state.report.generation = self.generation;
            for row in std::mem::take(&mut self.rows) {
                if let Some(old) = row.old {
                    self.retire(old);
                }
            }
            for (index, geometry) in changes {
                state.current[index] = Arc::clone(&geometry.mesh);
                self.current[index] = Arc::clone(&geometry.mesh);
            }
        } else if publication.status == MeshRowStatus::Writing {
            self.cursor += std::mem::take(&mut self.recorded);
            let total: u64 = self
                .rows
                .iter()
                .map(|r| (r.geometry.vertices.len() + r.geometry.indices.len()) as u64)
                .sum();
            if self.cursor == total {
                publication.status = MeshRowStatus::Ready;
            }
        }
        self.switching = false;
    }
}
