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

/// One named compute pass that answers posts independently of the drawn view.
#[derive(Clone, Debug, PartialEq)]
pub struct TickPass {
    /// Diagnostic name, unique within this contribution.
    pub name: String,
    /// Reads from frame-owned resources. Query inputs and outputs are owned
    /// separately by the contribution and do not appear in the frame graph.
    pub reads: Vec<EffectAccess>,
}

/// Effect resource indices for the scene's GPU Sun-path target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SunPathResources {
    /// Optical-depth texture sampled by opaque shading.
    pub optical_depth: usize,
    /// Mapping buffer read by opaque shading.
    pub mapping: usize,
}

/// Effect resource indices for the medium target and optional ground band.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewVolumeResources {
    /// The provider's medium coefficients sampled during integration.
    pub coefficients: usize,
    /// A ground band texture filled alongside those coefficients.
    pub ground_band: Option<usize>,
}

/// Resources and simulate passes declared once during preparation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EffectGraph {
    /// Physical descriptors and their retained ownership scopes.
    pub resources: Vec<(ResourceDescriptor, ResourceScope)>,
    /// Ordered simulate passes.
    pub passes: Vec<EffectPass>,
    /// Ordered compute passes executed outside the frame.
    pub tick_passes: Vec<TickPass>,
    /// Resources read by the fixed opaque draw callback. Late-only outputs stay out of forward.
    pub opaque_reads: Vec<EffectAccess>,
    /// The provider's Sun-path target, if an effect writes it. At most one pair
    /// is allowed. Both resources must also have their full opaque read declared.
    pub sun_paths: Vec<SunPathResources>,
    /// The scene-owned medium coefficient array filled by an effect.
    pub view_volume_media: Vec<ViewVolumeResources>,
    /// Render passes after the sky and stars, in contribution order.
    pub after_opaque: Vec<super::AfterOpaquePass>,
}

pub(crate) struct EffectResources {
    pub(crate) opaque_reads: Vec<Access>,
    pub(crate) sun_path_reads: Option<[Access; 2]>,
    pub(crate) medium_read: Option<Access>,
    pub(crate) ground_band_read: Option<Access>,
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
    fn sun_path_read_indices(&self) -> Result<Option<[usize; 2]>, crate::error::EffectGraphError> {
        use crate::error::EffectGraphError as Error;
        let path = match self.sun_paths.as_slice() {
            [] => return Ok(None),
            [path] => path,
            _ => return Err(Error::DuplicateSunPath),
        };
        let read = |resource, kind, range: Range<u64>| {
            self.opaque_reads
                .iter()
                .position(|access| {
                    access.resource == resource
                        && access.kind == kind
                        && access.range == range
                        && access.view.is_none()
                        && access.history.is_none()
                })
                .ok_or(Error::InvalidSunPathReads)
        };
        let mapping_bytes = size_of::<crate::atmosphere::GpuSunMapping>() as u64;
        Ok(Some([
            read(path.optical_depth, AccessKind::SampledRead, 0..1)?,
            read(path.mapping, AccessKind::UniformRead, 0..mapping_bytes)?,
        ]))
    }

    fn medium_read(&self) -> Result<Option<EffectAccess>, crate::error::EffectGraphError> {
        use crate::error::EffectGraphError as Error;
        let resource = match self.view_volume_media.as_slice() {
            [] => return Ok(None),
            [provider] => provider.coefficients,
            _ => return Err(Error::InvalidViewVolumeMedium),
        };
        let Some((ResourceDescriptor::Texture { layers, .. }, _)) = self.resources.get(resource)
        else {
            return Err(Error::InvalidViewVolumeMedium);
        };
        let range = 0..u64::from(*layers);
        if !self.passes.iter().any(|pass| {
            pass.accesses.iter().any(|access| {
                access.resource == resource
                    && access.range == range
                    && access.kind == AccessKind::StorageWrite
                    && access.history.is_none()
            })
        }) {
            return Err(Error::InvalidViewVolumeMedium);
        }
        Ok(Some(EffectAccess {
            resource,
            range,
            view: Some(wgpu::TextureViewDimension::D2Array),
            kind: AccessKind::SampledRead,
            history: None,
        }))
    }

    fn ground_band_read_index(&self) -> Result<Option<usize>, crate::error::EffectGraphError> {
        let [provider] = self.view_volume_media.as_slice() else {
            return Ok(None);
        };
        let Some(resource) = provider.ground_band else {
            return Ok(None);
        };
        let read = self
            .opaque_reads
            .iter()
            .position(|access| {
                access.resource == resource
                    && access.range == (0..1)
                    && access.kind == AccessKind::SampledRead
                    && access.history.is_none()
            })
            .ok_or(crate::error::EffectGraphError::InvalidViewVolumeMedium)?;
        Ok(Some(read))
    }

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

        self.sun_path_read_indices()?;
        self.medium_read()?;
        self.ground_band_read_index()?;
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
        for (index, pass) in self.tick_passes.iter().enumerate() {
            if !names.insert(&pass.name) {
                return Err(Error::DuplicateEffectPass(pass.name.clone()));
            }
            for read in &pass.reads {
                let (descriptor, scope) = self
                    .resources
                    .get(read.resource)
                    .ok_or(Error::UnknownEffectResource)?;
                if !matches!(
                    read.kind,
                    AccessKind::SampledRead | AccessKind::UniformRead | AccessKind::StorageRead
                ) || !matches!(
                    scope,
                    ResourceScope::Persistent { .. } | ResourceScope::History { .. }
                ) {
                    return Err(Error::TickPassRequiresRetainedRead(pass.name.clone()));
                }
                match scope {
                    ResourceScope::History { .. }
                        if read.history != Some(HistoryBinding::Previous) =>
                    {
                        return Err(Error::TickPassRequiresPrevious(pass.name.clone()));
                    }
                    ResourceScope::Persistent { .. } if read.history.is_some() => {
                        return Err(Error::HistoryBindingOnNonHistory);
                    }
                    _ => {}
                }
                descriptor
                    .validate_access(&read.range, read.kind, read.view, index)
                    .map_err(|source| Error::InvalidTickRead {
                        pass: pass.name.clone(),
                        source,
                    })?;
            }
        }
        Ok(())
    }

    pub(crate) fn validate_2d_tick_stage(&self) -> Result<(), crate::error::EffectGraphError> {
        if let Some(pass) = self.tick_passes.first() {
            return Err(crate::error::EffectGraphError::TickPassIn2d(
                pass.name.clone(),
            ));
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
            sun_path_reads: None,
            medium_read: None,
            ground_band_read: None,
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
        resources.sun_path_reads = self
            .sun_path_read_indices()?
            .map(|indices| indices.map(|index| resources.opaque_reads[index].clone()));
        if let Some(read) = self.medium_read()? {
            resources.medium_read = resources.accesses(&[read]).pop();
        }
        resources.ground_band_read = self
            .ground_band_read_index()?
            .map(|index| resources.opaque_reads[index].clone());
        Ok(resources)
    }
}

#[cfg(test)]
mod tests;
