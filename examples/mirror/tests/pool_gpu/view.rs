use std::{cell::RefCell, fmt::Write, rc::Rc, time::Duration};

use thrum_app::sky::StillWorld;
use thrum_asset::{BrowserFetch, DevClient};
use thrum_asset_gpu::{
    CookedAssets, CookedConfig, CookedDrawPath, CookedSource, MeshPlacement, RemoteFeed,
};
use thrum_render::{
    abi::InstanceRecord,
    capabilities::DeviceCapabilities,
    contribution::{
        AfterOpaqueKind, AfterOpaquePass, ContributionFrame, ContributionPasses, EffectAccess,
        EffectCadence, EffectGraph, SceneContribution, SceneContributions, SceneSurface,
        SecondaryView, SecondaryViewRequest, SecondaryViewSlot, ViewAir, ViewClipPlane,
    },
    extract::{ExtractReport, RenderExtractor, SourceTick},
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    present::SurfaceProfile,
    runtime::{FrameConfig, FrameError, FrameHost, SceneFrame, SceneSource, SurfaceOutput},
    synthetic::{
        FrameLighting, OUTPUT_FORMAT, PhotometricEnvironment, SyntheticPointLight, SyntheticScene,
        SyntheticView,
    },
};

use super::retire;

const SIDE: u32 = 32;
const BYTES: u64 = (SIDE * SIDE * 32) as u64;
const LIGHT: FrameLighting = FrameLighting::Photometric(PhotometricEnvironment {
    toward_sun: [0.0, 0.8, 0.6],
    sun_lux: 50_000.0,
    sun_colour: [1.0; 3],
    ambient_lux: [1500.0; 3],
    background_radiance: [0.1; 3],
});

struct Source {
    view: SyntheticView,
    lighting: FrameLighting,
    positions: Vec<[f64; 3]>,
}

impl SceneSource<SourceTick> for Source {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.0; 3], |sink| {
            for (slot, &position) in (0_u32..).zip(&self.positions) {
                sink.push(slot, u64::from(slot) + 1, u128::from(slot), position, 0);
            }
        })
    }
    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        Ok(SceneFrame {
            view: self.view,
            lighting: self.lighting,
        })
    }
}

fn terrain_camera(eye: [f32; 3], direction: [f32; 3]) -> SyntheticView {
    SyntheticView::perspective(
        eye,
        (glam::Vec3::from_array(eye) + glam::Vec3::from_array(direction)).to_array(),
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        10_000.0,
    )
    .expect("terrain perspective")
}

fn lit_scene(view: &SyntheticView) -> SyntheticScene {
    let mut scene = SyntheticScene::grid(1).expect("scene");
    scene.view = *view;
    scene.sun_shadows = true;
    scene.instances[0].current[3] = [0.0, 0.75, 0.0, 1.0];
    scene.instances[0].previous = scene.instances[0].current;
    scene.local_lights = vec![SyntheticPointLight {
        position: [2.0, 2.5, 4.0],
        radius: 8.0,
        candela: [500.0, 350.0, 50.0],
    }];
    scene
}

fn terrain_scene(view: &SyntheticView) -> SyntheticScene {
    let mut scene = SyntheticScene::grid(4).expect("terrain scene");
    scene.view = *view;
    scene.sun_shadows = true;
    scene.materials[0].base_rgba = [0.8, 0.8, 0.8, 1.0];
    // The Metal witness's distant receivers and hills. A no-Sun-map render
    // below checks that these actually exercise the height-map shadow.
    scene.instances = [
        ([0.0, -0.5, -3000.0], [600.0, 0.5, 500.0]),
        ([250.0, 125.0, -3000.0], [125.0, 125.0, 400.0]),
        ([-3984.0, 3400.0, -2000.0], [1500.0, 1000.0, 0.5]),
        ([-3184.0, 4200.0, -1200.0], [300.0; 3]),
    ]
    .into_iter()
    .zip(0_u32..)
    .map(|((centre, half), slot)| {
        let mut block = InstanceRecord::identity(slot, 0);
        for axis in 0..3 {
            block.current[axis][axis] = half[axis];
            block.normal[axis][axis] = half[axis].recip();
            block.current[3][axis] = centre[axis];
        }
        block.previous = block.current;
        block
    })
    .collect();
    scene.local_lights = vec![SyntheticPointLight {
        position: [-3784.0, 3400.0, -1900.0],
        radius: 100.0,
        candela: [10_000.0, 1000.0, 100.0],
    }];
    scene
}

