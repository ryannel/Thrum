use crate::binding;
use crate::contribution::{QualityInvalidation, QualityKnob, QualitySettings};
use crate::records::LampShadowQuality;

pub(in crate::synthetic) struct LampShadows {
    pub(in crate::synthetic) settings: QualitySettings,
    quality: wgpu::Buffer,
    unshadowed: wgpu::Buffer,
}

impl LampShadows {
    pub(in crate::synthetic) const KNOB: &str = "lamp shadow rays";

    pub(in crate::synthetic) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let capacity =
            u32::try_from(crate::synthetic::cluster::CAPACITY).expect("small cluster capacity");
        let settings = QualitySettings::new(vec![QualityKnob {
            name: Self::KNOB.into(),
            effect: "Maximum shadow rays per fragment, in lamps",
            value: capacity,
            range: 0..=capacity,
            invalidation: QualityInvalidation::ViewHistory,
            governor_steps: None,
        }])
        .expect("valid lamp shadow control");
        let quality = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lamp shadow quality"),
            size: size_of::<LampShadowQuality>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let unshadowed = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("unshadowed lamp references"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        queue.write_buffer(
            &quality,
            0,
            bytemuck::bytes_of(&LampShadowQuality {
                ray_limit: capacity,
            }),
        );
        Self {
            settings,
            quality,
            unshadowed,
        }
    }

    pub(in crate::synthetic) fn commit(&mut self, queue: &wgpu::Queue) -> QualityInvalidation {
        let invalidation = self.settings.commit();
        if invalidation != QualityInvalidation::None {
            queue.write_buffer(
                &self.quality,
                0,
                bytemuck::bytes_of(&LampShadowQuality {
                    ray_limit: self.settings.knobs()[0].value,
                }),
            );
        }
        invalidation
    }

    pub(in crate::synthetic) fn unshadowed(&self) -> &wgpu::Buffer {
        &self.unshadowed
    }

    pub(in crate::synthetic) fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 2] {
        [
            binding::uniform(
                16,
                wgpu::ShaderStages::FRAGMENT,
                size_of::<LampShadowQuality>() as u64,
            ),
            binding::storage(17, wgpu::ShaderStages::FRAGMENT, false, 4),
        ]
    }

    pub(in crate::synthetic) fn bindings(&self) -> [wgpu::BindGroupEntry<'_>; 2] {
        [
            wgpu::BindGroupEntry {
                binding: 16,
                resource: self.quality.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 17,
                resource: self.unshadowed.as_entire_binding(),
            },
        ]
    }
}
