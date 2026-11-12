//! Public, generation-prepared effect declarations. Physical resources stay with the effect.

use super::HistoryBinding;
use crate::graph::{
    Access, AccessKind, FrameGraph, Pass, PassScope, PipelineKey, ResourceDescriptor, ResourceScope,
};
use std::{collections::BTreeSet, ops::Range};

/// Work frequency, independent of graph compilation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectCadence {
    /// Execute on every frame.
    EveryFrame,
    /// Execute a whole-output update when requested by the effect.
    Skippable,
    /// Execute an in-place slice, or the whole resource on a full frame.
    TimeSliced,
}

/// A local resource access. History bindings refer to a logical pair, not a global frame side.
#[derive(Clone, Debug)]
pub struct EffectAccess {
    /// Index into this declaration's resources.
    pub resource: usize,
    /// Byte or mip/layer range, as in the frame graph.
    pub range: Range<u64>,
    /// Concrete GPU usage.
    pub kind: AccessKind,
    /// Explicit texture view, when different from the natural descriptor view.
    pub view: Option<wgpu::TextureViewDimension>,
    /// Required for a history resource; absent for other scopes.
    pub history: Option<HistoryBinding>,
}

/// One named simulate pass; upload, shadow and opaque use the existing callbacks.
#[derive(Clone, Debug)]
pub struct EffectPass {
    /// Diagnostic name, unique within this contribution.
    pub name: String,
    /// Runtime scheduling class.
    pub cadence: EffectCadence,
    /// Real resource hazards, never placeholder token resources.
    pub accesses: Vec<EffectAccess>,
}

/// Resources and simulate passes declared once during preparation.
#[derive(Clone, Debug, Default)]
pub struct EffectGraph {
    /// Physical descriptors and their retained ownership scopes.
    pub resources: Vec<(ResourceDescriptor, ResourceScope)>,
    /// Ordered simulate passes.
    pub passes: Vec<EffectPass>,
}

impl EffectGraph {
    /// Refuse declarations whose runtime skips could violate the compiled graph.
    /// # Errors
    /// Returns a named preparation error for a cadence, producer or binding misuse.
    pub fn validate(&self) -> Result<(), String> {
        let mut names = BTreeSet::new();
        let mut producers = vec![None; self.resources.len()];
        for (index, pass) in self.passes.iter().enumerate() {
            if !names.insert(&pass.name) {
                return Err(format!("DuplicateEffectPass: {}", pass.name));
            }
            for access in &pass.accesses {
                let (_, scope) = self
                    .resources
                    .get(access.resource)
                    .ok_or("UnknownEffectResource")?;
                let writes = matches!(
                    access.kind,
                    AccessKind::StorageWrite
                        | AccessKind::StorageReadWrite
                        | AccessKind::CopyWrite
                        | AccessKind::AttachmentLoad
                        | AccessKind::AttachmentClear
                );
                if writes
                    && pass.cadence != EffectCadence::EveryFrame
                    && (!matches!(
                        scope,
                        ResourceScope::Persistent { .. } | ResourceScope::History { .. }
                    ) || matches!(access.kind, AccessKind::AttachmentLoad))
                {
                    return Err(format!("CadenceRequiresRetainedOutput: {}", pass.name));
                }
                if matches!(scope, ResourceScope::History { .. }) {
                    if writes {
                        if pass.cadence == EffectCadence::TimeSliced {
                            return Err(format!("TimeSlicedHistory: {}", pass.name));
                        }
                        if access.history != Some(HistoryBinding::Next) {
                            return Err("HistoryWriteRequiresNext".into());
                        }
                        if let Some(previous) = producers[access.resource]
                            && previous != index
                        {
                            return Err("DuplicateHistoryProducer".into());
                        }
                        producers[access.resource] = Some(index);
                    } else if access.history.is_none() {
                        return Err("HistoryBindingRequired".into());
                    }
                } else if access.history.is_some() {
                    return Err("HistoryBindingOnNonHistory".into());
                }
            }
        }
        for (index, pass) in self.passes.iter().enumerate() {
            for access in &pass.accesses {
                if access.history == Some(HistoryBinding::Result)
                    && producers[access.resource].is_none_or(|producer| producer >= index)
                {
                    return Err("ResultBeforeProducer".into());
                }
            }
        }
        Ok(())
    }

    pub(crate) fn append(
        &self,
        graph: &mut FrameGraph,
        base: PipelineKey,
        keys: &mut BTreeSet<PipelineKey>,
    ) -> Result<Vec<Access>, crate::synthetic::RenderError> {
        self.validate()
            .map_err(crate::synthetic::RenderError::Validation)?;
        let mut owners = std::collections::BTreeMap::new();
        let resources = self
            .resources
            .iter()
            .map(|(descriptor, scope)| {
                let mut scope = *scope;
                if let ResourceScope::Persistent { owner, .. }
                | ResourceScope::History { owner, .. } = &mut scope
                {
                    let next = 0xEFFE_0000_0000_0000 + owners.len() as u64;
                    *owner = *owners.entry(*owner).or_insert(next);
                }
                let previous = graph.resource(descriptor.clone(), scope)?;
                let next = if matches!(scope, ResourceScope::History { .. }) {
                    graph.resource(descriptor.clone(), scope)?
                } else {
                    previous.clone()
                };
                Ok((previous, next))
            })
            .collect::<Result<Vec<_>, crate::graph::GraphError>>()?;
        for (index, pass) in self.passes.iter().enumerate() {
            let key = PipelineKey {
                role: 2000
                    + u32::try_from(index)
                        .map_err(|_| crate::synthetic::RenderError::SceneCapacity)?,
                ..base
            };
            keys.insert(key);
            graph.pass(Pass {
                name: format!("effect {index}: {}", pass.name),
                scope: PassScope::View(1),
                pipeline: Some(key),
                accesses: pass
                    .accesses
                    .iter()
                    .map(|access| Access {
                        resource: if matches!(
                            access.history,
                            Some(HistoryBinding::Next | HistoryBinding::Result)
                        ) {
                            resources[access.resource].1.clone()
                        } else {
                            resources[access.resource].0.clone()
                        },
                        range: access.range.clone(),
                        kind: access.kind,
                        view: access.view,
                        store: true,
                    })
                    .collect(),
                external_side_effect: true,
            });
        }
        let mut reads = Vec::new();
        for ((descriptor, _), (_, result)) in self.resources.iter().zip(&resources) {
            let (range, kind) = match descriptor {
                ResourceDescriptor::Texture {
                    layers,
                    mips,
                    usage,
                    ..
                } if usage.contains(wgpu::TextureUsages::TEXTURE_BINDING) => (
                    0..u64::from(*layers) * u64::from(*mips),
                    AccessKind::SampledRead,
                ),
                ResourceDescriptor::Buffer { bytes, usage }
                    if usage.contains(wgpu::BufferUsages::STORAGE) =>
                {
                    (0..*bytes, AccessKind::StorageRead)
                }
                _ => continue,
            };
            reads.push(Access {
                resource: result.clone(),
                range,
                kind,
                view: None,
                store: true,
            });
        }
        Ok(reads)
    }
}
