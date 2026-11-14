use super::{PASS, RESET};
use crate::graph::{
    Access, AccessKind, FrameGraph, Pass, PassScope, ResourceDescriptor, ResourceId, ResourceScope,
    TextureKind,
};
use crate::synthetic::RenderError;

pub(crate) struct Resources {
    pub(crate) output: ResourceId,
    pub(crate) next: ResourceId,
    pub(crate) pass: usize,
}

pub(crate) fn append(
    graph: &mut FrameGraph,
    source: &ResourceId,
    [width, height]: [u32; 2],
) -> Result<Resources, RenderError> {
    let descriptor = |width, height| ResourceDescriptor::Texture {
        dimension: TextureKind::D2,
        width,
        height,
        layers: 1,
        mips: 1,
        samples: 1,
        format: super::SCENE_FORMAT,
        usage: wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::STORAGE_BINDING,
    };
    let output = graph.resource(
        descriptor(width, height),
        ResourceScope::Persistent {
            owner: 0xA110,
            epoch: 0,
            initialized: true,
        },
    )?;
    let previous = graph.resource(
        descriptor(width, height),
        ResourceScope::History {
            owner: 0xA111,
            epoch: 0,
            initialized: true,
        },
    )?;
    let next = graph.resource(
        descriptor(width, height),
        ResourceScope::History {
            owner: 0xA111,
            epoch: 0,
            initialized: true,
        },
    )?;
    let zero = graph.resource(
        descriptor(1, 1),
        ResourceScope::Imported { initialized: true },
    )?;
    graph.pass(Pass {
        name: RESET.into(),
        scope: PassScope::View(1),
        pipeline: None,
        accesses: vec![
            Access::new(zero, 0..1, AccessKind::CopyRead),
            Access::new(previous.clone(), 0..1, AccessKind::CopyWrite),
        ],
        external_side_effect: false,
    });
    let pass = graph.native_pass(Pass {
        name: PASS.into(),
        scope: PassScope::View(1),
        pipeline: None,
        accesses: vec![
            Access::new(source.clone(), 0..1, AccessKind::CopyRead),
            Access::new(previous.clone(), 0..1, AccessKind::SampledRead),
            Access::new(output.clone(), 0..1, AccessKind::StorageWrite),
            Access::new(next.clone(), 0..1, AccessKind::StorageWrite),
        ],
        external_side_effect: false,
    });
    Ok(Resources { output, next, pass })
}
