//! Two GPU-retained transform samples. Physical instance slots are the index;
//! identity, generation and submitted serial reject history after compaction.

use crate::graph::CompiledPass;
use crate::timing::GpuTimings;

use super::{RenderError, abi, storage_layout, uniform_layout};

pub(super) const ENTRY_BYTES: u64 = 80;
pub(super) const SERIAL_BYTES: u64 = 32;
pub(super) const PASS_NAME: &str = "synthetic transform history";
const WORKGROUP: u32 = 64;

pub(super) struct TransformHistory {
    read_groups: [wgpu::BindGroup; 2],
    write_groups: [wgpu::BindGroup; 2],
    serial_buffer: wgpu::Buffer,
    pipeline: wgpu::ComputePipeline,
    read_side: usize,
    next_serial: u64,
    submitted_serial: u64,
    presentation_alpha: f32,
    extent: Option<[u32; 2]>,
}

impl TransformHistory {
    pub(super) fn new(
        device: &crate::runtime::DeviceSite,
        queue: &wgpu::Queue,
        instances: &wgpu::Buffer,
        mesh_vertices: &wgpu::Buffer,
        instance_layout: &wgpu::BindGroupLayout,
        count: u32,
    ) -> Self {
        let bytes = u64::from(count) * ENTRY_BYTES;
        let buffers = std::array::from_fn::<_, 2, _>(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("synthetic retained transform history"),
                size: bytes,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        });
        let serial_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("synthetic submitted history serials"),
            size: SERIAL_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let writer_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic history publisher layout"),
            entries: &[
                storage_layout(0, wgpu::ShaderStages::COMPUTE, abi::INSTANCE_BYTES as u64),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(ENTRY_BYTES),
                    },
                    count: None,
                },
                uniform_layout(2, SERIAL_BYTES, wgpu::ShaderStages::COMPUTE),
            ],
        });
        let read_groups = std::array::from_fn(|side| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("synthetic instance and previous transform"),
                layout: instance_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: instances.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buffers[side].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: serial_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: mesh_vertices.as_entire_binding(),
                    },
                ],
            })
        });
        let write_groups = std::array::from_fn(|side| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("synthetic history publisher"),
                layout: &writer_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: instances.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buffers[side].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: serial_buffer.as_entire_binding(),
                    },
                ],
            })
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("synthetic history publisher"),
            source: wgpu::ShaderSource::Wgsl(include_str!("history_gpu.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic history publisher"),
            bind_group_layouts: &[Some(&writer_layout)],
            immediate_size: 0,
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("synthetic history publisher"),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("publish"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let history = Self {
            read_groups,
            write_groups,
            serial_buffer,
            pipeline,
            read_side: 0,
            next_serial: 1,
            submitted_serial: 0,
            presentation_alpha: 1.0,
            extent: None,
        };
        history.write_serials(queue, false);
        history
    }

    pub(super) fn write_side(&self) -> usize {
        1 - self.read_side
    }

    pub(super) fn serial_buffer(&self) -> &wgpu::Buffer {
        &self.serial_buffer
    }

    pub(super) fn read_group(&self) -> &wgpu::BindGroup {
        &self.read_groups[self.read_side]
    }

    pub(super) fn reset(&mut self, queue: &wgpu::Queue) {
        self.submitted_serial = 0;
        self.write_serials(queue, false);
    }

    pub(super) fn set_extent(&mut self, queue: &wgpu::Queue, extent: [u32; 2]) {
        if self.extent != Some(extent) {
            self.extent = Some(extent);
            self.reset(queue);
        }
    }

    pub(super) fn prepare_sample(&mut self, queue: &wgpu::Queue, alpha: f32, reset: bool) {
        self.presentation_alpha = if alpha.is_finite() {
            alpha.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.write_serials(queue, reset);
    }

    fn write_serials(&self, queue: &wgpu::Queue, reset: bool) {
        // A prepared cut hides previous transforms for this sample alone.
        // A capture or abandoned encoder cannot retire the submitted serial.
        let read_serial = if reset { 0 } else { self.submitted_serial };
        let words = [
            u32::try_from(read_serial & u64::from(u32::MAX)).expect("masked serial fits u32"),
            u32::try_from(read_serial >> 32).expect("high serial word fits u32"),
            u32::try_from(self.next_serial & u64::from(u32::MAX)).expect("masked serial fits u32"),
            u32::try_from(self.next_serial >> 32).expect("high serial word fits u32"),
            self.presentation_alpha.to_bits(),
            0,
            0,
            0,
        ];
        let bytes: [u8; 32] = std::array::from_fn(|byte| words[byte / 4].to_le_bytes()[byte % 4]);
        queue.write_buffer(&self.serial_buffer, 0, &bytes);
    }

    pub(super) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        generation: u64,
        compiled: &CompiledPass,
        active_count: u32,
        timings: Option<&mut GpuTimings>,
    ) -> Result<(), RenderError> {
        if active_count == 0 {
            // Metal does not sample a compute pass without a dispatch.
            return Ok(());
        }
        let mut pass = compiled
            .begin_compute(encoder, generation, timings)?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.write_groups[self.write_side()], &[]);
        pass.dispatch_workgroups(active_count.div_ceil(WORKGROUP), 1, 1);
        Ok(())
    }

    pub(super) fn submitted(&mut self) {
        self.read_side = 1 - self.read_side;
        self.submitted_serial = self.next_serial;
        self.next_serial = self.next_serial.wrapping_add(1).max(1);
    }
}

#[cfg(test)]
mod tests;
