//! G5 asset-loop viewer: the runtime half of the content-addressed loop in a small renderer.
//!
//! A loader thread polls the store's `HEAD`, maps the manifest and any changed products, validates
//! them, uploads them into *new* GPU resources in bounded chunks, and hands the complete resident
//! set to the render thread over a channel. The render thread swaps sets between frames (an
//! O(1) pointer exchange) and never waits for the GPU; replaced resources are dropped and wgpu
//! retires them after their last submission. Profiles are selected from the device capability
//! record, never by probing in a pass.
//!
//! This package links only `thrum-asset` from the asset loop; the cooker is a separate process.

mod loader;
mod upload;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use thrum_asset::Profile;
use thrum_render::capabilities::DeviceCapabilities;

use crate::loader::{Loader, Prepared};

/// Logical assets the viewer draws.
#[derive(Clone, Debug)]
pub struct ViewerConfig {
    /// Product store root written by the cooker.
    pub store: PathBuf,
    /// Mesh asset name.
    pub mesh: String,
    /// Texture asset name.
    pub texture: String,
    /// Volume asset name.
    pub volume: String,
    /// Offscreen target extent.
    pub extent: [u32; 2],
    /// Loader `HEAD` poll interval.
    pub poll_interval: Duration,
    /// Largest single queue write while uploading.
    pub upload_chunk: usize,
}

impl ViewerConfig {
    /// Defaults for the demo store layout.
    pub fn demo(store: impl Into<PathBuf>) -> Self {
        Self {
            store: store.into(),
            mesh: "meshes/terrain".into(),
            texture: "textures/checker".into(),
            volume: "cloud".into(),
            extent: [960, 540],
            poll_interval: Duration::from_millis(1),
            upload_chunk: 1 << 20,
        }
    }
}

/// One rendered frame.
#[derive(Clone, Copy, Debug)]
pub struct FrameRecord {
    /// Frame number.
    pub index: u64,
    /// Publication generation drawn (zero before the first resident set).
    pub generation: u64,
    /// Render-thread CPU time from frame start through submit and non-blocking poll.
    pub cpu: Duration,
    /// When the frame was submitted.
    pub submitted: Instant,
}

/// One resident-set replacement.
#[derive(Clone, Copy, Debug)]
pub struct SwapRecord {
    /// Generation now drawn.
    pub generation: u64,
    /// When the loader saw the new `HEAD`.
    pub head_seen: Instant,
    /// When the loader finished mapping, validating and uploading.
    pub prepared: Instant,
    /// When the render thread swapped it in.
    pub swapped: Instant,
    /// Product bytes uploaded for this set (reused products upload nothing).
    pub uploaded_bytes: u64,
    /// Whether every product came from a memory mapping.
    pub mapped: bool,
    /// Profile of the texture product drawn.
    pub texture_profile: Profile,
    /// Profile of the volume product drawn.
    pub volume_profile: Profile,
}

/// Shared measurement log.
#[derive(Debug, Default)]
pub struct Records {
    /// Frames in order.
    pub frames: Vec<FrameRecord>,
    /// GPU completion time per frame index (filled by non-blocking callbacks).
    pub completions: Vec<(u64, Instant)>,
    /// Swaps in order.
    pub swaps: Vec<SwapRecord>,
    /// Loader failures; the previous resident set stays in use.
    pub errors: Vec<String>,
}

impl Records {
    /// GPU completion of the first frame that drew `generation`.
    pub fn visible(&self, generation: u64) -> Option<Instant> {
        let first = self
            .frames
            .iter()
            .find(|frame| frame.generation >= generation)?
            .index;
        self.completions
            .iter()
            .find(|(index, _)| *index == first)
            .map(|(_, at)| *at)
    }
}

/// Error starting the viewer.
#[derive(Debug)]
pub enum ViewerError {
    /// No adapter or device.
    Device(String),
}

