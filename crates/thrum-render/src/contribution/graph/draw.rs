//! Preparation rules for raster passes in the simulate stage.

use super::{EffectCadence, EffectGraph};
use crate::{
    error::EffectGraphError,
    graph::{AccessKind, ResourceDescriptor, ResourceScope, TextureKind},
};

/// A simulate pass's prepared colour view and clear value.
#[derive(Clone)]
pub struct EffectDrawTarget {
    pub(crate) view: wgpu::TextureView,
    pub(crate) clear: wgpu::Color,
    descriptor: ResourceDescriptor,
    unit: u64,
    blend: bool,
}

impl EffectDrawTarget {
    /// Prepare a single mip and layer of an effect-owned colour texture. Supply the same
    /// flattened mip/layer index in the graph access and the same blend choice in the pipeline.
    /// Views of the same declared resource must use the same backing texture.
    ///
    /// # Errors
    /// Refuses an unsupported attachment format, blend mode or subresource, or a
    /// memoryless texture whose contents cannot be stored by the executor.
    pub fn new(
        pass: &str,
        texture: &wgpu::Texture,
        mip: u32,
        layer: u32,
        clear: wgpu::Color,
        blend: bool,
        surface: &crate::contribution::SceneSurface<'_>,
    ) -> Result<Self, EffectGraphError> {
        if texture
            .usage()
            .contains(wgpu::TextureUsages::TRANSIENT_ATTACHMENT)
        {
            return Err(draw_error(
                pass,
                "memoryless colour attachments are unsupported",
            ));
        }
        let format = texture.format();
        check_draw_format(pass, format, surface.texture_format(format), blend)?;
        if texture.dimension() != wgpu::TextureDimension::D2
            || texture.sample_count() != 1
            || !texture
                .usage()
                .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
            || mip >= texture.mip_level_count()
            || layer >= texture.depth_or_array_layers()
        {
            return Err(draw_error(
                pass,
                "expected one mip and layer of a single-sample 2D colour attachment",
            ));
        }
        Ok(Self {
            view: texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some(pass),
                dimension: Some(wgpu::TextureViewDimension::D2),
                usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
                base_mip_level: mip,
                mip_level_count: Some(1),
                base_array_layer: layer,
                array_layer_count: Some(1),
                ..Default::default()
            }),
            clear,
            descriptor: ResourceDescriptor::of_texture(texture),
            unit: u64::from(mip) * u64::from(texture.depth_or_array_layers()) + u64::from(layer),
            blend,
        })
    }
}

pub(super) fn draw_error(pass: &str, reason: &'static str) -> EffectGraphError {
    EffectGraphError::SimulateDraw {
        pass: pass.to_owned(),
        reason,
    }
}

pub(super) fn check_draw_format(
    pass: &str,
    format: wgpu::TextureFormat,
    facts: wgpu::TextureFormatFeatures,
    blend: bool,
) -> Result<(), EffectGraphError> {
    if format.is_depth_stencil_format()
        || !facts
            .allowed_usages
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
    {
        return Err(draw_error(
            pass,
            "format is not supported as a colour attachment",
        ));
    }
    if blend
        && !facts
            .flags
            .contains(wgpu::TextureFormatFeatureFlags::BLENDABLE)
    {
        return Err(draw_error(
            pass,
            "colour attachment format does not support blending",
        ));
    }
    Ok(())
}