fn camera(eye: [f32; 3], target: [f32; 3]) -> SyntheticView {
    SyntheticView::perspective(eye, target, [0.0, 1.0, 0.0], 0.85, 1.0, 0.1, 100.0)
        .expect("perspective")
}

struct Probe {
    request: Option<SecondaryViewRequest>,
    slot: Option<SecondaryView>,
    pipeline: Option<(wgpu::ComputePipeline, wgpu::BindGroup)>,
    output: Rc<RefCell<Option<wgpu::Buffer>>>,
}

impl Probe {
    fn pass(&self) -> AfterOpaquePass {
        AfterOpaquePass {
            name: "stored view proof".into(),
            kind: AfterOpaqueKind::Compute,
            cadence: EffectCadence::EveryFrame,
            accesses: vec![EffectAccess {
                resource: 0,
                range: 0..BYTES,
                kind: AccessKind::StorageWrite,
                history: None,
                view: None,
            }],
            depth: self.request.is_none(),
            refraction: self.request.is_none(),
            images: thrum_render::contribution::AfterOpaqueImages::None,
            write_depth: false,
        }
    }
}

impl SceneContribution for Probe {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: self.request.is_some(),
            compute: false,
        }
    }
    fn secondary_view_slots(&self) -> Vec<SecondaryViewSlot> {
        self.request
            .iter()
            .map(|_| SecondaryViewSlot::new(1.0).expect("full-size slot"))
            .collect()
    }
    fn effect_graph(&self) -> EffectGraph {
        EffectGraph {
            resources: vec![(
                ResourceDescriptor::Buffer {
                    bytes: BYTES,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                },
                ResourceScope::Persistent {
                    owner: 1,
                    epoch: 1,
                    initialized: false,
                },
            )],
            after_opaque: vec![self.pass()],
            ..EffectGraph::default()
        }
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), FrameError> {
        self.slot = surface.secondary_views.first().cloned();
        let device = surface.device;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("view equality output"),
            size: BYTES,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];
        let mut bindings = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }];
        if let Some(slot) = &self.slot {
            for (binding, sample_type, view) in [
                (1, wgpu::TextureSampleType::Depth, slot.depth()),
                (
                    2,
                    wgpu::TextureSampleType::Float { filterable: false },
                    slot.colour(),
                ),
            ] {
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                });
                bindings.push(wgpu::BindGroupEntry {
                    binding,
                    resource: wgpu::BindingResource::TextureView(view),
                });
            }
        }
        let group = if self.slot.is_some() { 2 } else { 1 };
        let first_binding = u32::from(self.slot.is_some());
        let code = format!(
            r"
            @group({group}) @binding({first_binding}) var depths: texture_depth_2d;
            @group({group}) @binding({}) var colours: texture_2d<f32>;
            struct Pixel {{ rgba: vec4<f32>, depth: f32 }}
            @group(2) @binding(0) var<storage, read_write> pixels: array<Pixel>;
            @compute @workgroup_size(8,8) fn read_view(@builtin(global_invocation_id) p: vec3<u32>) {{
                let xy = vec2<i32>(p.xy);
                pixels[p.y * {SIDE}u + p.x] = Pixel(textureLoad(colours, xy, 0), textureLoad(depths, xy, 0));
            }}
        ",
            first_binding + 1
        );
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &entries,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &bindings,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                Some(surface.view_layout),
                Some(&self.pass().input_layout(device)),
                Some(&layout),
            ],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("view equality reader"),
            source: wgpu::ShaderSource::Wgsl(code.into()),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("view equality reader"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("read_view"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        self.pipeline = Some((pipeline, group));
        *self.output.borrow_mut() = Some(buffer);
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {
        if let (Some(slot), Some(request)) = (&self.slot, &self.request) {
            slot.request(request)
                .expect("request view on main vertical");
        }
    }
    fn encode_after_opaque(
        &mut self,
        _: usize,
        pass: &mut wgpu::ComputePass<'_>,
        _: &ContributionFrame,
    ) {
        let (pipeline, group) = self.pipeline.as_ref().expect("prepared observer");
        pass.set_pipeline(pipeline);
        pass.set_bind_group(2, group, &[]);
        pass.dispatch_workgroups(SIDE / 8, SIDE / 8, 1);
    }
}

struct Fixture<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    caps: &'a DeviceCapabilities,
}

