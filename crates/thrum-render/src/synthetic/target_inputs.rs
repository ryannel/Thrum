//! Retained inputs for allocating another target ring on the same device.

use super::{
    AirShadowPrepared, Arc, AtmospherePrepared, BucketRoute, CasterCulling, CloudPrepared,
    ContributionPasses, CullMode, DeviceCapabilities, PipelineKey, PreparedSynthetic, SdrToneStage,
    ShadowSelection, StarPrepared, SunShadowMethod, SyntheticView, TextureClasses,
    VisibilityPrepared, mesh_arena, mesh_route, native_witness, secondary, taau, temporal,
};

/// Device resources retain their original handles. Only extent-dependent
/// attachments, bindings and providers are allocated by a target build.
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent scene features are copied from the active renderer"
)]
pub(crate) struct SyntheticTargetInputs {
    pub(super) capabilities: DeviceCapabilities,
    pub(super) key: PipelineKey,
    pub(super) owner: Arc<()>,
    pub(super) native: Option<Arc<native_witness::Factory>>,
    pub(super) temporal_inputs: bool,
    pub(super) temporal: temporal::TemporalInputs,
    pub(super) taau: Option<taau::Taau>,
    pub(super) cloud: Option<CloudPrepared>,
    pub(super) classes: TextureClasses,
    pub(super) mesh_buffers: mesh_arena::ArenaBuffers,
    pub(super) ray_demand: bool,
    pub(super) ray_structure: bool,
    pub(super) traced_view: bool,
    pub(super) uses_shadow_rays: bool,
    pub(super) soft_sun: Option<super::raytrace::soft::SoftPipelines>,
    pub(super) shadow_selection: ShadowSelection,
    pub(super) bucket_route: Option<mesh_route::TargetMembers>,
    pub(super) caster_culling: Option<CasterCulling>,
    pub(super) display: SdrToneStage,
    pub(super) view_layout: wgpu::BindGroupLayout,
    pub(super) view_buffer: wgpu::Buffer,
    pub(super) view_sample: SyntheticView,
    pub(super) pending_view: Option<SyntheticView>,
    pub(super) frame_revision: u64,
    pub(super) directional_buffer: wgpu::Buffer,
    pub(super) environment_light: wgpu::Buffer,
    pub(super) cluster_bounds_bytes: Option<u64>,
    pub(super) exposure_samples: [wgpu::Buffer; 2],
    pub(super) instance_buffer: wgpu::Buffer,
    pub(super) history_serials: wgpu::Buffer,
    pub(super) material_bytes: u64,
    pub(super) instance_bytes: u64,
    pub(super) instance_count: u32,
    pub(super) local_light_bytes: u64,
    pub(super) cluster_head_bytes: u64,
    pub(super) cluster_index_bytes: u64,
    pub(super) shadow: Option<wgpu::Buffer>,
    pub(super) air_shadow: Option<AirShadowPrepared>,
    pub(super) atmosphere: Option<AtmospherePrepared>,
    pub(super) contribution_passes: ContributionPasses,
    pub(crate) effect_graph: crate::contribution::EffectGraph,
    pub(super) visibility: Option<VisibilityPrepared>,
    pub(super) stars: Option<StarPrepared>,
    pub(super) secondary: Vec<secondary::SecondaryPrepared>,
}

impl PreparedSynthetic {
    pub(crate) fn target_inputs(&self) -> SyntheticTargetInputs {
        SyntheticTargetInputs {
            capabilities: self.capabilities.clone(),
            key: self.key,
            owner: Arc::clone(&self.owner),
            native: self.native.clone(),
            temporal_inputs: self.temporal_inputs,
            temporal: self.temporal.clone(),
            taau: self.taau.clone(),
            cloud: self.cloud.clone(),
            classes: self.classes.clone(),
            mesh_buffers: self.mesh_arena.buffers().expect("prepared arena").clone(),
            ray_demand: self.mesh_arena.ray_demand(),
            ray_structure: self.ray_structure.is_some(),
            traced_view: self.traced_view.is_some(),
            uses_shadow_rays: self.uses_shadow_rays(),
            soft_sun: self.soft_sun.as_ref().map(|soft| soft.pipelines.clone()),
            shadow_selection: self.shadow_selection,
            bucket_route: self.bucket_route.as_ref().map(BucketRoute::target_members),
            caster_culling: self
                .caster_culling
                .as_ref()
                .map(|(culling, _)| culling.clone()),
            display: self.display.clone(),
            view_layout: self.view_layout.clone(),
            view_buffer: self.view_buffer.clone(),
            view_sample: self.view_sample,
            pending_view: self.pending_view,
            frame_revision: self.frame_revision,
            directional_buffer: self.directional_buffer.clone(),
            environment_light: self.environment_light.clone(),
            cluster_bounds_bytes: self
                .cluster_compute
                .as_ref()
                .map(|compute| compute.bounds.size()),
            exposure_samples: self.exposure.samples.clone(),
            instance_buffer: self.instance_upload.buffer().clone(),
            history_serials: self.history.serial_buffer().clone(),
            material_bytes: self.material_bytes,
            instance_bytes: self.instance_bytes,
            instance_count: self.instance_count,
            local_light_bytes: self.local_light_bytes,
            cluster_head_bytes: self.cluster_head_bytes,
            cluster_index_bytes: self.cluster_index_bytes,
            shadow: self
                .shadow
                .as_ref()
                .map(|shadow| shadow.sample_buffer.clone()),
            air_shadow: self.air_shadow.clone(),
            atmosphere: self.atmosphere.clone(),
            contribution_passes: self.contribution_passes,
            effect_graph: self.effect_graph.clone(),
            visibility: self.visibility.clone(),
            stars: self.stars.clone(),
            secondary: self.secondary.clone(),
        }
    }
}

impl SyntheticTargetInputs {
    pub(super) fn surface_uses_cascades(&self) -> bool {
        self.shadow_selection.sun == SunShadowMethod::Maps
    }

    pub(super) fn camera_culling(&self) -> CullMode {
        self.visibility
            .as_ref()
            .map_or(CullMode::Direct, VisibilityPrepared::mode)
    }
}
