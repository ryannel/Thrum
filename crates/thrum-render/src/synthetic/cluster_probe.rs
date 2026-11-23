//! Optional read-only diagnostic of the actual prepared cluster buffers.
use super::{ClusterTable, PreparedSynthetic, RenderError, SyntheticScene};
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// Bounded diagnostic staging of actual GPU-visible head/index buffers against
/// the existing CPU oracle. It never changes the renderer's buffer usages.
pub struct LightListProbe {
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    output: wgpu::Buffer,
    staging: wgpu::Buffer,
    expected: Vec<u8>,
    words: u32,
    pending: Option<Receiver<Result<(), wgpu::BufferAsyncError>>>,
}

impl LightListProbe {
    /// Prepare a read-only observer for the scene used to prepare `prepared`.
    /// The CPU oracle covers every cluster, including ranking and tie order.
    /// # Errors
    /// Refuses an invalid fixture or a readback larger than the device limit.
    pub fn new(prepared: &PreparedSynthetic, scene: &SyntheticScene) -> Result<Self, RenderError> {
        let oracle = ClusterTable::build(&scene.local_lights, &scene.view)?;
        let mut expected = oracle.heads;
        expected.extend_from_slice(&oracle.indices);
        let size = u64::try_from(expected.len()).map_err(|_| RenderError::ClusterProbeCapacity)?;
        let words = u32::try_from(size / 4).map_err(|_| RenderError::ClusterProbeCapacity)?;
        let site = prepared.capabilities.site();
        if size
            > prepared
                .capabilities
                .limits()
                .max_storage_buffer_binding_size
        {
            return Err(RenderError::ClusterProbeCapacity);
        }
        let output = site.create_buffer(&wgpu::BufferDescriptor {
            label: Some("light list probe output"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = site.create_buffer(&wgpu::BufferDescriptor {
            label: Some("light list probe readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let shader = crate::shaders::Shader::ClusterProbe.compile(site, "light list probe");
        let pipeline = site.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("light list probe"),
            layout: None,
            module: &shader,
            entry_point: Some("copy_lists"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let group = site.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("light list probe"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: prepared.cluster_head_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: prepared.cluster_index_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        Ok(Self {
            pipeline,
            group,
            output,
            staging,
            expected,
            words,
            pending: None,
        })
    }

    /// Number of head/index bytes checked against the CPU oracle.
    #[must_use]
    pub fn checked_bytes(&self) -> usize {
        self.expected.len()
    }

    /// Append the diagnostic after the actual scene encoder, on the same queue.
    /// The caller owns submission; the probe neither advances history nor submits.
    /// # Errors
    /// Refuses reuse before collection.
    pub fn encode(&mut self, encoder: &mut wgpu::CommandEncoder) -> Result<(), RenderError> {
        if self.pending.is_some() {
            return Err(RenderError::Readback);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("light list probe"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.group, &[]);
            pass.dispatch_workgroups(
                self.words.div_ceil(crate::shaders::CLUSTER_PROBE_WORKGROUP),
                1,
                1,
            );
        }
        encoder.copy_buffer_to_buffer(&self.output, 0, &self.staging, 0, self.output.size());
        let (send, receive) = mpsc::channel();
        encoder.map_buffer_on_submit(&self.staging, wgpu::MapMode::Read, .., move |value| {
            let _ = send.send(value);
        });
        self.pending = Some(receive);
        Ok(())
    }

    /// Collect without blocking. `Some(false)` is an exact mismatch, never a tolerance.
    /// # Errors
    /// Reports mapping failure or a disconnected callback.
    pub fn try_collect(&mut self) -> Result<Option<bool>, RenderError> {
        let Some(receive) = &self.pending else {
            return Ok(None);
        };
        match receive.try_recv() {
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) | Ok(Err(_)) => Err(RenderError::Readback),
            Ok(Ok(())) => {
                let mapped = self
                    .staging
                    .slice(..)
                    .get_mapped_range()
                    .map_err(|_| RenderError::Readback)?;
                let equal = mapped[..] == self.expected;
                drop(mapped);
                self.staging.unmap();
                self.pending = None;
                Ok(Some(equal))
            }
        }
    }
}
