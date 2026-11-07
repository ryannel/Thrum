//! The shipping PCSS consumer against independent finite-disk/plane geometry.
use super::observer::{constant_depth_maps, device, pipeline, read_visibility};
use wgpu::util::DeviceExt;

const SIDE: u32 = 256;
const SAMPLES: usize = 33;
const SCALE: f32 = 0.1;

const OBSERVER: &str = r"
@group(2) @binding(6) var<storage,read_write> result: array<f32>;
@compute @workgroup_size(64)
fn observe(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=66u { return; }
    let cascade=id.x/33u;
    let u=0.4+0.2*f32(id.x%33u)/32.0;
    let slope=light.direction_to_light.w;
    let x=2.0*u-1.0;
    let receiver=ShadowReceiver(vec3(x,0.0,0.25+slope*x),
        vec3(1.0,0.0,slope),vec3(0.0,1.0,0.0),normalize(vec3(-slope,0.0,1.0)));
    result[id.x]=cascade_visibility(cascade,receiver);
}
";

const CASCADE_HANDOFF: &str = r"
@group(2) @binding(6) var<storage,read_write> result: array<f32>;
@compute @workgroup_size(64)
fn observe(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x!=0u { return; }
    let receiver=ShadowReceiver(vec3(-0.96,0.0,0.25),
        vec3(1.0,0.0,0.0),vec3(0.0,1.0,0.0),vec3(0.0,0.0,1.0));
    result[0]=near_visibility(receiver);
}
";

fn oracle(u: f64, slope: f64, gap: f64) -> f64 {
    let mut visible = 0;
    for radius in 0..64 {
        let r = ((f64::from(radius) + 0.5) / 64.0).sqrt();
        for azimuth in 0..128 {
            let x = r * (std::f64::consts::TAU * (f64::from(azimuth) + 0.5) / 128.0).cos();
            let separation = gap / (1.0 - 2.0 * slope * f64::from(SCALE) * x);
            visible += usize::from(u + f64::from(SCALE) * separation * x < 0.5);
        }
    }
    visible as f64 / 8192.0
}

fn render_planes(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    depth: &wgpu::Texture,
    slope: f32,
    kind: u8,
) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("analytic test blocker depth"),
        source: wgpu::ShaderSource::Wgsl(
            r"
@group(0) @binding(0) var<uniform> parameters:vec4<f32>;
@vertex fn vertex(@builtin(vertex_index) i:u32) -> @builtin(position) vec4<f32> {
    let xy=array(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(xy[i],0.5,1.0);
}
@fragment fn fragment(@builtin(position) p:vec4<f32>) -> @builtin(frag_depth) f32 {
    if parameters.z==1.0 { return 0.0; }
    let x=2.0*p.x/256.0-1.0;
    // A continuous receiver bends toward the light past a concave join at
    // x=+-0.1. Its relative slope 0.5 stays below the slope any finite-disk ray
    // climbs, so geometric visibility is one everywhere on the receiver.
    if parameters.z==4.0 { return clamp(0.25+parameters.x*x+0.5*max(0.0,x-0.1),0.0,1.0); }
    if parameters.z==5.0 { return clamp(0.25+parameters.x*x+0.5*max(0.0,-0.1-x),0.0,1.0); }
    if parameters.z==3.0 && p.x<128.0 {
        return min(0.24,0.26+parameters.x*x);
    }
    let covered=select(p.x>=128.0,all(abs(p.xy-vec2(128.0))<=vec2(1.0)),parameters.z==2.0);
    let gap=select(0.0,parameters.y,covered);
    return clamp(0.25+parameters.x*x+gap,0.0,1.0);
}
"
            .into(),
        ),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[],
        }),
        multiview_mask: None,
        cache: None,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    for (layer, gap) in [0.05, 0.4].into_iter().enumerate() {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: [slope, gap, f32::from(kind), 0.0]
                .map(f32::to_le_bytes)
                .as_flattened(),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        let view = depth.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2),
            base_array_layer: u32::try_from(layer).expect("two layer depth target"),
            array_layer_count: Some(1),
            ..Default::default()
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
    queue.submit([encoder.finish()]);
}

