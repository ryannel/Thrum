use super::*;
use crate::atmosphere::products::tests::observer;

const CLEAR: [u16; 4] = [0, 0, 0, 0x3c00];
const CLOUDY: [u16; 4] = [0x5400, 0x5200, 0x5000, 0x3800];
const LEVELS: [f32; 3] = [0.0, 0.5, 1.0];

struct ForwardProbe {
    pipeline: wgpu::RenderPipeline,
    empty: wgpu::BindGroup,
    levels: [wgpu::Texture; 3],
}

impl ForwardProbe {
    fn new(device: &wgpu::Device, prepared: &PreparedSynthetic) -> Self {
        let (source, lighting, _) = prepared.forward_light_witness();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("real forward reflection term"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{source}
@vertex
fn probe_vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {{
    return vec4(vec2(f32((i << 1u) & 2u), f32(i & 2u)) * 2.0 - 1.0, 0.0, 1.0);
}}
@fragment
fn probe_fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {{
    let level = floor(position.x) * 0.5;
    let roughness = GGX_MIN_ROUGHNESS + (1.0 - GGX_MIN_ROUGHNESS) * level / 6.0;
    let value = indirect_radiance(vec3(1.0), 1.0, roughness, vec3(0.0, 0.0, 1.0),
        vec3(0.0, 0.0, 1.0), 1.0, vec3(0.0), position);
    let size = vec2<f32>(textureDimensions(sky_dfg));
    let unit = (roughness - GGX_MIN_ROUGHNESS) / (1.0 - GGX_MIN_ROUGHNESS);
    let uv = (vec2(1.0, unit) * (size - 1.0) + vec2(0.5)) / size;
    let dfg = textureSampleLevel(sky_dfg, sky_reflection_sampler, uv, 0.0).rg;
    return vec4(value / (dfg.x + dfg.y), 1.0);
}}
"
                )
                .into(),
            ),
        });
        let empty_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[],
        });
        let empty = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &empty_layout,
            entries: &[],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&empty_layout), Some(lighting)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("forward reflection term probe"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("probe_vertex"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("probe_fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::TextureFormat::Rgba32Float.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview_mask: None,
            cache: None,
        });
        let levels = std::array::from_fn(|_| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("forward reflection level probe"),
                size: wgpu::Extent3d {
                    width: 3,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        });
        Self {
            pipeline,
            empty,
            levels,
        }
    }

    fn sample(&self, probe: &Probe<'_>, slot: usize) -> Vec<[f32; 4]> {
        let device = probe.caps.device();
        let (_, _, lighting) = probe.prepared.forward_light_witness();
        let texture = &self.levels[slot];
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.empty, &[]);
            pass.set_bind_group(1, lighting, &[]);
            pass.draw(0..3, 0..1);
        }
        let copy = Readback::texture(device, &mut encoder, texture, Mip::level(0), ..);
        let submission = probe.queue.submit([encoder.finish()]);
        let bytes = copy.wait(device, submission);
        bytes
            .as_chunks::<16>()
            .0
            .iter()
            .map(|chunk| bytemuck::pod_read_unaligned(chunk))
            .collect()
    }
}

fn observer_products(probe: &Probe<'_>) -> crate::atmosphere::EnvironmentProducts {
    probe
        .prepared
        .environment_products_witness(probe.queue)
        .unwrap()
}

fn assert_handoff(observed: &[[f32; 4]], low: &[f32; 4], high: &[f32; 4]) {
    for (index, observed) in observed.iter().enumerate() {
        let weight = LEVELS[index];
        for band in 0..3 {
            let expected = (high[band] - low[band]).mul_add(weight, low[band]);
            // Metal cube filtering and normalization of the production forward
            // DFG term are allowed 0.2% of scene-linear radiance.
            let tolerance = expected.abs().max(1.0) * 0.002;
            assert!(
                (observed[band] - expected).abs() <= tolerance,
                "level {weight} band {band}: {} != {expected} (tolerance {tolerance})",
                observed[band]
            );
        }
    }
}

#[test]
fn cloud_and_clear_reflection_levels_decode_before_blending() {
    let Some(crate::test_device::Opened {
        gpu, device, queue, ..
    }) = crate::test_device::open(
        "Metal mixed reflection encodings",
        Some(wgpu::Backend::Metal),
        crate::capabilities::DeviceProfile::Portable,
        &crate::capabilities::DeviceRestriction::default(),
    )
    else {
        return;
    };
    crate::timestamp_permit::hold();
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        let air = moving(0.0);
        let mut spread = Probe::new(&caps, &queue, air, true).await;
        let mut clear = Probe::new(&caps, &queue, air, true).await;
        let mut cloudy = Probe::new(&caps, &queue, air, true).await;
        clear.overlay(CLEAR);
        cloudy.overlay(CLOUDY);
        clear.draw(air);
        cloudy.draw(air);
        let forward = ForwardProbe::new(&device, &spread.prepared);
        let lent = observer_products(&spread);
        let probe = observer::Probe::new(
            &device,
            &lent,
            "",
            ["vec4(environment_reflection(q.xyz, q.w, 1.0).radiance, 1.0)"],
        );
        let sample = |products: &crate::atmosphere::EnvironmentProducts| {
            let queries = LEVELS.map(|level| [0.0, 0.0, 1.0, 0.045 + 0.955 * level / 6.0]);
            probe
                .sample(&device, &queue, products, &queries, 0)
                .into_iter()
                .map(|[value]| value)
                .collect::<Vec<_>>()
        };
        let clear_lent = sample(&observer_products(&clear));
        let cloudy_lent = sample(&observer_products(&cloudy));
        let clear_forward = forward.sample(&clear, 0);
        let cloudy_forward = forward.sample(&cloudy, 1);
        assert_ne!(clear_lent[0], cloudy_lent[0]);

        spread.draw(air);
        spread.cadence(4);
        for (rgba, old_lent, new_lent, old_forward, new_forward) in [
            (
                CLOUDY,
                &clear_lent,
                &cloudy_lent,
                &clear_forward,
                &cloudy_forward,
            ),
            (
                CLEAR,
                &cloudy_lent,
                &clear_lent,
                &cloudy_forward,
                &clear_forward,
            ),
        ] {
            spread.draw(air); // Diffuse and level zero use the previous overlay.
            spread.overlay(rgba);
            spread.draw(air); // Level one uses the just-published overlay.
            assert_handoff(&sample(&lent), &old_lent[0], &new_lent[2]);
            assert_handoff(
                &forward.sample(&spread, 2),
                &old_forward[0],
                &new_forward[2],
            );
            for _ in 0..6 {
                spread.draw(air);
            } // Complete the change and the following refresh.
        }
    });
}