impl std::fmt::Display for ViewerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Device(reason) => write!(f, "GPU unavailable: {reason}"),
        }
    }
}

impl std::error::Error for ViewerError {}

const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Offscreen viewer with a background product loader.
pub struct Viewer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    capabilities: DeviceCapabilities,
    target: wgpu::Texture,
    depth: wgpu::TextureView,
    mesh_pipeline: wgpu::RenderPipeline,
    slice_pipeline: wgpu::RenderPipeline,
    scene_buffer: wgpu::Buffer,
    scene_group: wgpu::BindGroup,
    resident: Option<Prepared>,
    prepared: Receiver<Prepared>,
    stop: Arc<AtomicBool>,
    loader: Option<std::thread::JoinHandle<()>>,
    records: Arc<Mutex<Records>>,
    frame: u64,
    extent: [u32; 2],
}

impl Viewer {
    /// Create the device, pipelines and loader thread.
    ///
    /// # Errors
    /// No compatible adapter or device.
    pub fn new(config: ViewerConfig) -> Result<Self, ViewerError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .map_err(|error| ViewerError::Device(error.to_string()))?;
        let wanted = wgpu::Features::TEXTURE_COMPRESSION_BC
            | wgpu::Features::TEXTURE_COMPRESSION_BC_SLICED_3D
            | wgpu::Features::TEXTURE_COMPRESSION_ASTC;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("asset-loop"),
            required_features: adapter.features() & wanted,
            ..Default::default()
        }))
        .map_err(|error| ViewerError::Device(error.to_string()))?;
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 1);
        // A product the loader failed to reject must not abort the process: record the error and
        // keep drawing the previous resident set.
        let records = Arc::new(Mutex::new(Records::default()));
        let errors = Arc::clone(&records);
        device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
            if let Ok(mut log) = errors.lock() {
                log.errors.push(format!("GPU validation: {error}"));
            }
        }));
        Ok(Self::with_device(
            config,
            device,
            queue,
            capabilities,
            records,
        ))
    }

    fn with_device(
        config: ViewerConfig,
        device: wgpu::Device,
        queue: wgpu::Queue,
        capabilities: DeviceCapabilities,
        records: Arc<Mutex<Records>>,
    ) -> Self {
        let extent = config.extent;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("asset-loop target"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TARGET_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("asset-loop depth"),
                size: wgpu::Extent3d {
                    width: extent[0],
                    height: extent[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("asset-loop scene"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(96),
                },
                count: None,
            }],
        });
        let texture_entry = |binding, view_dimension| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension,
                multisampled: false,
            },
            count: None,
        };
        let product_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("asset-loop products"),
            entries: &[
                texture_entry(0, wgpu::TextureViewDimension::D2),
                texture_entry(1, wgpu::TextureViewDimension::D3),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let scene_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("asset-loop scene"),
            size: 96,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scene_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("asset-loop scene"),
            layout: &scene_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: scene_buffer.as_entire_binding(),
            }],
        });
        let (mesh_pipeline, slice_pipeline) = pipelines(&device, &scene_layout, &product_layout);
        let (sender, prepared) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let loader = Loader {
            device: device.clone(),
            queue: queue.clone(),
            layout: product_layout,
            texture_profiles: texture_preference(&capabilities),
            volume_profiles: volume_preference(&capabilities),
            config,
            records: Arc::clone(&records),
        };
        let stopping = Arc::clone(&stop);
        let loader = std::thread::Builder::new()
            .name("asset-loop loader".into())
            .spawn(move || loader.run(&sender, &stopping))
            .ok();
        Self {
            device,
            queue,
            capabilities,
            target,
            depth,
            mesh_pipeline,
            slice_pipeline,
            scene_buffer,
            scene_group,
            resident: None,
            prepared,
            stop,
            loader,
            records,
            frame: 0,
            extent,
        }
    }

    /// Capability record that selected the product profiles.
    pub fn capabilities(&self) -> &DeviceCapabilities {
        &self.capabilities
    }

    /// Texture profile preference chosen from the capability record.
    pub fn texture_profiles(&self) -> Vec<Profile> {
        texture_preference(&self.capabilities)
    }

    /// Volume profile preference chosen from the capability record.
    pub fn volume_profiles(&self) -> Vec<Profile> {
        volume_preference(&self.capabilities)
    }

    /// Shared measurement log.
    pub fn records(&self) -> Arc<Mutex<Records>> {
        Arc::clone(&self.records)
    }

    /// Generation currently drawn (zero before any resident set).
    pub fn generation(&self) -> u64 {
        self.resident.as_ref().map_or(0, |set| set.generation)
    }

    /// Render one frame without waiting for the GPU.
    pub fn frame(&mut self, time: f32) -> FrameRecord {
        let start = Instant::now();
        while let Ok(next) = self.prepared.try_recv() {
            let swapped = Instant::now();
            self.lock().swaps.push(SwapRecord {
                generation: next.generation,
                head_seen: next.head_seen,
                prepared: next.ready,
                swapped,
                uploaded_bytes: next.uploaded_bytes,
                mapped: next.mapped,
                texture_profile: next.texture_profile,
                volume_profile: next.volume_profile,
            });
            // Dropping the old set only releases handles; wgpu frees them after their last use.
            self.resident = Some(next);
        }
        let generation = self.generation();
        self.queue
            .write_buffer(&self.scene_buffer, 0, &scene_uniforms(self.extent, time));
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("asset-loop frame"),
            });
        self.record(&mut encoder);
        let index = self.frame;
        self.frame += 1;
        self.queue.submit([encoder.finish()]);
        let submitted = Instant::now();
        let records = Arc::clone(&self.records);
        self.queue.on_submitted_work_done(move || {
            if let Ok(mut log) = records.lock() {
                log.completions.push((index, Instant::now()));
            }
        });
        // Non-blocking: drives completion callbacks without waiting for the GPU.
        self.device.poll(wgpu::PollType::Poll).ok();
        let record = FrameRecord {
            index,
            generation,
            cpu: start.elapsed(),
            submitted,
        };
        self.lock().frames.push(record);
        record
    }

    fn record(&self, encoder: &mut wgpu::CommandEncoder) {
        let view = self
            .target
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("asset-loop scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.05,
                        g: 0.06,
                        b: 0.08,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        let Some(set) = &self.resident else { return };
        pass.set_bind_group(0, &self.scene_group, &[]);
        pass.set_bind_group(1, &set.bind_group, &[]);
        pass.set_pipeline(&self.mesh_pipeline);
        pass.set_vertex_buffer(0, set.mesh.positions.slice(..));
        pass.set_vertex_buffer(1, set.mesh.attributes.slice(..));
        pass.set_index_buffer(set.mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..set.mesh.index_count, 0, 0..1);
        pass.set_pipeline(&self.slice_pipeline);
        pass.draw(0..4, 0..1);
    }

    /// Render a frame and read it back as tightly packed sRGB RGBA8. Blocks; diagnostics only.
    pub fn capture(&mut self, time: f32) -> Vec<u8> {
        self.frame(time);
        let [width, height] = self.extent;
        let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("asset-loop capture"),
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("asset-loop capture"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let (send, receive) = channel();
        encoder.map_buffer_on_submit(&buffer, wgpu::MapMode::Read, .., move |result| {
            send.send(result).ok();
        });
        let submission = self.queue.submit([encoder.finish()]);
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(10)),
            })
            .ok();
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        if matches!(receive.recv_timeout(Duration::from_secs(10)), Ok(Ok(())))
            && let Ok(mapped) = buffer.slice(..).get_mapped_range()
        {
            for y in 0..height as usize {
                pixels.extend_from_slice(
                    &mapped[y * row as usize..y * row as usize + (width * 4) as usize],
                );
            }
        }
        pixels
    }

    /// Target extent.
    pub fn extent(&self) -> [u32; 2] {
        self.extent
    }

    /// Wait (bounded) for outstanding GPU work so completion records are final. Diagnostics only.
    pub fn settle(&self) {
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(5)),
            })
            .ok();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Records> {
        self.records
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Drop for Viewer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(loader) = self.loader.take() {
            loader.join().ok();
        }
    }
}

