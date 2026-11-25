//! Public lending seam for the Metal sky overlay and haze probes.
use std::sync::{Arc, Mutex};
use thrum_render::{
    atmosphere::{AtmosphereFrame, EnvironmentProducts},
    capabilities::DeviceCapabilities,
    contribution::{ContributionFrame, ContributionPasses, SceneContribution, SceneSurface},
    synthetic::{FrameLighting, SyntheticScene, SyntheticView},
};
use wgpu::util::DeviceExt;

pub(super) const BINDINGS: &str = r"
@group(1) @binding(0) var<storage,read> queries:array<vec4<f32>>;
@group(1) @binding(1) var<storage,read_write> output:array<vec4<f32>>;
";

type Overlay = fn(u32, u32, u32, u32) -> [f32; 4];
struct Borrow {
    products: Arc<Mutex<Option<EnvironmentProducts>>>,
    overlay: Option<Overlay>,
}
impl SceneContribution for Borrow {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }
    fn rebuild(
        &mut self,
        surface: &SceneSurface<'_>,
    ) -> Result<(), thrum_render::runtime::FrameError> {
        if let Some(sample) = self.overlay {
            let target = surface.sky_overlay_target().expect("registered overlay");
            let edge = target.width();
            let mut bytes = Vec::new();
            for face in 0..6 {
                for y in 0..edge {
                    for x in 0..edge {
                        for component in sample(face, x, y, edge) {
                            bytes.extend(half::f16::from_f32(component).to_le_bytes());
                        }
                    }
                }
            }
            surface.queue.write_texture(
                target.as_image_copy(),
                &bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(edge * 8),
                    rows_per_image: Some(edge),
                },
                target.size(),
            );
        }
        *self.products.lock().expect("observer") = surface.environment_products();
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}
}

pub(super) async fn prepare(
    opened: &thrum_test_support::Opened,
    frame: AtmosphereFrame,
    overlay: Option<Overlay>,
) -> (EnvironmentProducts, Vec<u8>) {
    let caps = DeviceCapabilities::capture(&opened.gpu.adapter, &opened.device, 1);
    let mut scene = SyntheticScene::grid(1).expect("scene");
    scene.sky_overlay = overlay.is_some();
    scene.view = SyntheticView::perspective(
        [0.0, 40.0, 0.0],
        [0.0, 40.0, 1.0],
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        200_000.0,
    )
    .expect("observer");
    let mut host = crate::support::host_with_extent(
        &caps,
        &opened.queue,
        scene,
        FrameLighting::Atmosphere(frame),
        [8, 8],
    )
    .await;
    let products = Arc::new(Mutex::new(None));
    host.set_scene_contribution(Box::new(Borrow {
        products: Arc::clone(&products),
        overlay,
    }))
    .expect("attach observer");
    let pixels = host.capture_pixels().expect("submit world light");
    let environment = products
        .lock()
        .expect("observer")
        .take()
        .expect("lent products");
    (environment, pixels)
}

pub(super) fn probe(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    env: &EnvironmentProducts,
    input: &[[f32; 4]],
    query: &str,
    outputs: u64,
) -> Vec<[f32; 4]> {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sky overlay observer"),
        entries: &[true, false]
            .into_iter()
            .enumerate()
            .map(|(i, read_only)| wgpu::BindGroupLayoutEntry {
                binding: u32::try_from(i).expect("two bindings"),
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect::<Vec<_>>(),
    });
    let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(input),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let bytes = params.size() * outputs;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&env.layout), Some(&layout)],
        immediate_size: 0,
    });
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("world light observer"),
        source: wgpu::ShaderSource::Wgsl(format!("{}\n{query}", env.shader_source).into()),
    });
    let site = thrum_render::runtime::DeviceSite::untracked(device.clone());
    let pipeline = site.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("world light observer"),
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, env.group(0), &[]);
        pass.set_bind_group(1, &group, &[]);
        pass.dispatch_workgroups(u32::try_from(input.len()).expect("bounded queries"), 1, 1);
    }
    let readback =
        thrum_test_support::readback::Readback::buffer(device, &mut encoder, &output, ..);
    let submission = queue.submit([encoder.finish()]);
    // The returned bytes carry no alignment guarantee, so decode each vec4 by value.
    readback
        .wait(device, submission)
        .as_chunks::<16>()
        .0
        .iter()
        .map(|texel| {
            std::array::from_fn(|i| {
                f32::from_le_bytes(texel[i * 4..i * 4 + 4].try_into().expect("four bytes"))
            })
        })
        .collect()
}
