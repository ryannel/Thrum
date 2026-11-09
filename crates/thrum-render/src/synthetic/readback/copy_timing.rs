//! A dependency-ordered GPU upper bracket for a blit on stage-counter GPUs.
use crate::{
    capabilities::DeviceCapabilities,
    timing::{GpuTimings, TimingError},
};

const BEFORE: &str = "readback dependency before copy";
const AFTER: &str = "readback dependency after copy";

pub(in crate::synthetic) struct CopyTiming {
    pub(super) timings: GpuTimings,
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
}

impl CopyTiming {
    pub(super) fn new(
        cap: &DeviceCapabilities,
        queue: &wgpu::Queue,
        buffer: &wgpu::Buffer,
    ) -> Result<Option<Self>, TimingError> {
        if !cap
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS)
        {
            return Ok(None);
        }
        let device = cap.site();
        let _transition = device.transition("capture-copy-timing");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("readback dependency marker"),
            source: wgpu::ShaderSource::Wgsl(include_str!("copy_marker.wgsl").into()),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("readback dependency marker"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let sink = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback dependency marker sink"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("readback dependency marker"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sink.as_entire_binding(),
                },
            ],
        });
        Ok(Some(Self {
            timings: GpuTimings::new(cap, queue, 2)?,
            pipeline,
            group,
        }))
    }

    pub(super) fn marker(
        &mut self,
        generation: u64,
        encoder: &mut wgpu::CommandEncoder,
        after: bool,
    ) -> Result<(), TimingError> {
        let writes = self
            .timings
            .compute_writes(generation, if after { AFTER } else { BEFORE })?;
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("readback dependency marker"),
            timestamp_writes: writes,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
        Ok(())
    }

    pub(super) fn collect(&mut self, generation: u64) -> Result<std::task::Poll<f64>, TimingError> {
        let Some(passes) = self.timings.try_collect(generation)? else {
            return Ok(std::task::Poll::Pending);
        };
        let after = passes[1].nanoseconds;
        let gap = self
            .timings
            .gap_between(BEFORE, AFTER)
            .ok_or(TimingError::InvalidCapacity)?;
        // Before-marker end -> after-marker end. The first marker reads the
        // buffer before the blit overwrites it; the second reads the result.
        // RAW/WAR hazards order real GPU work, so empty-pass elision/overlap
        // cannot fabricate a copy time. Includes marker/encoder overhead.
        Ok(std::task::Poll::Ready(gap + after))
    }
}
