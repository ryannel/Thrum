//! Explicit Metal build observation. The marker passes bracket the production
//! build without sampling host wait or readback time.

use std::sync::{Arc, Mutex};

use super::PreparedSynthetic;

/// Opt-in observer for mesh structure builds recorded on one device.
#[derive(Clone)]
pub struct RayBuildProbe(Arc<Probe>);

struct Probe {
    device: wgpu::Device,
    marker: wgpu::ComputePipeline,
    binding: wgpu::BindGroup,
    samples: Mutex<Vec<RayBuildSample>>,
}

/// The timestamp bracket for one mesh build. Resolve it after the recording
/// has submitted and completed, or discard it with an abandoned recording.
pub struct RayBuildSample {
    /// Mesh name in the observing producer's records.
    pub mesh: String,
    /// Triangles passed to the build.
    pub triangles: u32,
    queries: wgpu::QuerySet,
}

impl RayBuildSample {
    /// Resolve and copy the two GPU ticks to a 16-byte, mappable readback buffer.
    #[must_use]
    pub fn resolve(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
    ) -> wgpu::Buffer {
        let buffer = |usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ray mesh build time"),
                size: 16,
                usage,
                mapped_at_creation: false,
            })
        };
        let resolve = buffer(wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC);
        let readback = buffer(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ);
        encoder.resolve_query_set(&self.queries, 0..2, &resolve, 0);
        encoder.copy_buffer_to_buffer(&resolve, 0, &readback, 0, Some(16));
        readback
    }
}

impl PreparedSynthetic {
    pub(crate) fn observe_ray_mesh_builds(
        &mut self,
        device: &wgpu::Device,
    ) -> Option<RayBuildProbe> {
        self.ray_structure.as_ref()?;
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let probe = self
            .ray_build_probe
            .get_or_insert_with(|| RayBuildProbe::new(device));
        Some(probe.clone())
    }
}

impl RayBuildProbe {
    fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ray mesh timestamp marker"),
            source: wgpu::ShaderSource::Wgsl(
                "@group(0) @binding(0) var<storage, read_write> sink: atomic<u32>;
                 @compute @workgroup_size(1) fn mark() { atomicAdd(&sink, 1u); }"
                    .into(),
            ),
        });
        let marker = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ray mesh timestamp marker"),
            layout: None,
            module: &shader,
            entry_point: Some("mark"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let sink = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ray mesh timestamp marker sink"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ray mesh timestamp marker group"),
            layout: &marker.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sink.as_entire_binding(),
            }],
        });
        Self(Arc::new(Probe {
            device: device.clone(),
            marker,
            binding,
            samples: Mutex::default(),
        }))
    }

    /// Record the real mesh build with timestamp boundaries on both sides.
    /// Call this in place of the uninstrumented build while observing the scene.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        mesh: String,
        build: &wgpu::BlasBuildEntry<'_>,
    ) {
        let triangles = match &build.geometry {
            wgpu::BlasGeometries::TriangleGeometries(geometry) => geometry
                .iter()
                .map(|mesh| mesh.size.index_count.unwrap_or(mesh.size.vertex_count) / 3)
                .sum(),
            wgpu::BlasGeometries::AabbGeometries(_) => 0,
        };
        let queries = self.0.device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("ray mesh build bracket"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        self.marker(encoder, &queries, None, Some(0));
        encoder.build_acceleration_structures([build], []);
        self.marker(encoder, &queries, Some(1), None);
        if let Ok(mut samples) = self.0.samples.lock() {
            samples.push(RayBuildSample {
                mesh,
                triangles,
                queries,
            });
        }
    }

    fn marker(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queries: &wgpu::QuerySet,
        begin: Option<u32>,
        end: Option<u32>,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("ray mesh timestamp marker"),
            timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                query_set: queries,
                beginning_of_pass_write_index: begin,
                end_of_pass_write_index: end,
            }),
        });
        pass.set_pipeline(&self.0.marker);
        pass.set_bind_group(0, &self.0.binding, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }

    /// Drain records once per observed frame. Resolve only those whose
    /// recording was submitted, on the same device after it has completed.
    #[must_use]
    pub fn take_recorded(&self) -> Vec<RayBuildSample> {
        self.0
            .samples
            .lock()
            .map_or_else(|_| Vec::new(), |mut samples| std::mem::take(&mut *samples))
    }
}