/// Apple GPUs prefer ASTC; other block-compression devices use BC7; everything falls back to
/// portable products.
fn texture_preference(capabilities: &DeviceCapabilities) -> Vec<Profile> {
    let features = capabilities.features();
    let mut order = Vec::new();
    if features.contains(wgpu::Features::TEXTURE_COMPRESSION_ASTC) {
        order.push(Profile::AppleAstc);
    }
    if features.contains(wgpu::Features::TEXTURE_COMPRESSION_BC) {
        order.push(Profile::DesktopBc);
    }
    order.push(Profile::Portable);
    order
}

/// BC4 volumes need sliced-3D BC support; otherwise R8.
fn volume_preference(capabilities: &DeviceCapabilities) -> Vec<Profile> {
    let features = capabilities.features();
    let mut order = Vec::new();
    if features.contains(
        wgpu::Features::TEXTURE_COMPRESSION_BC | wgpu::Features::TEXTURE_COMPRESSION_BC_SLICED_3D,
    ) {
        order.push(Profile::DesktopBc);
    }
    order.push(Profile::Portable);
    order
}

fn scene_uniforms(extent: [u32; 2], time: f32) -> [u8; 96] {
    let aspect = extent[0] as f32 / extent[1] as f32;
    let projection =
        glam::camera::rh::proj::directx::perspective_infinite_reverse(0.9, aspect, 0.05);
    let eye = glam::Vec3::new(1.9 * (time * 0.2).sin(), 1.4, 1.9 * (time * 0.2).cos());
    let view = glam::camera::rh::view::look_at_mat4(eye, glam::Vec3::ZERO, glam::Vec3::Y);
    let light = glam::Vec3::new(0.4, 0.8, 0.3).normalize();
    let mut values = [0.0_f32; 24];
    values[..16].copy_from_slice(&(projection * view).to_cols_array());
    values[16..20].copy_from_slice(&[light.x, light.y, light.z, 0.25]);
    values[20] = 0.5 + 0.45 * (time * 0.7).sin();
    let mut bytes = [0; 96];
    bytes.copy_from_slice(bytemuck::cast_slice(&values));
    bytes
}