fn observe(device: &wgpu::Device, queue: &wgpu::Queue, slope: f32, kind: u8) -> Vec<f32> {
    let mut samples = [0_f32; 68];
    let identity = glam::Mat4::IDENTITY.to_cols_array();
    samples[..16].copy_from_slice(&identity);
    samples[16..32].copy_from_slice(&identity);
    samples[36..38].fill(0.000_03);
    samples[40..44].copy_from_slice(&[SCALE, SCALE, 0.000_001, 0.000_001]);
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("two finite-disk test blocker distances"),
        size: wgpu::Extent3d {
            width: SIDE,
            height: SIDE,
            depth_or_array_layers: 2,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    render_planes(device, queue, &depth, slope, kind);
    let view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    read_visibility(
        device,
        queue,
        &pipeline(device, OBSERVER),
        &samples,
        &view,
        [0.0, 0.0, 1.0, slope],
        u32::try_from(SAMPLES * 2).expect("two small sample rows"),
    )
}

#[test]
fn finite_sun_shadow_width_tracks_blocker_distance_and_tilted_receivers() {
    pollster::block_on(async {
        let (device, queue) = device().await;
        let clear_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        assert!(
            observe(&device, &queue, 3.0, 1)
                .iter()
                .all(|x| (*x - 1.0).abs() < 1e-6),
            "grazing receiver planes never turn empty depth into a blocker"
        );
        assert!(clear_scope.pop().await.is_none());
        // Observed receivers lie on the plane; lanes 0..=24 sit before the
        // right join (x<=0.1) and lanes 8..=32 after the left one (x>=-0.1).
        for (kind, lanes) in [(4, 0..=24), (5, 8..=32)] {
            let join_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
            let join = observe(&device, &queue, 3.0, kind);
            for cascade in 0..2 {
                for lane in lanes.clone() {
                    let visible = join[cascade * SAMPLES + lane];
                    assert!(
                        visible > 0.99,
                        "a concave join on a grazing receiver is not its own blocker: \
                         kind{kind} cascade{cascade} lane{lane} {visible}"
                    );
                }
            }
            assert!(join_scope.pop().await.is_none());
        }
        let narrow_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let narrow = observe(&device, &queue, 0.0, 2);
        assert!(
            narrow[SAMPLES + SAMPLES / 2] > 0.8,
            "a narrow central caster contributes its finite-source penumbra"
        );
        assert!(narrow_scope.pop().await.is_none());
        let behind_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let reference = observe(&device, &queue, 0.4, 0);
        let behind = observe(&device, &queue, 0.4, 3);
        for lane in 15..18 {
            assert!(
                (reference[SAMPLES + lane] - behind[SAMPLES + lane]).abs() < 0.1,
                "downstream depths cannot collapse an upstream penumbra"
            );
        }
        assert!(behind_scope.pop().await.is_none());
        for slope in [0.0, 0.4] {
            let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
            let observed = observe(&device, &queue, slope, 0);
            let error = scope.pop().await;
            assert!(error.is_none(), "{error:?}");
            for (cascade, gap) in [0.05, 0.4].into_iter().enumerate() {
                let row = &observed[cascade * SAMPLES..(cascade + 1) * SAMPLES];
                for (i, actual) in row.iter().enumerate() {
                    let u = 0.4 + 0.2 * i as f64 / 32.0;
                    let expected = oracle(u, f64::from(slope), gap);
                    assert!(
                        (f64::from(*actual) - expected).abs() <= 0.12,
                        "slope{slope} gap{gap} u{u}: GPU{actual} reference{expected}"
                    );
                }
                assert!(row.windows(2).all(|v| v[0] >= v[1] - 0.02));
            }
        }
    });
}

#[test]
fn near_and_last_cascades_cover_complementary_blocker_distances() {
    pollster::block_on(async {
        let (device, queue) = device().await;
        let pipeline = pipeline(&device, CASCADE_HANDOFF);
        let mut samples = [0_f32; 68];
        samples[..16].copy_from_slice(&glam::Mat4::IDENTITY.to_cols_array());
        let far = glam::Mat4::from_scale(glam::Vec3::splat(0.5));
        samples[16..32].copy_from_slice(&far.to_cols_array());
        samples[36..38].fill(0.000_03);
        samples[40..44].copy_from_slice(&[SCALE, SCALE, 0.000_001, 0.000_001]);
        samples[46..48].copy_from_slice(&[1.0, 0.5]);
        // The receiver sits at u=0.02 in the near map. After the two-texel guard,
        // its near blocker-gap limit is about 0.122. The last cascade doubles
        // the depth span, so its complementary window starts at about 0.061.
        for (depths, expected) in [
            ([0.30, 0.0], 0.0),
            ([0.0, 0.225], 0.0),
            ([0.45, 0.0], 1.0),
            ([0.0, 0.165], 1.0),
        ] {
            let maps = constant_depth_maps(&device, &queue, SIDE, &depths);
            let result = read_visibility(
                &device,
                &queue,
                &pipeline,
                &samples,
                &maps.create_view(&wgpu::TextureViewDescriptor::default()),
                [0.0, 0.0, 1.0, 0.0],
                1,
            );
            assert!(
                (result[0] - expected).abs() <= 1e-6,
                "near and last depths {depths:?}: GPU {}, expected {expected}",
                result[0]
            );
        }
    });
}
