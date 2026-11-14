//! Preparation-time restrictions for a whole-encoder native segment.
use super::{AccessKind, FrameGraph, GraphError, Pass, ResourceId, ResourceScope};

impl FrameGraph {
    /// Declare a native segment with the same hazard and lifetime analysis as
    /// ordinary passes. The executor must record it on a dedicated whole raw
    /// encoder, between wgpu encoders, in one ordered coordinator submission.
    ///
    /// Written resources must be initialized through wgpu, retained externally
    /// through completion, and never aliased or discarded. A graph declaration
    /// does not initialize a physical allocation or transfer its ownership.
    pub fn native_pass(&mut self, pass: Pass) -> usize {
        let index = self.pass(pass);
        self.native_passes.insert(index);
        index
    }

    pub(super) fn validate_native_resources(&self) -> Result<(), GraphError> {
        for &index in &self.native_passes {
            for access in &self.passes[index].accesses {
                let resource = self
                    .resources
                    .get(access.resource.index)
                    .filter(|_| std::sync::Arc::ptr_eq(&access.resource.owner, &self.owner))
                    .ok_or(GraphError::InvalidRange { pass: index })?;
                if matches!(
                    access.kind,
                    AccessKind::AttachmentClear | AccessKind::AttachmentLoad
                ) || (access.kind.writes()
                    && (resource.scope == ResourceScope::Transient
                        || !self.initial_contents(access.resource.index)))
                {
                    return Err(GraphError::NativeResource { pass: index });
                }
            }
        }
        Ok(())
    }

    pub(super) fn validate_native_surface(&self, surface: &ResourceId) -> Result<(), GraphError> {
        for &index in &self.native_passes {
            if self.passes[index]
                .accesses
                .iter()
                .any(|access| access.resource == *surface)
            {
                return Err(GraphError::NativeSurface { pass: index });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
