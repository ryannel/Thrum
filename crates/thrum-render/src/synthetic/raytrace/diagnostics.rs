use super::{PreparedSynthetic, RenderError};

/// One recorded instance from the list encoded by wgpu.
#[derive(Debug)]
pub struct RayStructureInstance {
    /// Logical scene slot, the ray query's custom data.
    pub slot: u32,
    /// Mesh arena row, or `u32::MAX` for a contribution placement.
    pub mesh_row: u32,
    /// The row-major affine transform passed to wgpu.
    pub transform: [f32; 12],
    /// Ray mask for shadows and ambient occlusion.
    pub mask: u8,
}

impl PreparedSynthetic {
    /// Exact forward source used by a diagnostic probe of the composed operations.
    #[must_use]
    pub fn forward_source_for_probe(&self) -> &str {
        &self.surface.shader_source
    }

    /// Inspect the prepared caster volumes, including the independent height-map volume.
    #[must_use]
    pub fn ray_inspection_shadow_volumes(
        &self,
        bounds: Option<crate::contribution::ShadowBounds>,
    ) -> Vec<crate::contribution::SecondaryShadowCascade> {
        self.shadow
            .as_ref()
            .map_or_else(Vec::new, |shadow| shadow.inspection_volumes(bounds))
    }

    /// Inspect the last recorded list, including from an abandoned encoder.
    #[must_use]
    pub fn ray_structure_instances(&self) -> Option<Vec<RayStructureInstance>> {
        let state = self.ray_structure.as_ref()?.state.lock().ok()?;
        Some(
            state
                .tlas
                .get()
                .iter()
                .flatten()
                .zip(&state.mesh_rows)
                .map(|(instance, &mesh_row)| RayStructureInstance {
                    slot: instance.custom_data,
                    mesh_row,
                    transform: instance.transform,
                    mask: instance.mask,
                })
                .collect(),
        )
    }

    /// Bind the scene structure to a diagnostic pipeline at binding zero.
    /// The caller records its ray query after the prepared graph's build.
    #[must_use]
    pub fn ray_structure_probe_group(
        &self,
        layout: &wgpu::BindGroupLayout,
    ) -> Option<wgpu::BindGroup> {
        let state = self.ray_structure.as_ref()?.state.lock().ok()?;
        Some(
            self.capabilities
                .device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("scene structure probe"),
                    layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: state.tlas.as_binding(),
                    }],
                }),
        )
    }

    /// The count of mesh structures allocated for the current arena rows.
    #[must_use]
    pub fn ray_mesh_structure_count(&self) -> usize {
        self.mesh_arena.mesh_structures().len()
    }

    /// Vertex, index and instance buffer usages in that order.
    ///
    /// # Panics
    /// A prepared scene must have its arena buffers.
    #[must_use]
    pub fn ray_structure_input_usages(&self) -> [wgpu::BufferUsages; 3] {
        let buffers = self.mesh_arena.buffers().expect("prepared mesh arena");
        [
            buffers.vertices.usage(),
            buffers.indices.usage(),
            self.instance_upload.buffer().usage(),
        ]
    }

    /// Record just the production structure build for a contended cost measurement.
    ///
    /// # Errors
    /// Refuses a scene without a structure build.
    pub fn encode_ray_structure_measurement(
        &self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), RenderError> {
        self.record_ray_structure(encoder, None)
    }
}
