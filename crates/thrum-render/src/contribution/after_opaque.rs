//! Prepared inputs shared by late scene effects. No allocation occurs in a frame.

/// One render pass after opaque, sky and stars and before reconstruction and tone.
/// List order is execution order; medium integration remains the consumer's policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AfterOpaquePass {
    /// Executor timing name, unique within the effect.
    pub name: String,
    /// Sample the current frame's reverse-Z depth at group 1, binding 0.
    pub depth: bool,
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
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                });
            }
        }
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&self.name),
            entries: &entries,
        })
    }
}
