//! Row recording and ownership transfer at the frame host's publication barrier.

use super::super::{PreparedSynthetic, RenderError, camera_culling};

impl PreparedSynthetic {
    pub(crate) fn record_mesh_rows(&mut self, queue: &wgpu::Queue) -> Result<bool, RenderError> {
        let Some(rows) = self.row_updates.as_mut() else {
            return Ok(false);
        };
        let start = crate::runtime::Instant::now();
        let switched = rows
            .record(&mut self.mesh_arena, self.capabilities.device(), queue)
            .map_err(RenderError::Arena)?;
        rows.observe(start.elapsed());
        if switched {
            self.refresh_mesh_rows(queue)?;
        }
        Ok(switched)
    }

    fn refresh_mesh_rows(&mut self, queue: &wgpu::Queue) -> Result<(), RenderError> {
        if let Some(route) = self.bucket_route.as_mut() {
            self.caster_boxes = Some(route.refresh_rows(&self.mesh_arena)?);
        }
        let records = camera_culling::mesh_records(&self.mesh_arena)?;
        if let Some(visibility) = self.visibility.as_ref() {
            visibility.update_meshes(queue, &records);
        }
        if let Some((culling, _)) = self.caster_culling.as_ref() {
            culling.update_meshes(queue, &records);
        }
        for slot in &self.secondary {
            slot.update_caster_meshes(queue, &records);
        }
        Ok(())
    }

    /// At the host's drained publication barrier, retain the arena together with
    /// the write cursor, allocations and offered switch that belong to it.
    pub(crate) fn inherit_mesh_rows(
        &mut self,
        previous: &mut Self,
        queue: &wgpu::Queue,
    ) -> Result<bool, RenderError> {
        if self.key.device_generation != previous.key.device_generation
            || self.mesh_arena.ray_demand() != previous.mesh_arena.ray_demand()
            || !self
                .row_updates
                .as_ref()
                .zip(previous.row_updates.as_ref())
                .is_some_and(|(next, old)| next.same_reader(old))
        {
            return Ok(false);
        }
        self.swap_mesh_rows(previous);
        let refreshed = (|| {
            self.refresh_mesh_rows(queue)?;
            self.history.rebind_mesh_vertices(
                self.capabilities.device(),
                self.instance_upload.buffer(),
                &self
                    .mesh_arena
                    .buffers()
                    .ok_or(RenderError::MissingMeshBuffers)?
                    .vertices,
            );
            Ok(())
        })();
        if let Err(error) = refreshed {
            self.swap_mesh_rows(previous);
            return Err(error);
        }
        Ok(true)
    }

    /// Roll back an inherited arena if refreshing the candidate fails. Neither
    /// candidate nor previous scene can record while the host owns this barrier.
    pub(crate) fn swap_mesh_rows(&mut self, previous: &mut Self) {
        std::mem::swap(&mut self.mesh_arena, &mut previous.mesh_arena);
        std::mem::swap(&mut self.row_updates, &mut previous.row_updates);
    }

    /// A different device cannot inherit buffers. Reject a recovery candidate
    /// whose CPU row snapshot predates a commit on the still-active host.
    pub(crate) fn has_current_mesh_rows(&self, previous: &Self) -> bool {
        match (&self.row_updates, &previous.row_updates) {
            (Some(next), Some(old)) => next.same_geometry(old),
            (None, None) => true,
            _ => false,
        }
    }

    pub(crate) fn submit_mesh_rows(&mut self) {
        if let Some(rows) = self.row_updates.as_mut() {
            rows.submitted();
        }
    }
}
