use crate::contribution::{QualityInvalidation, QualityKnob, QualitySettings};
use crate::records::LampShadowQuality;

pub(in crate::synthetic) fn structure_layout_entry() -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 15,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::AccelerationStructure {
            vertex_return: false,
        },
        count: None,
    }
}

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
            crate::synthetic::uniform_layout(
                16,
                size_of::<LampShadowQuality>() as u64,
                wgpu::ShaderStages::FRAGMENT,
            ),
            wgpu::BindGroupLayoutEntry {
                binding: 17,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(4),
                },
                count: None,
            },
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
