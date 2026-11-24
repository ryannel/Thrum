//! A dependency-ordered GPU upper bracket for a blit on stage-counter GPUs.
use crate::{
    capabilities::DeviceCapabilities,
    timing::{GpuTimings, TimingError},
};

const BEFORE: &str = "readback dependency before copy";
const AFTER: &str = "readback dependency after copy";
const WORD_BYTES: u64 = 4;

pub(crate) struct CopyTiming {
    pub(super) timings: GpuTimings,
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    input: wgpu::Buffer,
    output: wgpu::Buffer,
}

impl CopyTiming {
    pub(crate) fn prepare(
        cap: &DeviceCapabilities,
        queue: &wgpu::Queue,
        slots: usize,
    ) -> Result<Vec<Self>, TimingError> {
        if !cap
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS)
        {
            return Ok(Vec::new());
        }
        let device = cap.site();
        let _transition = device.transition("capture-copy-timing");
        let shader =
            crate::shaders::Shader::CopyMarker.compile(device, "readback dependency marker");
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("readback dependency marker"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        (0..slots)
            .map(|_| Self::new(cap, queue, &pipeline))
            .collect()
    }

    fn new(
        cap: &DeviceCapabilities,
        queue: &wgpu::Queue,
        pipeline: &wgpu::ComputePipeline,
    ) -> Result<Self, TimingError> {
        let device = cap.site();
        let input = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback dependency marker input"),
            size: WORD_BYTES,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback dependency marker sink"),
            size: WORD_BYTES,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("readback dependency marker"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        Ok(Self {
            timings: GpuTimings::new(cap, queue, 2)?,
            pipeline: pipeline.clone(),
            group,
            input,
            output,
        })
    }

    pub(super) fn marker(
        &mut self,
        generation: u64,
        encoder: &mut wgpu::CommandEncoder,
        staging: &wgpu::Buffer,
        after: bool,
    ) -> Result<(), TimingError> {
        let writes = self.timings.compute_writes(
            generation,
            if after { AFTER } else { BEFORE },
            crate::timing::TimingCategory::Transfer,
        )?;
        // Keep the dependency on the first staging word while allowing the
        // marker's bind group and queries to survive a change of pixel grid.
        if after {
            encoder.copy_buffer_to_buffer(staging, 0, &self.input, 0, WORD_BYTES);
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("readback dependency marker"),
            timestamp_writes: writes,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
        drop(pass);
        if !after {
            // The following texture copy overwrites this word before mapping.
            encoder.copy_buffer_to_buffer(&self.output, 0, staging, 0, WORD_BYTES);
        }
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
        // Before-marker end -> after-marker end. Includes the two one-word
        // transfers that order the markers around the pixel copy.
        Ok(std::task::Poll::Ready(gap + after))
    }
}
