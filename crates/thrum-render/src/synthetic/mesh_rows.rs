//! CPU preparation and the shared publication handshake for live mesh rows.

use std::sync::{Arc, Mutex};

use super::SyntheticMesh;
use super::mesh_arena::{ArenaLimits, check};
mod host;
mod live;
pub use super::mesh_arena::ArenaError as MeshRowError;
pub(super) use live::LiveRows;

/// Maximum geometry bytes written by the row route in one frame (1 MiB).
pub const ROW_WRITE_BYTES: u64 = 1 << 20;

/// Host observations of bounded row writes; timings are contended, not qualified.
#[derive(Clone, Debug, Default)]
pub struct MeshRowReport {
    /// Publication whose row switch most recently committed on this reader.
    pub generation: Option<u64>,
    /// Most recent recording's geometry bytes, including discarded recordings.
    pub last_write_bytes: u64,
    /// Largest recording, bounded by `ROW_WRITE_BYTES`.
    pub max_write_bytes: u64,
    /// Geometry bytes whose frames were submitted.
    pub submitted_bytes: u64,
    /// Render-thread recording time for frames with geometry writes.
    pub write_times: Vec<std::time::Duration>,
    /// Render-thread recording time for frames without geometry writes.
    pub steady_times: Vec<std::time::Duration>,
    /// Highest allocated vertex and index element endpoints.
    pub high_water: [u32; 2],
    /// Payload capacity reserved in the two arena buffers.
    pub reserved_bytes: u64,
}

/// Validated, packed geometry, prepared before the render thread takes it.
#[derive(Clone, Debug)]
pub struct MeshRowGeometry {
    pub(super) mesh: Arc<SyntheticMesh>,
    pub(super) vertices: Arc<[u8]>,
    pub(super) indices: Arc<[u8]>,
    pub(super) bounds: [[f32; 3]; 2],
    pub(super) cast_width: f32,
}

impl MeshRowGeometry {
    /// Apply the arena's admission and pack one row on the caller's thread.
    /// # Errors
    /// Returns a named admission error without changing any row.
    pub fn new(mesh: SyntheticMesh, max_binding: u64) -> Result<Self, MeshRowError> {
        let bounds = check(&mesh.vertices, &mesh.indices)?;
        let limits = ArenaLimits::for_binding(max_binding);
        if mesh.vertices.len() as u64 > u64::from(limits.vertices)
            || mesh.indices.len() as u64 > u64::from(limits.indices)
        {
            return Err(MeshRowError::Capacity);
        }
        let vertices = bytemuck::cast_slice(&mesh.vertices).to_vec().into();
        let indices = bytemuck::cast_slice(&mesh.indices).to_vec().into();
        Ok(Self {
            cast_width: super::shadow::cast_width::measure(&mesh.vertices, &mesh.indices, bounds),
            mesh: Arc::new(mesh),
            vertices,
            indices,
            bounds,
        })
    }

    /// Geometry retained for initial preparation and device reconstruction.
    pub fn mesh(&self) -> &SyntheticMesh {
        &self.mesh
    }
}

#[cfg(test)]
mod tests;

/// A replacement's preparation as observed by the publication owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeshRowStatus {
    /// The host has not submitted every bounded write yet.
    Writing,
    /// All writes were submitted; the publication owner may offer the switch.
    Ready,
    /// The switch frame was submitted.
    Committed,
    /// Admission refused the whole replacement.
    Refused {
        /// Index in the scene's authored mesh selection.
        row: usize,
        /// Arena admission failure.
        reason: MeshRowError,
    },
}

#[derive(Debug)]
pub(super) struct Publication {
    pub(super) serial: u64,
    pub(super) generation: u64,
    pub(super) rows: Vec<(usize, MeshRowGeometry)>,
    pub(super) status: MeshRowStatus,
    pub(super) offered: bool,
}

#[derive(Debug)]
pub(super) struct State {
    pub(super) next_publication: u64,
    pub(super) report: MeshRowReport,
    pub(super) binding_limit: Option<u64>,
    pub(super) current: Vec<Arc<SyntheticMesh>>,
    pub(super) pending: Option<Publication>,
}

