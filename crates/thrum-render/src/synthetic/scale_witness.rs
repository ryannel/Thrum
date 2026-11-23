//! Attachment and view record controls used by the render scale witness.
use super::{OffscreenTarget, PreparedSynthetic};

impl PreparedSynthetic {
    /// The exact view uniform region consumed by the pending forward draw.
    #[doc(hidden)]
    pub fn scale_view_witness(&self, target: &OffscreenTarget) -> (&wgpu::Buffer, u64) {
        (
            &self.view_buffer,
            u64::from(self.view_stride) * (target.frame_revision & 1),
        )
    }
}

impl OffscreenTarget {
    /// Install a marker pipeline that writes colour, motion, reactive and depth
    /// in the undrawn border immediately before reconstruction.
    #[doc(hidden)]
    pub fn set_border_witness(&mut self, pipeline: wgpu::RenderPipeline) {
        self.border_witness = Some(pipeline);
    }

    /// Render-extent attachment sizes including all TAAU inputs.
    #[doc(hidden)]
    pub fn scale_attachment_extents(&self) -> [[u32; 2]; 4] {
        [
            &self.depth,
            &self.scene_view,
            &self.motion_view,
            &self.reactive_view,
        ]
        .map(|view| [view.texture().width(), view.texture().height()])
    }

    /// The actual motion attachment written by the scene.
    #[doc(hidden)]
    pub fn scale_motion_witness(&self) -> &wgpu::Texture {
        self.motion_view.texture()
    }

    pub(super) fn record_border_witness(&self, encoder: &mut wgpu::CommandEncoder) {
        let Some(pipeline) = &self.border_witness else {
            return;
        };
        if self.drawn == self.extent() {
            return;
        }
        let [width, height] = self.extent();
        let [right, bottom] = self.drawn;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render scale attachment border witness"),
            color_attachments: &[&self.scene_view, &self.motion_view, &self.reactive_view].map(
                |view| {
                    Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })
                },
            ),
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        for [x, y, w, h] in [
            [right, 0, width - right, bottom],
            [0, bottom, width, height - bottom],
        ] {
            if w != 0 && h != 0 {
                pass.set_scissor_rect(x, y, w, h);
                pass.draw(0..3, 0..1);
            }
        }
    }
}
