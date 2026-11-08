//! Current-frame Sun receiver mask. The optional graph owns its stored
//! resources separately from forward depth and camera occlusion.

use std::collections::BTreeSet;

use super::{
    AUX_BYTES, CompiledPass, DEPTH_FORMAT, GraphColorAttachment, GraphDepthAttachment,
    GraphTextures, PipelineKey, RenderError, ResourceId, SceneContext,
};
use crate::graph::{
    Access, AccessKind, FrameGraph, Pass, PassScope, ResourceDescriptor, ResourceScope,
};

pub(super) const GUIDE: wgpu::TextureFormat = wgpu::TextureFormat::R32Uint;
pub(super) const RASTER_PASS: &str = "Sun receiver raster";
pub(super) const RESOLVE_PASS: &str = "Sun receiver resolve";
pub(super) const RASTER_ENTRY: &str = "receiver_fragment";
const RESOLVED: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;
const RASTER_ROLE: u32 = 1100;
const RESOLVE_ROLE: u32 = 1101;
const DEPTH_BINDING: u32 = 6;
const MASK_BINDING: u32 = 7;

pub(super) fn raster_key(base: PipelineKey) -> PipelineKey {
    PipelineKey {
        role: RASTER_ROLE,
        ..base
    }
}

pub(super) fn resolve_key(base: PipelineKey) -> PipelineKey {
    PipelineKey {
        role: RESOLVE_ROLE,
        ..base
    }
}

pub(super) fn forward_entries() -> [wgpu::BindGroupLayoutEntry; 2] {
    [
        (DEPTH_BINDING, wgpu::TextureSampleType::Depth),
        (
            MASK_BINDING,
            wgpu::TextureSampleType::Float { filterable: false },
        ),
    ]
    .map(|(binding, sample_type)| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    })
}

pub(super) struct Resources {
    depth: ResourceId,
    guide: ResourceId,
    resolved: ResourceId,
}

impl Resources {
    pub(super) fn declare(
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        base: PipelineKey,
        [width, height]: [u32; 2],
        mut reads: Vec<Access>,
        aux: ResourceId,
        forward: &mut Vec<Access>,
    ) -> Result<Self, RenderError> {
        use wgpu::TextureUsages as T;
        let resource = |graph: &mut FrameGraph, format, writes| {
            graph.resource(
                ResourceDescriptor::Texture {
                    width,
                    height,
                    layers: 1,
                    mips: 1,
                    samples: 1,
                    format,
                    usage: T::TEXTURE_BINDING | writes,
                },
                ResourceScope::Transient,
            )
        };
        let depth = resource(graph, DEPTH_FORMAT, T::RENDER_ATTACHMENT)?;
        let guide = resource(graph, GUIDE, T::RENDER_ATTACHMENT)?;
        let resolved = resource(graph, RESOLVED, T::STORAGE_BINDING)?;
        let texture_access = |resource: &ResourceId, kind| Access {
            resource: resource.clone(),
            range: 0..1,
            kind,
            store: kind == AccessKind::StorageWrite,
        };
        let raster = raster_key(base);
        let resolve = resolve_key(base);
        keys.extend([raster, resolve]);
        reads.extend([&depth, &guide].map(|id| texture_access(id, AccessKind::AttachmentClear)));
        graph.pass(Pass {
            name: RASTER_PASS.into(),
            scope: PassScope::View(1),
            pipeline: Some(raster),
            accesses: reads,
            external_side_effect: false,
        });
        graph.pass(Pass {
            name: RESOLVE_PASS.into(),
            scope: PassScope::View(1),
            pipeline: Some(resolve),
            accesses: vec![
                texture_access(&depth, AccessKind::SampledRead),
                texture_access(&guide, AccessKind::SampledRead),
                texture_access(&resolved, AccessKind::StorageWrite),
                Access {
                    resource: aux,
                    range: 0..AUX_BYTES as u64,
                    kind: AccessKind::UniformRead,
                    store: true,
                },
            ],
            external_side_effect: false,
        });
        forward.extend([&depth, &resolved].map(|id| texture_access(id, AccessKind::SampledRead)));
        Ok(Self {
            depth,
            guide,
            resolved,
        })
    }
}

pub(super) struct Prepared {
    pub(super) forward_view_layout: wgpu::BindGroupLayout,
    pub(super) raster: [wgpu::RenderPipeline; 2],
    compute: wgpu::ComputePipeline,
}

