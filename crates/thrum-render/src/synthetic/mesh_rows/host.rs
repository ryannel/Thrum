//! Row recording and ownership transfer at the frame host's publication barrier.

use super::super::{PreparedSynthetic, RenderError, camera_culling, mesh_arena::MeshArena};

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
        Self::refresh_mesh_records(
            &self.mesh_arena,
            queue,
            &mut self.bucket_route,
            &mut self.caster_boxes,
            self.visibility.as_ref(),
            self.caster_culling.as_ref().map(|(culling, _)| culling),
            &self.secondary,
        )?;
        self.instance_upload
            .refresh_cast_widths(queue, self.caster_boxes.as_deref());
        Ok(())
    }

    fn refresh_mesh_records(
        arena: &MeshArena,
        queue: &wgpu::Queue,
        route: &mut Option<super::super::mesh_route::BucketRoute>,
        boxes: &mut Option<Vec<super::super::CasterBox>>,
        visibility: Option<&crate::visibility::VisibilityPrepared>,
        caster_culling: Option<&super::super::CasterCulling>,
        secondary: &[super::super::secondary::SecondaryPrepared],
    ) -> Result<(), RenderError> {
        if let Some(route) = route {
            *boxes = Some(route.refresh_rows(arena)?);
        }
        let records = camera_culling::mesh_records(arena)?;
        if let Some(visibility) = visibility {
            visibility.update_meshes(queue, &records);
        }
        if let Some(culling) = caster_culling {
            culling.update_meshes(queue, &records);
        }
        for slot in secondary {
            slot.update_caster_meshes(queue, &records);
        }
        Ok(())
    }

    /// Bind the candidate against the accepted arena, keeping the arena and its
    /// upload cursor with the current owner until the publication barrier.
    pub(crate) fn stage_mesh_rows(
        &mut self,
        previous: &Self,
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
        Self::refresh_mesh_records(
            &previous.mesh_arena,
            queue,
            &mut self.bucket_route,
            &mut self.caster_boxes,
            self.visibility.as_ref(),
            self.caster_culling.as_ref().map(|(culling, _)| culling),
            &self.secondary,
        )?;
        self.instance_upload
            .refresh_cast_widths(queue, self.caster_boxes.as_deref());
        self.history.rebind_mesh_vertices(
            self.capabilities.device(),
            self.instance_upload.buffer(),
            &previous
                .mesh_arena
                .buffers()
                .ok_or(RenderError::MissingMeshBuffers)?
                .vertices,
        );
        Ok(true)
    }

    /// Transfer the borrowed arena and live row state after all checks pass.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn commit_mesh_rows(&mut self, previous: &mut Self) {
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