/// Generic row reader connected by the game to a publication owner.
/// The frame host alone writes, switches and retires arena geometry.
#[derive(Clone, Debug)]
pub struct MeshRows(pub(super) Arc<Mutex<State>>);

impl PartialEq for MeshRows {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl MeshRows {
    /// Retain the prepared geometry's identity across device reconstruction.
    pub fn from_geometry(meshes: &[MeshRowGeometry]) -> Self {
        Self(Arc::new(Mutex::new(State {
            next_publication: 0,
            report: MeshRowReport::default(),
            binding_limit: None,
            current: meshes
                .iter()
                .map(|geometry| Arc::clone(&geometry.mesh))
                .collect(),
            pending: None,
        })))
    }

    /// Start with the same admitted geometry used by the scene's selection.
    pub fn new(meshes: Vec<SyntheticMesh>) -> Self {
        Self(Arc::new(Mutex::new(State {
            next_publication: 0,
            report: MeshRowReport::default(),
            binding_limit: None,
            current: meshes.into_iter().map(Arc::new).collect(),
            pending: None,
        })))
    }

    /// Reserve a smaller arena than the device maximum for a bounded scene.
    /// Set before preparation; initial geometry is checked against the same limit.
    #[must_use]
    pub fn with_binding_limit(self, bytes: u64) -> Self {
        if let Ok(mut state) = self.0.lock() {
            state.binding_limit = Some(bytes);
        }
        self
    }

    /// Snapshot of the host's row-write observations.
    pub fn report(&self) -> MeshRowReport {
        self.0
            .lock()
            .map_or_else(|_| MeshRowReport::default(), |state| state.report.clone())
    }

    /// The most recently submitted publication, without copying the write history.
    #[must_use]
    pub fn generation(&self) -> Option<u64> {
        self.0.lock().ok()?.report.generation
    }

    /// Queue geometry prepared by the publication owner. An offered publication
    /// is protected until its switch commits; the owner must defer newer work.
    /// # Errors
    /// Refuses unknown or repeated row indices and a protected publication.
    pub fn prepare(
        &self,
        generation: u64,
        rows: Vec<(usize, MeshRowGeometry)>,
    ) -> Result<(), MeshRowError> {
        let mut state = self.0.lock().map_err(|_| MeshRowError::UnknownMesh)?;
        if state
            .pending
            .as_ref()
            .is_some_and(|p| p.offered && p.status != MeshRowStatus::Committed)
        {
            return Err(MeshRowError::Capacity);
        }
        let mut seen = std::collections::BTreeSet::new();
        if rows
            .iter()
            .any(|(row, _)| *row >= state.current.len() || !seen.insert(*row))
        {
            return Err(MeshRowError::UnknownMesh);
        }
        let serial = state
            .next_publication
            .checked_add(1)
            .ok_or(MeshRowError::Capacity)?;
        state.next_publication = serial;
        state.pending = Some(Publication {
            serial,
            generation,
            rows,
            status: MeshRowStatus::Writing,
            offered: false,
        });
        Ok(())
    }

    /// Observe preparation for exactly this publication, never a prior one.
    pub fn status(&self, generation: u64) -> Option<MeshRowStatus> {
        let state = self.0.lock().ok()?;
        state
            .pending
            .as_ref()
            .filter(|p| p.generation == generation)
            .map(|p| p.status)
    }

    /// Fix the generation every reader will show in the next recorded frame.
    /// Returns false until the host has submitted every row write.
    pub fn offer(&self, generation: u64) -> bool {
        let Ok(mut state) = self.0.lock() else {
            return false;
        };
        let Some(p) = state
            .pending
            .as_mut()
            .filter(|p| p.generation == generation && p.status == MeshRowStatus::Ready)
        else {
            return false;
        };
        p.offered = true;
        true
    }

    /// Abandon an unoffered publication when any reader refuses it.
    pub fn cancel(&self, generation: u64) {
        if let Ok(mut state) = self.0.lock()
            && state
                .pending
                .as_ref()
                .is_some_and(|p| p.generation == generation && !p.offered)
        {
            state.pending = None;
        }
    }

    /// Drop device-specific preparation after the host retires that device.
    /// Committed CPU geometry stays available for reconstruction.
    pub fn invalidate_device(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.pending = None;
        }
    }
}
