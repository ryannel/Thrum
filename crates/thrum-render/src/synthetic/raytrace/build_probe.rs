//! Explicit Metal build observation. The marker passes bracket the production
//! build without sampling host wait or readback time.

use crate::binding;
use std::sync::{Arc, Mutex};

use super::PreparedSynthetic;
use crate::runtime::DeviceSite;

/// Opt-in observer for mesh structure builds recorded on one device.
#[derive(Clone)]
pub struct RayBuildProbe(Arc<Probe>);

struct Probe {
    device: wgpu::Device,
    marker: wgpu::ComputePipeline,
    consumer: wgpu::ComputePipeline,
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
    /// Resolve four GPU ticks to 32 mappable bytes. Each pair spans a marker
    /// through a consuming query. The second pair skips the mesh build.
    #[must_use]
    pub fn resolve(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
    ) -> wgpu::Buffer {
        let buffer = |usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ray mesh build time"),
                size: 32,
                usage,
                mapped_at_creation: false,
            })
        };
        let resolve = buffer(wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC);
        let readback = buffer(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ);
        encoder.resolve_query_set(&self.queries, 0..4, &resolve, 0);
        encoder.copy_buffer_to_buffer(&resolve, 0, &readback, 0, Some(32));
        readback
    }
}

impl PreparedSynthetic {
    pub(crate) fn observe_ray_mesh_builds(&mut self) -> Option<RayBuildProbe> {
        let structure = self.ray_structure.as_mut()?;
        let device = self.capabilities.site();
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let probe = structure
            .probe
            .get_or_insert_with(|| RayBuildProbe::new(device));
        Some(probe.clone())
    }

    pub(in crate::synthetic) fn ray_build_probe(&self) -> Option<RayBuildProbe> {
        self.ray_structure.as_ref()?.probe.clone()
    }
}

impl RayBuildProbe {
    pub(super) fn new(device: &DeviceSite) -> Self {
        let _transition = device.transition("ray-mesh-build-probe");
        let shader = crate::shaders::Shader::RayBuild.compile(device, "ray mesh timestamp marker");
        let sink_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ray mesh timestamp sink layout"),
            entries: &[binding::storage(0, wgpu::ShaderStages::COMPUTE, false, 4)],
        });
        let structure_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ray mesh timestamp structure layout"),
            entries: &[binding::acceleration_structure(
                0,
                wgpu::ShaderStages::COMPUTE,
            )],
        });
        let pipeline = |entry, groups| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ray mesh timestamp pipeline layout"),
                bind_group_layouts: groups,
                immediate_size: 0,
            });
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some("ray mesh timestamp marker"),
                layout: Some(&layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        let marker_groups: [Option<&wgpu::BindGroupLayout>; 1] = [Some(&sink_layout)];
        let consumer_groups: [Option<&wgpu::BindGroupLayout>; 2] =
            [Some(&sink_layout), Some(&structure_layout)];
        let marker = pipeline("mark", &marker_groups);
        let consumer = pipeline("consume", &consumer_groups);
        let sink = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ray mesh timestamp marker sink"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ray mesh timestamp marker group"),
            layout: &sink_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sink.as_entire_binding(),
            }],
        });
        Self(Arc::new(Probe {
            device: wgpu::Device::clone(device),
            marker,
            consumer,
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
        let Ok(queries) =
            crate::timing::create_timestamp_query_set(&self.0.device, "ray mesh build bracket", 4)
        else {
            encoder.build_acceleration_structures([build], []);
            return;
        };
        let triangles = match &build.geometry {
            wgpu::BlasGeometries::TriangleGeometries(geometry) => geometry
                .iter()
                .map(|mesh| mesh.size.index_count.unwrap_or(mesh.size.vertex_count) / 3)
                .sum(),
            wgpu::BlasGeometries::AabbGeometries(_) => 0,
        };
        let mut tlas = super::create_tlas(&self.0.device, 1);
        tlas[0] = Some(wgpu::TlasInstance::new(
            build.blas,
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            0,
            1,
        ));
        let group = self.0.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ray mesh timestamp consumer binding"),
            layout: &self.0.consumer.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: tlas.as_binding(),
            }],
        });
        self.marker(encoder, &queries, 0, None);
        encoder.build_acceleration_structures([build], [&tlas]);
        self.marker(encoder, &queries, 1, Some(&group));
        // Ordered consumer on the same inputs and TLAS build, with the BLAS
        // already available. Both intervals include the same probe commands.
        self.marker(encoder, &queries, 2, None);
        encoder.build_acceleration_structures([], [&tlas]);
        self.marker(encoder, &queries, 3, Some(&group));
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
        end: u32,
        structure: Option<&wgpu::BindGroup>,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("ray mesh timestamp marker"),
            timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                query_set: queries,
                beginning_of_pass_write_index: None,
                end_of_pass_write_index: Some(end),
            }),
        });
        pass.set_pipeline(if structure.is_some() {
            &self.0.consumer
        } else {
            &self.0.marker
        });
        pass.set_bind_group(0, &self.0.binding, &[]);
        if let Some(structure) = structure {
            pass.set_bind_group(1, structure, &[]);
        }
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
