//! The one small render target a GPU test draws to.
//!
//! [`new`] returns a [`SIZE`]-square colour target in [`FORMAT`], with a [`DEPTH_FORMAT`] depth target where the
//! test asks for one, in wgpu types alone. A test that drives a host passes [`SIZE`] to it, so the host renders at
//! the size the test reads back. A test whose subject is a size, such as a resize, keeps its own target.

/// Width and height of the target in texels.
pub const SIZE: u32 = 64;

/// Format of the colour target: four bytes a texel, which [`Target::read`] returns tightly packed.
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Format of the depth target.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// The target's extent, one layer of [`SIZE`] by [`SIZE`].
pub const EXTENT: wgpu::Extent3d = wgpu::Extent3d {
    width: SIZE,
    height: SIZE,
    depth_or_array_layers: 1,
};

/// Whether [`new`] makes a depth target beside the colour target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
    /// Colour only.
    Without,
    /// A [`DEPTH_FORMAT`] depth target as well.
    With,
}

/// A texture of the target and its default view.
pub struct Attachment {
    /// The texture, usable as a render attachment, a copy source and a sampled binding.
    pub texture: wgpu::Texture,
    /// Its default view, for a render pass attachment.
    pub view: wgpu::TextureView,
}

/// The colour target, and the depth target where one was asked for.
pub struct Target {
    /// The [`FORMAT`] colour target.
    pub colour: Attachment,
    /// The [`DEPTH_FORMAT`] depth target, with [`Depth::With`].
    pub depth: Option<Attachment>,
}

fn attachment(device: &wgpu::Device, label: &str, format: wgpu::TextureFormat) -> Attachment {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: EXTENT,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    Attachment { texture, view }
}

/// Make the test's target on `device`: [`SIZE`] square, in [`FORMAT`], with a depth target where `depth` asks.
#[must_use]
pub fn new(device: &wgpu::Device, depth: Depth) -> Target {
    Target {
        colour: attachment(device, "test target colour", FORMAT),
        depth: (depth == Depth::With)
            .then(|| attachment(device, "test target depth", DEPTH_FORMAT)),
    }
}

impl Target {
    /// Read the colour target back through the kit's readback: [`SIZE`] rows of [`SIZE`] texels, four bytes each.
    ///
    /// # Panics
    /// As [`crate::readback::texture`].
    #[must_use]
    pub fn read(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        crate::readback::texture(
            device,
            queue,
            &self.colour.texture,
            crate::readback::Mip::level(0),
            ..,
        )
    }
}

// Inline because the repository ignores every folder named `target`.
#[cfg(test)]
mod tests {
    use super::{DEPTH_FORMAT, Depth, EXTENT, FORMAT, SIZE};

    /// Draws a quad over the left half of clip space, red at depth 0.25.
    const SHADER: &str = "
    @vertex
    fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
        var corners = array<vec2<f32>, 6>(
            vec2(-1.0, -1.0), vec2(0.0, -1.0), vec2(-1.0, 1.0),
            vec2(-1.0, 1.0), vec2(0.0, -1.0), vec2(0.0, 1.0),
        );
        return vec4(corners[index], 0.25, 1.0);
    }

    @fragment
    fn fs() -> @location(0) vec4<f32> {
        return vec4(1.0, 0.0, 0.0, 1.0);
    }
    ";

    const RED: [u8; 4] = [255, 0, 0, 255];
    const CLEAR: [u8; 4] = [0, 0, 0, 255];

    fn device(test: &str) -> Option<(wgpu::Device, wgpu::Queue)> {
        let gpu = crate::gpu(test, None)?;
        Some(
            pollster::block_on(
                gpu.adapter
                    .request_device(&wgpu::DeviceDescriptor::default()),
            )
            .expect("target test device"),
        )
    }

    fn pipeline(device: &wgpu::Device, depth: bool) -> wgpu::RenderPipeline {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("target test"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("target test"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: depth.then(|| wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(FORMAT.into())],
            }),
            multiview_mask: None,
            cache: None,
        })
    }

    /// Clear the target to opaque black, depth to 1, and draw the left-half quad.
    fn draw(device: &wgpu::Device, queue: &wgpu::Queue, target: &super::Target) {
        let pipeline = pipeline(device, target.depth.is_some());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("target test"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.colour.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: target.depth.as_ref().map(|depth| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view: &depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.draw(0..6, 0..1);
        }
        queue.submit([encoder.finish()]);
    }

    /// Every texel of `texels` (`SIZE` rows of `SIZE`, `N` bytes each) is `left` in the left half and `right` in the
    /// right, so a target of another size or a draw that wrote nothing fails.
    fn assert_halves<const N: usize>(what: &str, texels: &[u8], left: [u8; N], right: [u8; N]) {
        let size = usize::try_from(SIZE).unwrap();
        assert_eq!(
            texels.len(),
            size * size * N,
            "{what}: bytes for {SIZE}x{SIZE}"
        );
        for (index, texel) in texels.as_chunks::<N>().0.iter().enumerate() {
            let (row, column) = (index / size, index % size);
            let expected = if column < size / 2 { left } else { right };
            assert_eq!(*texel, expected, "{what} at row {row}, column {column}");
        }
    }

    #[test]
    fn target_is_the_kit_size_and_reads_back_a_draw() {
        let test = "target_is_the_kit_size_and_reads_back_a_draw";
        let Some((device, queue)) = device(test) else {
            return;
        };
        let target = super::new(&device, Depth::Without);
        assert!(target.depth.is_none());
        assert_eq!(target.colour.texture.size(), EXTENT);
        assert_eq!(target.colour.texture.format(), FORMAT);
        draw(&device, &queue, &target);
        assert_halves("colour", &target.read(&device, &queue), RED, CLEAR);
    }

    #[test]
    fn target_depth_is_the_kit_size_and_holds_the_draw() {
        let test = "target_depth_is_the_kit_size_and_holds_the_draw";
        let Some((device, queue)) = device(test) else {
            return;
        };
        let target = super::new(&device, Depth::With);
        let depth = target.depth.as_ref().expect("depth target asked for");
        assert_eq!(depth.texture.size(), EXTENT);
        assert_eq!(depth.texture.format(), DEPTH_FORMAT);
        draw(&device, &queue, &target);
        assert_halves("colour", &target.read(&device, &queue), RED, CLEAR);
        let depths = crate::readback::texture(
            &device,
            &queue,
            &depth.texture,
            crate::readback::Mip::level(0),
            ..,
        );
        assert_halves(
            "depth",
            &depths,
            0.25_f32.to_le_bytes(),
            1.0_f32.to_le_bytes(),
        );
    }
}