impl EffectGraph {
    pub(super) fn validate_simulate_draws(&self) -> Result<(), EffectGraphError> {
        for (index, pass) in self.passes.iter().enumerate() {
            if pass
                .accesses
                .iter()
                .any(|access| access.resource >= self.resources.len())
            {
                return Err(EffectGraphError::UnknownSimulateResource(pass.name.clone()));
            }
            let mut attachments = pass.attachments();
            let Some(attachment) = attachments.next() else {
                continue;
            };
            // Keep the established refusal for declarations that violate the shared
            // async or loaded-cadence rules before checking the draw-specific rules.
            if pass.async_eligible {
                return Err(EffectGraphError::AsyncRequiresCompute(pass.name.clone()));
            }
            if attachment.kind == AccessKind::AttachmentLoad
                && pass.cadence != EffectCadence::EveryFrame
            {
                return Err(EffectGraphError::CadenceRequiresRetainedOutput(
                    pass.name.clone(),
                ));
            }
            let reject = |reason| draw_error(&pass.name, reason);
            if attachments.next().is_some() || attachment.kind != AccessKind::AttachmentClear {
                return Err(reject("requires one cleared colour attachment"));
            }
            let (descriptor, scope) = &self.resources[attachment.resource];
            let ResourceDescriptor::Texture {
                dimension: TextureKind::D2,
                samples: 1,
                format,
                ..
            } = descriptor
            else {
                return Err(reject("requires a single-sample 2D or 2D-array texture"));
            };
            if format.is_depth_stencil_format()
                || !matches!(
                    scope,
                    ResourceScope::Persistent { .. } | ResourceScope::Transient
                )
                || attachment.history.is_some()
            {
                return Err(reject(
                    "requires an owned persistent or transient colour texture",
                ));
            }
            if attachment.range.end.checked_sub(attachment.range.start) != Some(1)
                || attachment
                    .view
                    .is_some_and(|view| view != wgpu::TextureViewDimension::D2)
                || descriptor
                    .validate_access(
                        &attachment.range,
                        attachment.kind,
                        Some(wgpu::TextureViewDimension::D2),
                        index,
                    )
                    .is_err()
            {
                return Err(reject("requires a view of exactly one mip and layer"));
            }
            if pass.cadence == EffectCadence::TimeSliced {
                return Err(reject("cannot be time-sliced"));
            }
            if pass.accesses.iter().any(|access| {
                matches!(
                    access.kind,
                    AccessKind::CopyRead | AccessKind::CopyWrite | AccessKind::AccelerationBuild
                )
            }) {
                return Err(reject("cannot declare copy or acceleration build work"));
            }
            // Capture reruns transient draws and reuses persistent colour. All writes
            // must follow that choice so capture preserves state and produces scratch inputs.
            let transient_colour = matches!(scope, ResourceScope::Transient);
            for access in &pass.accesses {
                if matches!(
                    access.kind,
                    AccessKind::StorageWrite | AccessKind::StorageReadWrite
                ) && transient_colour
                    != matches!(self.resources[access.resource].1, ResourceScope::Transient)
                {
                    return Err(reject(if transient_colour {
                        "a draw with a transient colour attachment cannot write retained storage"
                    } else {
                        "a draw with a persistent colour attachment cannot write transient storage"
                    }));
                }
            }
            let has_other_writer = self
                .declarations()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .flat_map(|(_, (_, _, _, accesses))| accesses)
                .any(|access| {
                    access.resource == attachment.resource
                        && access.range.start < attachment.range.end
                        && attachment.range.start < access.range.end
                        && matches!(
                            access.kind,
                            AccessKind::StorageWrite
                                | AccessKind::StorageReadWrite
                                | AccessKind::CopyWrite
                                | AccessKind::AttachmentClear
                                | AccessKind::AttachmentLoad
                        )
                });
            if has_other_writer {
                return Err(reject("attachment overlaps another pass's write"));
            }
        }
        Ok(())
    }

    // The graph has structurally validated attachments before capability admission.
    pub(crate) fn validate_draw_formats(
        &self,
        capabilities: &crate::capabilities::DeviceCapabilities,
    ) -> Result<(), EffectGraphError> {
        for pass in &self.passes {
            if let Some(attachment) = pass.attachments().next()
                && let ResourceDescriptor::Texture { format, .. } =
                    self.resources[attachment.resource].0
            {
                check_draw_format(
                    &pass.name,
                    format,
                    capabilities.texture_format(format),
                    false,
                )?;
            }
        }
        Ok(())
    }

    // Called only after structural validation of the graph.
    pub(crate) fn validate_draw_targets<'a>(
        &self,
        capabilities: &crate::capabilities::DeviceCapabilities,
        target: impl Fn(usize) -> Option<&'a EffectDrawTarget>,
    ) -> Result<(), EffectGraphError> {
        let mut written = Vec::new();
        for (index, pass) in self.passes.iter().enumerate() {
            let Some(attachment) = pass.attachments().next() else {
                continue;
            };
            let descriptor = &self.resources[attachment.resource].0;
            let ResourceDescriptor::Texture { format, .. } = descriptor else {
                unreachable!()
            };
            let target = target(index);
            check_draw_format(
                &pass.name,
                *format,
                capabilities.texture_format(*format),
                target.is_some_and(|target| target.blend),
            )?;
            let target =
                target.ok_or_else(|| draw_error(&pass.name, "missing prepared colour view"))?;
            if target.descriptor != *descriptor || target.unit != attachment.range.start {
                return Err(draw_error(
                    &pass.name,
                    "prepared colour view differs from the declaration",
                ));
            }
            let texture = target.view.texture();
            for &(resource, backing, unit) in &written {
                if backing == texture {
                    if unit == target.unit {
                        return Err(draw_error(
                            &pass.name,
                            "prepared colour subresource overlaps another pass's write",
                        ));
                    }
                } else if resource == attachment.resource {
                    return Err(draw_error(
                        &pass.name,
                        "prepared colour views of one resource must share a backing texture",
                    ));
                }
            }
            written.push((attachment.resource, texture, target.unit));
        }
        Ok(())
    }
}