fn pipelines(
    device: &wgpu::Device,
    scene: &wgpu::BindGroupLayout,
    products: &wgpu::BindGroupLayout,
) -> (wgpu::RenderPipeline, wgpu::RenderPipeline) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("asset-loop"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("asset-loop"),
        bind_group_layouts: &[Some(scene), Some(products)],
        immediate_size: 0,
    });
    let target = [Some(wgpu::ColorTargetState {
        format: TARGET_FORMAT,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    })];
    let depth = |compare, write| wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(write),
        depth_compare: Some(compare),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    };
    let position = [wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    }];
    let attributes = [
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Snorm16x2,
            offset: 0,
            shader_location: 1,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 4,
            shader_location: 2,
        },
    ];
    let mesh = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("asset-loop mesh"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("mesh_vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                Some(wgpu::VertexBufferLayout {
                    array_stride: 12,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &position,
                }),
                Some(wgpu::VertexBufferLayout {
                    array_stride: 12,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attributes,
                }),
            ],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(depth(wgpu::CompareFunction::Greater, true)),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("mesh_fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &target,
        }),
        multiview_mask: None,
        cache: None,
    });
    let slice = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("asset-loop volume slice"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("slice_vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            ..Default::default()
        },
        depth_stencil: Some(depth(wgpu::CompareFunction::Always, false)),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("slice_fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &target,
        }),
        multiview_mask: None,
        cache: None,
    });
    (mesh, slice)
}
