//! Prepared inputs shared by late scene effects. No allocation occurs in a frame.

use crate::binding;

/// Work recorded after opaque, sky and stars.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterOpaqueKind {
    /// Load scene colour, motion and reactive targets and draw into them.
    Render,
    /// Read requested scene inputs and write declared effect-local resources.
    Compute,
}

/// One pass after opaque, sky and stars and before reconstruction and tone.
/// List order is execution order; medium integration remains the consumer's policy.
#[derive(Clone, Debug, PartialEq)]
pub struct AfterOpaquePass {
    /// Executor timing name, unique within the effect.
    pub name: String,
    /// Render composition or compute into effect-owned outputs.
    pub kind: AfterOpaqueKind,
    /// Runtime scheduling class. Render passes load attachments and must run every frame.
    pub cadence: super::EffectCadence,
    /// Effect-local resource hazards, including history previous/next/result bindings.
    pub accesses: Vec<super::EffectAccess>,
    /// Sample the current frame's reverse-Z depth at group 1, binding 0.
    pub depth: bool,
    /// Attach and store scene depth for a displaced surface. Cannot also sample it.
    pub write_depth: bool,
    /// Sample one shared pre-effect scene-colour copy at group 1, binding 1.
    pub refraction: bool,
}

impl AfterOpaquePass {
    /// Explicit group 1 layout used by this pass. Group 0 is the scene view layout.
    /// Create the pipeline in `rebuild`; the renderer prepares matching target bindings.
    #[must_use]
    pub fn input_layout(&self, device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let mut entries = Vec::new();
        for (enabled, binding, sample_type) in [
            (self.depth, 0, wgpu::TextureSampleType::Depth),
            (
                self.refraction,
                1,
                wgpu::TextureSampleType::Float { filterable: false },
            ),
        ] {
            if enabled {
                entries.push(binding::texture(
                    binding,
                    wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                    sample_type,
                    wgpu::TextureViewDimension::D2,
                ));
            }
        }
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&self.name),
            entries: &entries,
        })
    }
}