impl Prepared {
    pub(super) fn new(
        device: &crate::runtime::DeviceSite,
        forward_view_layout: wgpu::BindGroupLayout,
        raster: [wgpu::RenderPipeline; 2],
    ) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(RESOLVE_PASS),
            source: wgpu::ShaderSource::Wgsl(include_str!("receiver_resolve.wgsl").into()),
        });
        let compute = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(RESOLVE_PASS),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Self {
            forward_view_layout,
            raster,
            compute,
        }
    }

    pub(super) fn target(
        &self,
        device: &wgpu::Device,
        resources: &Resources,
        textures: &GraphTextures,
        aux: &wgpu::Buffer,
        view_entries: &[wgpu::BindGroupEntry<'_>],
    ) -> Result<Target, RenderError> {
        let view = |id| {
            textures
                .texture(id)
                .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
                .ok_or(RenderError::GraphPassMismatch)
        };
        let depth = view(&resources.depth)?;
        let guide = view(&resources.guide)?;
        let resolved = view(&resources.resolved)?;
        let compute = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(RESOLVE_PASS),
            layout: &self.compute.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: aux.as_entire_binding(),
                },
                texture_entry(1, &depth),
                texture_entry(2, &guide),
                texture_entry(3, &resolved),
            ],
        });
        // Clone descriptor entries only at target preparation. The original
        // entries remain unextended for the receiver and occlusion raster.
        let mut forward = view_entries.to_vec();
        forward.extend([
            texture_entry(DEPTH_BINDING, &depth),
            texture_entry(MASK_BINDING, &resolved),
        ]);
        let forward = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("forward with Sun receiver mask"),
            layout: &self.forward_view_layout,
            entries: &forward,
        });
        Ok(Target {
            depth,
            guide,
            compute,
            forward,
        })
    }
}

fn texture_entry(binding: u32, view: &wgpu::TextureView) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::TextureView(view),
    }
}

pub(super) struct Target {
    depth: wgpu::TextureView,
    guide: wgpu::TextureView,
    compute: wgpu::BindGroup,
    pub(super) forward: wgpu::BindGroup,
}

impl SceneContext<'_> {
    pub(super) fn receiver_raster(&mut self, compiled: &CompiledPass) -> Result<(), RenderError> {
        let target = self
            .target
            .receiver
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let ids = self
            .target
            .ids
            .receiver
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let prepared = self
            .prepared
            .receiver
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let mut pass = compiled
            .begin_render(
                self.encoder,
                [Some(GraphColorAttachment {
                    resource: &ids.guide,
                    range: 0..1,
                    view: &target.guide,
                    clear: wgpu::Color::TRANSPARENT,
                })],
                Some(GraphDepthAttachment {
                    resource: &ids.depth,
                    range: 0..1,
                    view: &target.depth,
                    clear: 0.0,
                }),
                self.generation,
                self.timings.as_deref_mut(),
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        // Contributions without an explicit receiver implementation leave the
        // whole view on the stock path.
        if self
            .contribution
            .as_deref()
            .is_none_or(|contribution| contribution.supports_receivers(&self.contribution_frame))
        {
            let sided = usize::from(self.prepared.instance_upload.two_sided_count() != 0);
            pass.set_pipeline(&prepared.raster[sided]);
            self.prepared.bind_opaque_inputs(
                &mut pass,
                self.target,
                &self.target.view_bind_group,
                self.exposure_side,
            );
            self.prepared.draw_camera(&mut pass, self.target);
            if let Some(contribution) = self.contribution.as_deref() {
                contribution.draw_receivers(&mut pass, &self.contribution_frame);
            }
        }
        Ok(())
    }

    pub(super) fn receiver_resolve(&mut self, compiled: &CompiledPass) -> Result<(), RenderError> {
        let target = self
            .target
            .receiver
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let prepared = self
            .prepared
            .receiver
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let mut pass = compiled
            .begin_compute(self.encoder, self.generation, self.timings.as_deref_mut())?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_pipeline(&prepared.compute);
        pass.set_bind_group(0, &target.compute, &[]);
        pass.dispatch_workgroups(
            self.target.width.div_ceil(8),
            self.target.height.div_ceil(8),
            1,
        );
        Ok(())
    }
}
