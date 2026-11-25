//! Normalize HDR scale for MetalFX and publish the corresponding tone input.
use super::super::{RenderError, SceneContext, SyntheticTargetInputs};
use crate::{
    graph::{
        Access, AccessKind, CompiledPass, FrameGraph, GraphTextures, PassBody, PassScope,
        PipelineKey, ResourceDescriptor, ResourceId, ResourceScope, TextureKind,
    },
    pass_key::{PassKey, PipelineRole},
    runtime::DeviceSite,
};
use std::collections::BTreeSet;

const SAMPLE_BYTES: u64 = super::super::exposure::SAMPLE_BYTES;
const REPAIR_BYTES: u64 = 3 * size_of::<u32>() as u64;
pub(in crate::synthetic) const PASS: &str = PassKey::MetalFxNormalize.constant_name();

#[derive(Clone)]
pub(in crate::synthetic) struct Normalize {
    native: wgpu::ComputePipeline,
    twin: wgpu::ComputePipeline,
}

pub(in crate::synthetic) struct NormalizedResources {
    pub(in crate::synthetic) colour: ResourceId,
    pub(in crate::synthetic) exposure: ResourceId,
    source: ResourceId,
    pub(in crate::synthetic) pass: usize,
}

pub(in crate::synthetic) struct NormalizedTarget {
    pub(in crate::synthetic) exposure: [wgpu::Buffer; 2],
    native: [wgpu::BindGroup; 2],
    twin: [wgpu::BindGroup; 2],
    #[cfg(all(feature = "creation-counters", not(target_arch = "wasm32")))]
    pub(in crate::synthetic) repairs: wgpu::Buffer,
}

impl Normalize {
    pub(in crate::synthetic) fn new(device: &DeviceSite) -> Self {
        let shader = crate::shaders::Shader::MetalFxNormalize.compile(device, PASS);
        let pipeline = |entry_point| {
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some(PASS),
                layout: None,
                module: &shader,
                entry_point: Some(entry_point),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        Self {
            native: pipeline("normalize"),
            twin: pipeline("twin"),
        }
    }

    pub(in crate::synthetic) fn target(
        &self,
        device: &wgpu::Device,
        textures: &GraphTextures,
        ids: &NormalizedResources,
        samples: &[wgpu::Buffer; 2],
    ) -> Result<NormalizedTarget, RenderError> {
        let texture_view = |id| {
            textures
                .texture(id)
                .ok_or(RenderError::GraphPassMismatch)
                .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
        };
        let source = texture_view(&ids.source)?;
        let colour = texture_view(&ids.colour)?;
        let exposure = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MetalFX matching tone exposure"),
            size: SAMPLE_BYTES,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        let repairs = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MetalFX half-float repairs"),
            size: REPAIR_BYTES,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let groups = |pipeline: &wgpu::ComputePipeline, native| {
            samples.each_ref().map(|sample| {
                let mut entries = vec![
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: sample.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: exposure.as_entire_binding(),
                    },
                ];
                if native {
                    entries.extend([
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&source),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&colour),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: repairs.as_entire_binding(),
                        },
                    ]);
                }
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(PASS),
                    layout: &pipeline.get_bind_group_layout(0),
                    entries: &entries,
                })
            })
        };
        Ok(NormalizedTarget {
            native: groups(&self.native, true),
            twin: groups(&self.twin, false),
            exposure: [exposure.clone(), exposure],
            #[cfg(all(feature = "creation-counters", not(target_arch = "wasm32")))]
            repairs,
        })
    }
}

impl SyntheticTargetInputs {
    pub(in crate::synthetic) fn append_normalization(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        scene: &ResourceId,
        exposure: &ResourceId,
        [width, height]: [u32; 2],
    ) -> Result<Option<NormalizedResources>, RenderError> {
        if self.temporal.normalize.is_none() {
            return Ok(None);
        }
        let colour = graph.resource(
            ResourceDescriptor::Texture {
                dimension: TextureKind::D2,
                width,
                height,
                layers: 1,
                mips: 1,
                samples: 1,
                format: crate::render_config::SCENE_FORMAT,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            },
            ResourceScope::Transient,
        )?;
        let buffer = |graph: &mut FrameGraph, bytes, usage, initialized| {
            graph.resource(
                ResourceDescriptor::Buffer { bytes, usage },
                ResourceScope::Imported { initialized },
            )
        };
        let output = buffer(
            graph,
            SAMPLE_BYTES,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::UNIFORM,
            false,
        )?;
        let repairs = buffer(
            graph,
            REPAIR_BYTES,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            true,
        )?;
        let key = PipelineRole::MetalFxNormalize.key(self.key);
        keys.insert(key);
        let pass = graph.declare(
            PassKey::MetalFxNormalize,
            PassBody {
                scope: PassScope::View(1),
                pipeline: Some(key),
                accesses: vec![
                    Access::new(scene.clone(), 0..1, AccessKind::SampledRead),
                    Access::new(colour.clone(), 0..1, AccessKind::StorageWrite),
                    Access::new(exposure.clone(), 0..SAMPLE_BYTES, AccessKind::UniformRead),
                    Access::new(output.clone(), 0..SAMPLE_BYTES, AccessKind::StorageWrite),
                    Access::new(repairs, 0..REPAIR_BYTES, AccessKind::StorageReadWrite),
                ],
                external_side_effect: false,
            },
        );
        Ok(Some(NormalizedResources {
            colour,
            exposure: output,
            source: scene.clone(),
            pass,
        }))
    }
}

impl SceneContext<'_> {
    pub(in crate::synthetic) fn normalize_metalfx(
        &mut self,
        compiled: &CompiledPass,
    ) -> Result<(), RenderError> {
        let pipeline = self
            .prepared
            .temporal
            .normalize
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let target = self
            .target
            .temporal
            .normalized
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let native = self.native.ok_or(RenderError::GraphPassMismatch)?;
        let mut pass = compiled
            .begin_compute(self.encoder, self.generation, self.timings.as_deref_mut())?
            .ok_or(RenderError::GraphPassMismatch)?;
        if native.uses_twin() {
            pass.set_pipeline(&pipeline.twin);
            pass.set_bind_group(0, &target.twin[self.exposure_side], &[]);
            pass.dispatch_workgroups(1, 1, 1);
        } else {
            let [width, height] = native.content_extent();
            pass.set_pipeline(&pipeline.native);
            pass.set_bind_group(0, &target.native[self.exposure_side], &[]);
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
        Ok(())
    }
}

#[cfg(all(feature = "creation-counters", not(target_arch = "wasm32")))]
impl super::super::OffscreenTarget {
    pub(crate) fn metalfx_repairs(&self) -> Option<&wgpu::Buffer> {
        self.temporal
            .normalized
            .as_ref()
            .map(|target| &target.repairs)
    }
}

#[cfg(test)]
mod tests;