impl Fixture<'_> {
    async fn draw(
        &self,
        scene: SyntheticScene,
        lighting: FrameLighting,
        request: Option<&SecondaryViewRequest>,
        extent: [u32; 2],
        cooked: Option<&RemoteFeed>,
        expected_path: Option<CookedDrawPath>,
    ) -> Vec<u8> {
        let positions = if cooked.is_some() {
            Vec::new()
        } else {
            scene
                .instances
                .iter()
                .map(|row| std::array::from_fn(|axis| f64::from(row.current[3][axis])))
                .collect()
        };
        let mut host = Box::new(
            Box::pin(FrameHost::new(
                self.caps,
                self.queue,
                FrameConfig {
                    render_extent: extent,
                    surface: Some(SurfaceProfile {
                        extent,
                        format: OUTPUT_FORMAT,
                    }),
                },
                StillWorld::default(),
                Source {
                    view: scene.view,
                    lighting,
                    positions,
                },
                scene,
                1.0,
            ))
            .await
            .expect("host"),
        );
        let mut parts: Vec<Box<dyn SceneContribution>> = Vec::new();
        let stats = cooked.map(|remote| {
            let assets = CookedAssets::new(CookedConfig::new(
                CookedSource::Remote(remote.clone()),
                vec![MeshPlacement::new("meshes/rock", "textures/white").at([0.0, 0.75, 0.0])],
            ))
            .expect("cooked rock placement");
            let stats = assets.stats();
            parts.push(Box::new(assets));
            stats
        });
        let output = Rc::new(RefCell::new(None));
        parts.push(Box::new(Probe {
            request: request.copied(),
            slot: None,
            pipeline: None,
            output: Rc::clone(&output),
        }));
        host.set_scene_contribution(Box::new(SceneContributions::new(parts)))
            .expect("view and main fixture");
        host.set_timing_requested(false);
        let presenter = self
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("view proof output"),
                size: wgpu::Extent3d {
                    width: extent[0],
                    height: extent[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: OUTPUT_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let surface = SurfaceOutput {
            view: &presenter,
            extent,
        };
        let sent = host
            .drive(Duration::ZERO, Some(&surface))
            .expect("submit fixture");
        assert!(sent.submitted);
        let deadline = retire::Instant::now() + Duration::from_secs(15);
        loop {
            if host.poll_completed().expect("frame complete").is_some() {
                if stats.as_ref().is_none_or(|stats| {
                    let stats = stats.lock().expect("cooked statistics");
                    assert!(stats.errors.is_empty(), "{:?}", stats.errors);
                    stats.drawn.is_some()
                }) {
                    break;
                }
                assert!(
                    host.drive(Duration::ZERO, Some(&surface))
                        .expect("draw cooked content")
                        .submitted
                );
            }
            assert!(
                retire::Instant::now() < deadline,
                "fixture did not become drawable"
            );
            retire::pause().await;
        }
        if let Some(stats) = stats {
            let stats = stats.lock().expect("cooked statistics");
            assert_eq!(stats.draw_path, expected_path);
            if let Some(path) = expected_path {
                assert_eq!(
                    stats.secondary_draw_paths,
                    if request.is_some() {
                        vec![(0, path)]
                    } else {
                        Vec::new()
                    }
                );
            }
        }
        let buffer = output.take().expect("output");
        retire::copy_buffer(self.device, self.queue, &buffer).await
    }
}

fn decode(value: &[u8; 32]) -> [f32; 8] {
    std::array::from_fn(|i| f32::from_le_bytes(value[i * 4..][..4].try_into().expect("word")))
}

fn compare(ordinary: &[u8], view: &[u8]) -> (usize, [u32; 2]) {
    let mut covered = 0;
    let mut maxima = [0; 2];
    for (main, view) in ordinary
        .as_chunks::<32>()
        .0
        .iter()
        .zip(view.as_chunks::<32>().0)
    {
        let main = decode(main);
        let view = decode(view);
        assert_eq!(main[4] > 0.0, view[3] > 0.0, "opaque coverage");
        if main[4] == 0.0 {
            continue;
        }
        covered += 1;
        for c in 0..3 {
            maxima[0] = maxima[0].max(u32::from(
                half::f16::from_f32(main[c])
                    .to_bits()
                    .abs_diff(half::f16::from_f32(view[c]).to_bits()),
            ));
        }
        maxima[1] = maxima[1].max(main[4].to_bits().abs_diff(view[4].to_bits()));
    }
    assert!(
        covered > 10 && maxima[0] <= 1 && maxima[1] <= 1,
        "covered={covered} maximum stored steps {maxima:?}"
    );
    (covered, maxima)
}

pub(super) async fn prove(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    store: &str,
) -> String {
    let mut client = DevClient::new(BrowserFetch, store).expect("store URL");
    let feed = RemoteFeed::new(
        client
            .poll()
            .await
            .expect("fetch cooked assets")
            .expect("published generation"),
    );
    let hidden = camera([0.4, 3.7, 5.0], [2.0, 3.7, 8.0]);
    let camera = camera([0.4, 1.7, 5.0], [0.0, 0.75, 0.0]);
    let direct = caps.clone().without_indirect_execution();
    let mut report = String::new();
    for (name, caps, cooked, path) in [
        ("entities", caps, None, None),
        (
            "cooked direct",
            &direct,
            Some(&feed),
            Some(CookedDrawPath::Direct),
        ),
        (
            "cooked indirect",
            caps,
            Some(&feed),
            Some(CookedDrawPath::Indirect),
        ),
    ] {
        let fixture = Fixture {
            device,
            queue,
            caps,
        };
        for cascades in 0..=2 {
            let ordinary = camera
                .with_sun_shadow_profile(cascades, 128.0)
                .expect("shadow profile");
            let request = SecondaryViewRequest::new(&camera, [SIDE; 2], None, ViewAir::Camera)
                .expect("request")
                .with_shadows(cascades, 128.0)
                .expect("profile");
            let main = fixture
                .draw(lit_scene(&ordinary), LIGHT, None, [SIDE; 2], cooked, path)
                .await;
            let scales: &[u32] = if cooked.is_some() { &[2] } else { &[1, 4] };
            for scale in scales {
                let view = fixture
                    .draw(
                        lit_scene(&hidden),
                        LIGHT,
                        Some(&request),
                        [SIDE * scale; 2],
                        cooked,
                        path,
                    )
                    .await;
                let (covered, maxima) = compare(&main, &view);
                writeln!(report, "{name}, 1/{scale} extent, {cascades} cascades: {covered} covered, max colour/depth steps {maxima:?}")
                    .expect("string");
            }
        }
        if cooked.is_some() {
            let clip = ViewClipPlane::new([0.0, 1.0, 0.0, -50.0]).expect("clip plane");
            let clipped =
                SecondaryViewRequest::new(&camera, [SIDE; 2], Some(clip), ViewAir::Camera)
                    .expect("clip request");
            let away = SecondaryViewRequest::new(&hidden, [SIDE; 2], None, ViewAir::Camera)
                .expect("away request");
            let small = SecondaryViewRequest::new(&camera, [SIDE; 2], None, ViewAir::Camera)
                .expect("size request")
                .with_least_pixels(128.0)
                .expect("least size");
            for (case, request, expected_clear) in [
                ("clip", clipped, true),
                ("away", away, true),
                ("least size", small, path == Some(CookedDrawPath::Indirect)),
            ] {
                let output = fixture
                    .draw(
                        lit_scene(&hidden),
                        LIGHT,
                        Some(&request),
                        [64; 2],
                        cooked,
                        path,
                    )
                    .await;
                let clear = output
                    .as_chunks::<32>()
                    .0
                    .iter()
                    .all(|pixel| pixel[12..20] == [0; 8]);
                assert_eq!(clear, expected_clear, "{name}: {case}");
                writeln!(report, "{name}, {case}: clear={clear}").expect("string");
            }
        }
    }
    prove_terrain(
        &Fixture {
            device,
            queue,
            caps,
        },
        &mut report,
    )
    .await;
    report
}

async fn prove_terrain(fixture: &Fixture<'_>, report: &mut String) {
    use glam::{Mat4, Vec3, Vec4};

    let high = terrain_camera([16.0, 1000.0, 0.0], [-16.0, -1000.0, -3000.0]);
    let raised = terrain_camera([16.0, 1080.0, 0.0], [-16.0, -1080.0, -3000.0]);
    let mirror_main = terrain_camera([16.0, 10.0, 0.0], [-2.0, -1.7, -1.0]);
    let mirror = terrain_camera([16.0, 0.0, 0.0], [-2.0, 1.7, -1.0]);
    let old_texel = |view: &SyntheticView, direction: Vec3| {
        let eye = Vec3::from_array(view.camera_position());
        let matrix = glam::camera::rh::view::look_at_mat4(eye, eye + direction, Vec3::Y);
        (matrix.inverse().w_axis.x / 32.0).round()
    };
    let main_texel = old_texel(&mirror_main, Vec3::new(-2.0, -1.7, -1.0));
    let mirror_texel = old_texel(&mirror, Vec3::new(-2.0, 1.7, -1.0));
    assert_ne!(
        main_texel.to_bits(),
        mirror_texel.to_bits(),
        "matrix-recovered eyes must split the texel edge"
    );
    writeln!(
        report,
        "terrain old-fit texels at the declared x=16 m edge: {main_texel}, {mirror_texel}"
    )
    .expect("string");

    let light = FrameLighting::Photometric(PhotometricEnvironment {
        toward_sun: [1.0 / 3.0_f64.sqrt(); 3],
        sun_lux: 100_000.0,
        sun_colour: [1.0; 3],
        ambient_lux: [10_000.0, 12_000.0, 15_000.0],
        background_radiance: [100.0; 3],
    });
    for (label, main, view) in [
        ("same position", high, high),
        ("80 m raised and repitched", high, raised),
        ("texel-edge mirror pair", mirror_main, mirror),
    ] {
        let mut clear_scene = terrain_scene(&view);
        clear_scene.sun_shadows = false;
        let clear = fixture
            .draw(clear_scene, light, None, [SIDE; 2], None, None)
            .await;
        let inverse = Mat4::from_cols_array_2d(&view.view_proj_columns()).inverse();
        for count in 0..=2 {
            let ordinary = view
                .with_sun_shadow_profile(count, 128.0)
                .expect("main profile");
            let expected = fixture
                .draw(terrain_scene(&ordinary), light, None, [SIDE; 2], None, None)
                .await;
            let mut distant_shadowed = 0;
            for (index, (shadow, clear)) in expected
                .as_chunks::<32>()
                .0
                .iter()
                .zip(clear.as_chunks::<32>().0)
                .enumerate()
            {
                let [red, _, _, _, depth, ..] = decode(shadow);
                if depth > 0.0 && red < decode(clear)[0] * 0.9 {
                    let index = u32::try_from(index).expect("witness pixel index");
                    let x = (index % SIDE) as f32 + 0.5;
                    let y = (index / SIDE) as f32 + 0.5;
                    let p = inverse
                        * Vec4::new(
                            2.0 * x / SIDE as f32 - 1.0,
                            1.0 - 2.0 * y / SIDE as f32,
                            depth,
                            1.0,
                        );
                    // The normalized clip point uses w=1. Undo that division
                    // to recover clip w, which is forward depth in metres.
                    distant_shadowed += usize::from(p.w.recip() > 128.0);
                }
            }
            assert!(
                distant_shadowed > 0,
                "{label}: missing hill shadows beyond 128 m"
            );
            let request = SecondaryViewRequest::new(&view, [SIDE; 2], None, ViewAir::Camera)
                .expect("request")
                .with_shadows(count, 128.0)
                .expect("secondary profile");
            for scale in [1, 4] {
                let actual = fixture
                    .draw(
                        terrain_scene(&main),
                        light,
                        Some(&request),
                        [SIDE * scale; 2],
                        None,
                        None,
                    )
                    .await;
                let (covered, maxima) = compare(&expected, &actual);
                writeln!(report, "terrain {label}, 1/{scale} extent, {count} cascades: {covered} covered, {distant_shadowed} hill-shadowed past 128 m, max colour/depth steps {maxima:?}")
                    .expect("string");
            }
        }
    }
}
