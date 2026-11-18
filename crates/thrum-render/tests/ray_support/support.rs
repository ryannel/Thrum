//! Shared target admission, scene setup and WGSL ray readback for the structure proofs.

use thrum_render::capabilities::{DeviceCapabilities, HardwareRequest};
use thrum_render::synthetic::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene};
use thrum_test_support::{Gpu, Profile, Restriction};
use wgpu::util::DeviceExt;

/// Admit the Metal adapter used by the proof.
/// # Panics
/// The proof cannot run without its target adapter.
pub fn gate(test: &str) -> Gpu {
    thrum_test_support::gpu(test, Some(wgpu::Backend::Metal)).expect("proof requires Metal")
}

/// Open a named Metal device and select the Apple tile recipe.
/// # Panics
/// The adapter and device must admit the named hardware and the recipe.
pub fn device(
    gpu: &Gpu,
    launch: &str,
    generation: u64,
) -> (wgpu::Device, wgpu::Queue, DeviceCapabilities) {
    let request = HardwareRequest::parse(launch).expect("named hardware");
    let (device, queue, capabilities) = gpu
        .fresh_adapter()
        .device_with_hardware(
            Profile::Portable,
            &Restriction::default(),
            generation,
            &request,
        )
        .expect("admitted Metal device");
    let capabilities = capabilities
        .with_recipe_override("apple-tile")
        .expect("Apple tile");
    (device, queue, capabilities)
}

/// Prepare a small offscreen scene with the requested device generation.
/// # Panics
/// Preparation and target compilation must succeed.
pub fn prepare(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    scene: &SyntheticScene,
) -> (PreparedSynthetic, OffscreenTarget) {
    let prepared = pollster::block_on(PreparedSynthetic::prepare_with_capabilities(
        caps,
        queue,
        scene,
        FrameLighting::Diagnostic(thrum_render::synthetic::SyntheticLight::default()),
        1.0,
    ))
    .expect("prepare scene");
    let target = OffscreenTarget::new(device, &prepared, 64, 64).expect("prepare target");
    (prepared, target)
}

/// Wait for a submitted GPU proof to finish.
/// # Panics
/// The timed GPU poll must complete.
pub fn wait(device: &wgpu::Device) {
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(20)),
        })
        .expect("GPU completes");
}

/// Collect the bytes of a submitted map-read buffer.
/// # Panics
/// The map and its completion callback must succeed.
pub fn read(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Vec<u8> {
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            send.send(result).expect("readback channel");
        });
    wait(device);
    receive.recv().expect("map callback").expect("valid map");
    let data = buffer.slice(..).get_mapped_range().expect("range").to_vec();
    buffer.unmap();
    data
}

/// The diagnostic compute pipeline and its bound scene structure.
pub struct Probe {
    pipeline: wgpu::ComputePipeline,
    structure: wgpu::BindGroup,
}

impl Probe {
    /// Bind the query pipeline to a prepared scene.
    /// # Panics
    /// The scene must provide its ray structure.
    pub fn new(device: &wgpu::Device, scene: &PreparedSynthetic) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene ray answers"),
            source: wgpu::ShaderSource::Wgsl(include_str!("probe.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("scene ray answers"),
            layout: None,
            module: &shader,
            entry_point: Some("probe"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let structure = scene
            .ray_structure_probe_group(&pipeline.get_bind_group_layout(0))
            .expect("scene ray structure");
        Self {
            pipeline,
            structure,
        }
    }

    /// Rays point down the Z axis through separated bodies.
    pub fn record(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        origins: &[[f32; 4]],
    ) -> wgpu::Buffer {
        self.record_timed(device, encoder, origins, None)
    }

    /// Record rays inside the named Metal pass timestamps.
    /// # Panics
    /// The fixture's ray count must fit in a workgroup grid.
    pub fn record_timed(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        origins: &[[f32; 4]],
        timestamp_writes: Option<wgpu::ComputePassTimestampWrites<'_>>,
    ) -> wgpu::Buffer {
        let rays = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("probe origins"),
            contents: bytemuck::cast_slice(origins),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let bytes = origins.len() as u64 * 4;
        let buffer = |usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("probe answers"),
                size: bytes,
                usage,
                mapped_at_creation: false,
            })
        };
        let answers = buffer(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let readback = buffer(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ);
        let payload = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("probe rays and answers"),
            layout: &self.pipeline.get_bind_group_layout(1),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: rays.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: answers.as_entire_binding(),
                },
            ],
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("trace scene probe"),
                timestamp_writes,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.structure, &[]);
            pass.set_bind_group(1, &payload, &[]);
            let count = u32::try_from(origins.len()).expect("probe ray count fits u32");
            pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&answers, 0, &readback, 0, Some(bytes));
        readback
    }
}

/// Read the returned logical slots (one-based, with zero for a miss).
pub fn answers(device: &wgpu::Device, readback: &wgpu::Buffer) -> Vec<u32> {
    read(device, readback)
        .as_chunks::<4>()
        .0
        .iter()
        .copied()
        .map(u32::from_le_bytes)
        .collect()
}

/// A single triangle with an unambiguous interior at the given x.
pub fn plane(center: f32) -> thrum_render::synthetic::SyntheticMesh {
    use thrum_render::synthetic::{SyntheticMesh, SyntheticVertex};
    SyntheticMesh {
        vertices: [
            [center - 0.5, -0.4, 0.0],
            [center + 0.5, -0.4, 0.0],
            [center, 0.8, 0.0],
        ]
        .map(|position| SyntheticVertex {
            position,
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
            tangent: [1.0, 0.0, 0.0, 1.0],
        })
        .to_vec(),
        indices: vec![0, 1, 2],
    }
}
