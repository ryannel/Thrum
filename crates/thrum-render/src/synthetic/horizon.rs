//! Maximum-depth mips over the top-down caster map for distant receiver shadows.

use super::{RenderError, shadow};
use crate::graph::{
    Access, AccessKind, FrameGraph, Pass, PassScope, PipelineKey, ResourceDescriptor, ResourceId,
    ResourceScope,
};
use crate::visibility::pyramid::{PYRAMID_FORMAT, PyramidPrepared, PyramidTarget};

pub(super) const PASS: &str = "synthetic Sun horizon maxima";
const SIZE: u32 = shadow::SIZE / 2;
const MIPS: u32 = SIZE.ilog2() + 1;
const BINDING: u32 = 6;

pub(super) const VIEW_ENTRY: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
    binding: BINDING,
    visibility: wgpu::ShaderStages::FRAGMENT,
    ty: wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Float { filterable: false },
        view_dimension: wgpu::TextureViewDimension::D2,
        multisampled: false,
    },
    count: None,
};

pub(super) fn prepare(device: &wgpu::Device) -> PyramidPrepared {
    PyramidPrepared::with_shader(
        device,
        PASS,
        include_str!("horizon_reduce.wgsl"),
        ["depth_parent", "mip_parent"],
    )
}

pub(super) struct HorizonTarget {
    pub(super) pyramid: PyramidTarget,
    view: wgpu::TextureView,
}

impl HorizonTarget {
    pub(super) fn new(
        device: &wgpu::Device,
        prepared: &PyramidPrepared,
        depth: &wgpu::TextureView,
        texture: &wgpu::Texture,
    ) -> Self {
        Self {
            pyramid: prepared.target_from_texture(device, depth, texture.clone()),
            view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
        }
    }

    pub(super) fn view_entry(&self) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding: BINDING,
            resource: wgpu::BindingResource::TextureView(&self.view),
        }
    }
}

pub(super) fn append_graph(
    graph: &mut FrameGraph,
    key: PipelineKey,
    maps: &ResourceId,
    forward_reads: &mut Vec<Access>,
) -> Result<ResourceId, RenderError> {
    let pyramid = graph.resource(
        ResourceDescriptor::Texture {
            width: SIZE,
            height: SIZE,
            layers: 1,
            mips: MIPS,
            samples: 1,
            format: PYRAMID_FORMAT,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        },
        ResourceScope::Transient,
    )?;
    let range = 0..u64::from(MIPS);
    graph.pass(Pass {
        name: PASS.into(),
        scope: PassScope::View(1),
        pipeline: Some(key),
        accesses: vec![
            Access {
                resource: maps.clone(),
                range: u64::from(shadow::LAYERS - 1)..u64::from(shadow::LAYERS),
                kind: AccessKind::SampledRead,
                store: true,
            },
            Access {
                resource: pyramid.clone(),
                range: range.clone(),
                kind: AccessKind::StorageWrite,
                store: true,
            },
        ],
        external_side_effect: false,
    });
    forward_reads.push(Access {
        resource: pyramid.clone(),
        range,
        kind: AccessKind::SampledRead,
        store: true,
    });
    Ok(pyramid)
}

impl super::SceneContext<'_> {
    pub(super) fn horizon(&mut self, pass: &crate::graph::CompiledPass) -> Result<(), RenderError> {
        let (prepared, target) = self
            .prepared
            .horizon
            .as_ref()
            .zip(self.target.horizon.as_ref())
            .ok_or(RenderError::GraphPassMismatch)?;
        let mut pass = pass
            .begin_compute(self.encoder, self.generation, self.timings.as_deref_mut())?
            .ok_or(RenderError::GraphPassMismatch)?;
        prepared.encode(&mut pass, &target.pyramid);
        Ok(())
    }
}
