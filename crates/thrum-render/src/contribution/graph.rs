//! Public, generation-prepared effect declarations. Physical resources stay with the effect.

use super::HistoryBinding;
use crate::graph::PassBody;
use crate::graph::{
    Access, AccessKind, FrameGraph, PassScope, PipelineKey, ResourceDescriptor, ResourceId,
    ResourceScope,
};
use crate::pass_key::{PassKey, PipelineRole};
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
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
pub struct EffectPass {
    /// Diagnostic name, unique within this contribution.
    pub name: String,
    /// Runtime scheduling class.
    pub cadence: EffectCadence,
    /// Eligible for a future compute queue. This never changes scheduling today.
    pub async_eligible: bool,
    /// Real resource hazards, never placeholder token resources.
    pub accesses: Vec<EffectAccess>,
}

/// Resources and simulate passes declared once during preparation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EffectGraph {
    /// Physical descriptors and their retained ownership scopes.
    pub resources: Vec<(ResourceDescriptor, ResourceScope)>,
    /// Ordered simulate passes.
    pub passes: Vec<EffectPass>,
    /// Resources read by the fixed opaque draw callback. Late-only outputs stay out of forward.
    pub opaque_reads: Vec<EffectAccess>,
    /// Render passes after the sky and stars, in contribution order.
    pub after_opaque: Vec<super::AfterOpaquePass>,
}

pub(crate) struct EffectResources {
    pub(crate) opaque_reads: Vec<Access>,
    pub(crate) simulate: Vec<usize>,
    pairs: Vec<(ResourceId, ResourceId)>,
}
impl EffectResources {
    pub(crate) fn accesses(&self, accesses: &[EffectAccess]) -> Vec<Access> {
        accesses
            .iter()
            .map(|access| Access {
                resource: if matches!(
                    access.history,
                    Some(HistoryBinding::Next | HistoryBinding::Result)
                ) {
                    self.pairs[access.resource].1.clone()
                } else {
                    self.pairs[access.resource].0.clone()
                },
                range: access.range.clone(),
                kind: access.kind,
                view: access.view,
                store: true,
            })
            .collect()
    }
}

