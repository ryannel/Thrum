//! The shipping height-map march against a flat plateau at a known height and extent.
use super::observer::{SAMPLE_FLOATS, constant_depth_maps, device, pipeline, read_visibility};
use super::{HORIZON_RADIUS, SIZE, SUN_ANGULAR_RADIUS, Vec3, height_matrix};

const OBSERVER: &str = r"
@group(2) @binding(6) var<storage,read_write> result: array<f32>;
@compute @workgroup_size(64)
fn observe(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=3u { return; }
    let height=array(-5.0,5.0,120.0)[id.x];
    let receiver=ShadowReceiver(vec3(0.0,height,0.0),
        vec3(1.0,0.0,0.0),vec3(0.0,0.0,1.0),vec3(0.0,1.0,0.0));
    result[id.x]=horizon_visibility(receiver);
}
";

#[test]
fn a_below_horizontal_sun_is_partially_hidden_by_distant_lower_terrain() {
    pollster::block_on(async {
        let (device, queue) = device().await;
        let (matrix, _) = height_matrix(Vec3::ZERO, Some((0.0, 0.0))).expect("height map fit");
        let texel = 2.0 * HORIZON_RADIUS / SIZE as f32;
        let depth = matrix.row(2);
        let mut samples = [0.0; SAMPLE_FLOATS];
        samples[48..64].copy_from_slice(&matrix.to_cols_array());
        samples[64..68].copy_from_slice(&[depth.y, depth.w, 0.5 / HORIZON_RADIUS, texel]);
        let maps = constant_depth_maps(&device, &queue, SIZE, &[0.0, 0.0, depth.w]);
        let elevation = -0.1_f32.to_radians();
        let (sin, cos) = elevation.sin_cos();
        let observed = read_visibility(
            &device,
            &queue,
            &pipeline(&device, OBSERVER, false),
            &samples,
            (
                &maps.create_view(&wgpu::TextureViewDescriptor::default()),
                None,
            ),
            [cos, sin, 0.0, 0.0],
            3,
        );
        // For a receiver above a flat plateau the far edge sets the highest angle.
        // Account for the shader's conservative height allowance and allow 0.015
        // visibility error for the last geometric sample falling short of the edge.
        let edge = ((-5.0 - 0.125 * texel) / HORIZON_RADIUS).atan();
        let limit = (edge - elevation) / SUN_ANGULAR_RADIUS;
        let reference = (0..4096)
            .map(|i| 2.0 * (f64::from(i) + 0.5) / 4096.0 - 1.0)
            .map(|y| ((1.0 - y * y).sqrt(), y > f64::from(limit)))
            .fold((0.0, 0.0), |(visible, total), (weight, lit)| {
                (visible + if lit { weight } else { 0.0 }, total + weight)
            });
        assert!(observed[0] <= 1e-6, "receiver below plateau: {observed:?}");
        let expected = reference.0 / reference.1;
        assert!(
            (f64::from(observed[1]) - expected).abs() < 0.015,
            "grazing Sun: GPU {}, plateau reference {expected}",
            observed[1]
        );
        assert!(
            observed[2] > 1.0 - 1e-6,
            "receiver high above plateau: {observed:?}"
        );
    });
}

const CELL_OBSERVER: &str = r"
@group(2) @binding(6) var<storage,read_write> result: array<f32>;
@compute @workgroup_size(64)
fn observe(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=2u { return; }
    let receiver=ShadowReceiver(vec3(0.0,0.0,128.0*f32(id.x)),
        vec3(1.0,0.05,0.0),vec3(0.0,0.0,1.0),normalize(vec3(-0.05,1.0,0.0)));
    result[id.x]=horizon_visibility(receiver);
}
";

#[test]
fn horizon_cells_preserve_a_lit_rasterized_slope_and_a_distant_crest() {
    pollster::block_on(async {
        let (device, queue) = device().await;
        let (matrix, _) =
            height_matrix(Vec3::ZERO, Some((-820.0, 1_200.0))).expect("height map fit");
        let depth = matrix.row(2);
        let mut samples = [0.0; SAMPLE_FLOATS];
        samples[44] = 1_200.0;
        samples[48..64].copy_from_slice(&matrix.to_cols_array());
        samples[64..68].copy_from_slice(&[depth.y, depth.w, 0.5 / HORIZON_RADIUS, 32.0]);
        let maps = constant_depth_maps(&device, &queue, SIZE, &[0.0; 3]);
        let view = maps.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2),
            base_array_layer: 2,
            array_layer_count: Some(1),
            ..Default::default()
        });
        let parents = rasterize_horizon_witness(&device, &queue, &view, matrix);
        let elevation = 3.2_f32.to_radians();
        let (sin, cos) = elevation.sin_cos();
        let observed = read_visibility(
            &device,
            &queue,
            &pipeline(&device, CELL_OBSERVER, true),
            &samples,
            (
                &maps.create_view(&wgpu::TextureViewDescriptor::default()),
                Some(&parents),
            ),
            [cos, sin, 0.0, 0.0],
            2,
        );
        // tan(3.2° - solar radius) exceeds the exact 0.05 slope.
        assert!(
            observed[0] > 0.999,
            "the sloped plane sees the full disk: {observed:?}"
        );
        // The 12.54 km crest has slope 1200 / 12544 > tan(3.2° + solar radius).
        assert!(
            observed[1] < 0.001,
            "the distant crest hides the disk: {observed:?}"
        );
    });
}

fn rasterize_horizon_witness(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    height: &wgpu::TextureView,
    matrix: glam::Mat4,
) -> wgpu::TextureView {
    use crate::synthetic::horizon;
    use wgpu::util::DeviceExt;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ramp and crest height witness"),
        source: wgpu::ShaderSource::Wgsl(
            r"
@group(0) @binding(0) var<uniform> height_view_proj: mat4x4<f32>;
@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corners = array(vec2(0.0,0.0),vec2(1.0,0.0),vec2(0.0,1.0),
                        vec2(0.0,1.0),vec2(1.0,0.0),vec2(1.0,1.0));
    let p = corners[index%6u];
    if index < 6u {
        let xz = p*32768.0-16384.0;
        return height_view_proj*vec4(xz.x,0.05*xz.x,xz.y,1.0);
    }
    return height_view_proj*vec4(12544.0+32.0*p.x,1200.0,64.0+128.0*p.y,1.0);
}
"
            .into(),
        ),
    });
    let raster = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: None,
        multiview_mask: None,
        cache: None,
    });
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: matrix.to_cols_array().map(f32::to_le_bytes).as_flattened(),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &raster.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        }],
    });
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: SIZE / 2,
            height: SIZE / 2,
            depth_or_array_layers: 1,
        },
        mip_level_count: SIZE.ilog2(),
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    let prepared = horizon::prepare(device);
    let target = prepared.target_from_texture(device, height, texture.clone());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: height,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&raster);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..12, 0..1);
    }
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        prepared.encode(&mut pass, &target);
    }
    queue.submit([encoder.finish()]);
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}