impl EffectGraph {
    fn declarations(
        &self,
    ) -> impl Iterator<Item = (Option<&str>, EffectCadence, bool, &[EffectAccess])> {
        self.passes
            .iter()
            .map(|pass| {
                (
                    Some(pass.name.as_str()),
                    pass.cadence,
                    pass.async_eligible,
                    pass.accesses.as_slice(),
                )
            })
            .chain(std::iter::once((
                None,
                EffectCadence::EveryFrame,
                false,
                self.opaque_reads.as_slice(),
            )))
            .chain(self.after_opaque.iter().map(|pass| {
                (
                    Some(pass.name.as_str()),
                    pass.cadence,
                    false,
                    pass.accesses.as_slice(),
                )
            }))
    }
    /// Refuse declarations whose runtime skips could violate the compiled graph.
    /// # Errors
    /// Returns a named preparation error for a cadence, producer or binding misuse.
    pub fn validate(&self) -> Result<(), crate::error::EffectGraphError> {
        use crate::error::EffectGraphError as Error;

        let mut owners = BTreeSet::new();
        for (_, scope) in &self.resources {
            if let ResourceScope::Persistent { owner, .. } | ResourceScope::History { owner, .. } =
                scope
                && !owners.insert(*owner)
            {
                return Err(Error::DuplicateEffectOwner);
            }
        }
        let mut names = BTreeSet::new();
        for pass in &self.after_opaque {
            if pass.write_depth && (pass.depth || pass.kind != super::AfterOpaqueKind::Render) {
                return Err(Error::AfterOpaqueDepthConflict(pass.name.clone()));
            }
            if pass.kind == super::AfterOpaqueKind::Render
                && pass.accesses.iter().any(|access| {
                    matches!(
                        access.kind,
                        AccessKind::AttachmentLoad
                            | AccessKind::AttachmentClear
                            | AccessKind::CopyRead
                            | AccessKind::CopyWrite
                    )
                })
            {
                return Err(Error::AfterOpaqueUsesSceneAttachments(pass.name.clone()));
            }
            if pass.kind == super::AfterOpaqueKind::Render
                && pass.cadence != EffectCadence::EveryFrame
            {
                return Err(Error::CadenceRequiresRetainedOutput(pass.name.clone()));
            }
            if pass.kind == super::AfterOpaqueKind::Compute
                && pass.accesses.iter().any(|access| {
                    matches!(
                        access.kind,
                        AccessKind::AttachmentLoad
                            | AccessKind::AttachmentClear
                            | AccessKind::CopyRead
                            | AccessKind::CopyWrite
                            | AccessKind::VertexRead
                            | AccessKind::IndexRead
                    )
                })
            {
                return Err(Error::AfterOpaqueRequiresComputeAccess(pass.name.clone()));
            }
        }
        let mut producers = vec![None; self.resources.len()];
        for (index, (name, cadence, async_eligible, accesses)) in self.declarations().enumerate() {
            if let Some(name) = name
                && !names.insert(name)
            {
                return Err(Error::DuplicateEffectPass(name.to_owned()));
            }
            for access in accesses {
                if async_eligible
                    && matches!(
                        access.kind,
                        AccessKind::AttachmentLoad
                            | AccessKind::AttachmentClear
                            | AccessKind::CopyRead
                            | AccessKind::CopyWrite
                            | AccessKind::VertexRead
                            | AccessKind::IndexRead
                    )
                {
                    return Err(Error::AsyncRequiresCompute(
                        name.unwrap_or("opaque reads").to_owned(),
                    ));
                }
                let (_, scope) = self
                    .resources
                    .get(access.resource)
                    .ok_or(Error::UnknownEffectResource)?;
                let writes = matches!(
                    access.kind,
                    AccessKind::StorageWrite
                        | AccessKind::StorageReadWrite
                        | AccessKind::CopyWrite
                        | AccessKind::AttachmentLoad
                        | AccessKind::AttachmentClear
                );
                if name.is_none() && (writes || access.kind == AccessKind::CopyRead) {
                    return Err(Error::OpaqueReadRequiresReadAccess);
                }
                if writes
                    && cadence != EffectCadence::EveryFrame
                    && (!matches!(
                        scope,
                        ResourceScope::Persistent { .. } | ResourceScope::History { .. }
                    ) || matches!(access.kind, AccessKind::AttachmentLoad))
                {
                    return Err(Error::CadenceRequiresRetainedOutput(
                        name.unwrap_or("opaque reads").to_owned(),
                    ));
                }
                if matches!(scope, ResourceScope::History { .. }) {
                    if writes {
                        if cadence == EffectCadence::TimeSliced {
                            return Err(Error::TimeSlicedHistory(
                                name.unwrap_or("opaque reads").to_owned(),
                            ));
                        }
                        if access.history != Some(HistoryBinding::Next) {
                            return Err(Error::HistoryWriteRequiresNext);
                        }
                        if let Some(previous) = producers[access.resource]
                            && previous != index
                        {
                            return Err(Error::DuplicateHistoryProducer);
                        }
                        producers[access.resource] = Some(index);
                    } else if access.history.is_none() {
                        return Err(Error::HistoryBindingRequired);
                    }
                } else if access.history.is_some() {
                    return Err(Error::HistoryBindingOnNonHistory);
                }
            }
        }
        for (index, (_, _, _, accesses)) in self.declarations().enumerate() {
            for access in accesses {
                if access.history == Some(HistoryBinding::Result)
                    && producers[access.resource].is_none_or(|producer| producer >= index)
                {
                    return Err(Error::ResultBeforeProducer);
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
    ) -> Result<EffectResources, crate::error::RenderError> {
        self.validate()?;
        let mut owners = std::collections::BTreeMap::new();
        let pairs = self
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
        let mut resources = EffectResources {
            pairs,
            opaque_reads: Vec::new(),
            simulate: Vec::new(),
        };
        for (index, pass) in self.passes.iter().enumerate() {
            let key = PipelineRole::Effect(
                u32::try_from(index).map_err(|_| crate::error::RenderError::EffectPassCount)?,
            )
            .key(base);
            keys.insert(key);
            let id = graph.declare_named(
                PassKey::Effect(crate::pass_key::pass_index(index)),
                format!("effect {index}: {}", pass.name),
                PassBody {
                    scope: PassScope::View(1),
                    pipeline: Some(key),
                    accesses: resources.accesses(&pass.accesses),
                    external_side_effect: true,
                },
            );
            resources.simulate.push(id);
            if pass.async_eligible {
                graph.mark_async_eligible(id);
            }
        }
        resources.opaque_reads = resources.accesses(&self.opaque_reads);
        Ok(resources)
    }
}

#[cfg(test)]
mod tests;
