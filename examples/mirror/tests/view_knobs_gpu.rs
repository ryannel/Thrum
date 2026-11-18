//! The governor's API changes the pool in a running Metal frame.

#[cfg(target_os = "macos")]
mod metal {

    use std::{
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };

    use glam::{Mat4, Vec4};
    use mirror_scene::{Pool, fixture};
    use thrum_app::sky::{Camera, StillWorld};
    use thrum_render::{
        capabilities::DeviceCapabilities,
        contribution::{
            AfterOpaqueKind, AfterOpaquePass, ContributionFrame, ContributionPasses, EffectAccess,
            EffectCadence, EffectGraph, QualityError, QualityInvalidation, QualityKnob,
            SceneContribution, SceneSurface, SecondaryView, SecondaryViewFrame,
            SecondaryViewRequest, SecondaryViewSlot, ViewAir, ViewClipPlane,
        },
        graph::{AccessKind, ResourceDescriptor, ResourceScope},
        present::SurfaceProfile,
        runtime::{CaptureLimits, FrameConfig, FrameHost, FrameSample, SurfaceOutput},
        synthetic::{CaptureSource, OUTPUT_FORMAT},
    };

    const EXTENT: [u32; 2] = [384, 256];
    const VIEWPORT_REPORT_BYTES: u64 = 16;
    type Host = FrameHost<StillWorld, fixture::Source>;

    fn viewport_pass() -> AfterOpaquePass {
        AfterOpaquePass {
            name: "reflection viewport witness".into(),
            kind: AfterOpaqueKind::Compute,
            cadence: EffectCadence::EveryFrame,
            accesses: vec![EffectAccess {
                resource: 0,
                range: 0..VIEWPORT_REPORT_BYTES,
                kind: AccessKind::StorageWrite,
                history: None,
                view: None,
            }],
            depth: false,
            refraction: false,
            write_depth: false,
        }
    }

    #[derive(Default)]
    struct Observation {
        reference: Option<SecondaryViewRequest>,
        view: Option<SecondaryViewFrame>,
        measure_viewport: bool,
    }

    // Render the real pool in both runs. In the reference run supersede its
    // request with one whose values are supplied directly, without the knob
    // implementation. This gives an expected image for each endpoint on the
    // same host and device, with exactly the same main frame and pool shader.
    struct PoolWitness {
        pool: Pool,
        slot: Option<SecondaryView>,
        observation: Arc<Mutex<Observation>>,
        viewport_report: wgpu::Buffer,
        viewport_probe: Option<(wgpu::ComputePipeline, wgpu::BindGroup)>,
    }

    impl SceneContribution for PoolWitness {
        fn quality_knobs(&self) -> Vec<QualityKnob> {
            self.pool.quality_knobs()
        }
        fn set_quality_knob(&mut self, name: &str, value: u32) -> Result<(), QualityError> {
            self.pool.set_quality_knob(name, value)
        }
        fn commit_quality_knobs(&mut self) -> QualityInvalidation {
            self.pool.commit_quality_knobs()
        }
        fn secondary_view_slots(&self) -> Vec<SecondaryViewSlot> {
            self.pool.secondary_view_slots()
        }
        fn secondary_views_enabled(&self) -> bool {
            true
        }
        fn effect_graph(&self) -> EffectGraph {
            let mut graph = self.pool.effect_graph();
            graph.resources.push((
                ResourceDescriptor::Buffer {
                    bytes: VIEWPORT_REPORT_BYTES,
                    usage: self.viewport_report.usage(),
                },
                ResourceScope::Persistent {
                    owner: 1,
                    epoch: 1,
                    initialized: false,
                },
            ));
            graph.after_opaque.push(viewport_pass());
            graph
        }
        fn passes(&self) -> ContributionPasses {
            self.pool.passes()
        }
        fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
            let slot = surface.secondary_views.first().expect("pool slot");
            assert_eq!(slot.maximum_extent(), EXTENT);
            self.slot = Some(slot.clone());
            let device = surface.device;
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("reflection viewport witness"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("view_knobs_gpu/viewport.wgsl").into(),
                ),
            });
            let entries = [
                wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ];
            let group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("reflection viewport witness"),
                entries: &std::array::from_fn::<_, 3, _>(|binding| wgpu::BindGroupLayoutEntry {
                    binding: binding as u32,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: entries[binding],
                    count: None,
                }),
            });
            let inputs = viewport_pass().input_layout(device);
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("reflection viewport witness"),
                bind_group_layouts: &[
                    Some(surface.view_layout),
                    Some(&inputs),
                    Some(&group_layout),
                ],
                immediate_size: 0,
            });
            let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some("reflection viewport witness"),
                layout: Some(&layout),
                module: &shader,
                entry_point: Some("inspect_viewport"),
                compilation_options: Default::default(),
                cache: None,
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("reflection viewport witness"),
                layout: &group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(slot.colour()),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(slot.depth()),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.viewport_report.as_entire_binding(),
                    },
                ],
            });
            self.viewport_probe = Some((pipeline, group));
            self.pool.rebuild(surface)
        }
        fn after_opaque_runs(&self, index: usize, _: &ContributionFrame) -> bool {
            index == 0 || self.observation.lock().unwrap().measure_viewport
        }
        fn encode_after_opaque(
            &mut self,
            _: usize,
            pass: &mut wgpu::ComputePass<'_>,
            _: &ContributionFrame,
        ) {
            let (pipeline, group) = self
                .viewport_probe
                .as_ref()
                .expect("prepared viewport probe");
            pass.set_pipeline(pipeline);
            pass.set_bind_group(2, group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        fn encode_upload(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
            self.pool.encode_upload(encoder, frame);
            if let Some(reference) = self.observation.lock().unwrap().reference {
                self.slot
                    .as_ref()
                    .unwrap()
                    .request(&reference)
                    .expect("reference request");
            }
        }
        fn encode_secondary_upload(
            &mut self,
            _: &mut wgpu::CommandEncoder,
            _: &ContributionFrame,
            views: &[SecondaryViewFrame],
        ) {
            assert_eq!(views.len(), 1);
            self.observation.lock().unwrap().view = Some(views[0]);
        }
        fn draw_after_opaque(
            &self,
            index: usize,
            pass: &mut wgpu::RenderPass<'_>,
            frame: &ContributionFrame,
        ) {
            self.pool.draw_after_opaque(index, pass, frame);
        }
        fn encode_compute(&mut self, _: &mut wgpu::ComputePass<'_>, _: &ContributionFrame) {}
        fn computes(&self, _: &ContributionFrame) -> bool {
            false
        }
        fn encode_after_compute(&mut self, _: &mut wgpu::CommandEncoder) {}
        fn draw(&self, _: &mut wgpu::RenderPass<'_>) {}
        fn submitted(&mut self, _: bool) {}
    }

    fn read_viewport(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        report: &wgpu::Buffer,
    ) -> [u32; 4] {
        let (tx, rx) = std::sync::mpsc::channel();
        wgpu::util::DownloadBuffer::read_buffer(device, queue, &report.slice(..), move |result| {
            tx.send(result).expect("map caller");
        });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(15)),
            })
            .expect("map viewport report");
        let bytes = rx.recv().expect("map callback").expect("viewport bytes");
        std::array::from_fn(|i| u32::from_le_bytes(bytes[i * 4..][..4].try_into().unwrap()))
    }

    #[derive(Clone, Copy)]
    struct Expected {
        viewport: [u32; 2],
        far: f32,
        least: f32,
        count: u32,
        reach: f32,
    }

    impl Expected {
        fn endpoint(name: &str, value: u32) -> Self {
            let mut expected = Self {
                viewport: EXTENT,
                far: 32_000.0,
                least: 0.0,
                count: 1,
                reach: 128.0,
            };
            match name {
                "view extent percent" => expected.viewport = EXTENT.map(|side| side * value / 100),
                "view far metres" => expected.far = value as f32,
                "view least pixels" => expected.least = value as f32,
                "view Sun cascade count" => expected.count = value,
                "view Sun reach metres" => expected.reach = value as f32,
                _ => panic!("unexpected quality control {name}"),
            }
            expected
        }

        fn request(self, camera: Camera, eye: [f32; 3]) -> SecondaryViewRequest {
            let level = fixture::POOL.level;
            let view = thrum_render::synthetic::SyntheticView::perspective(
                [eye[0], 2.0 * level - eye[1], eye[2]],
                [
                    camera.target[0],
                    2.0 * level - camera.target[1],
                    camera.target[2],
                ],
                [0.0, 1.0, 0.0],
                camera.vertical_fov_degrees.to_radians(),
                EXTENT[0] as f32 / EXTENT[1] as f32,
                0.1,
                self.far,
            )
            .expect("authored reflected camera");
            let clip = ViewClipPlane::new([0.0, 1.0, 0.0, -level]).expect("clip at water");
            SecondaryViewRequest::new(&view, self.viewport, Some(clip), ViewAir::Plane)
                .expect("direct request")
                .with_least_pixels(self.least)
                .expect("direct culling threshold")
                .with_shadows(self.count, self.reach)
                .expect("direct shadow profile")
        }
    }

    fn sample(
        host: &mut Host,
        device: &wgpu::Device,
        surface: &SurfaceOutput<'_>,
        witness: &wgpu::Buffer,
    ) -> (Vec<u8>, FrameSample, Vec<u8>) {
        let instrumented = host.pipeline_creation_count().is_some();
        let mut encoded = host
            .advance(Duration::ZERO)
            .expect("advance")
            .encode_scene()
            .expect("encode");
        let witnessed = encoded.copy_main_draw_witness(witness);
        assert_eq!(witnessed.is_some(), instrumented);
        assert!(encoded.submit(Some(surface)).expect("drive").submitted);
        let deadline = Instant::now() + Duration::from_secs(15);
        let (mut image, mut sample) = (None, None);
        loop {
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(15)),
                })
                .expect("Metal wait");
            sample = sample.or_else(|| host.poll_completed().expect("retire"));
            image = image.or_else(|| host.try_submitted_image().expect("tap"));
            if let (Some(frame), Some(report)) = (&image, &sample) {
                assert_eq!(frame.sample.submission_order, report.submission_order);
                let data = frame.rgba.clone();
                host.recycle_submitted_image(image.expect("image"));
                let witness_bytes = witnessed.map_or_else(Vec::new, |size| {
                    let (tx, rx) = std::sync::mpsc::channel();
                    witness.map_async(wgpu::MapMode::Read, 0..size, move |result| {
                        tx.send(result).expect("map caller");
                    });
                    device
                        .poll(wgpu::PollType::Wait {
                            submission_index: None,
                            timeout: Some(Duration::from_secs(15)),
                        })
                        .expect("map wait");
                    rx.recv().expect("map callback").expect("main witness");
                    let bytes = witness
                        .get_mapped_range(0..size)
                        .expect("main draw bytes")
                        .to_vec();
                    witness.unmap();
                    bytes
                });
                return (data, sample.expect("timings"), witness_bytes);
            }
            assert!(Instant::now() < deadline, "missing submitted output");
        }
    }

    fn objects(device: &wgpu::Device) -> [isize; 11] {
        let hal = device.get_internal_counters().hal;
        [
            hal.buffers.read(),
            hal.textures.read(),
            hal.texture_views.read(),
            hal.samplers.read(),
            hal.render_pipelines.read(),
            hal.compute_pipelines.read(),
            hal.shader_modules.read(),
            hal.query_sets.read(),
            hal.bind_groups.read(),
            hal.bind_group_layouts.read(),
            hal.pipeline_layouts.read(),
        ]
    }

    fn pool_pixels(matrix: Mat4) -> Vec<bool> {
        let inverse = matrix.inverse();
        let [w, h] = EXTENT;
        (0..w * h)
            .map(|i| {
                let x = (i % w) as f32 + 0.5;
                let y = (i / w) as f32 + 0.5;
                let at = |depth| {
                    let world = inverse
                        * Vec4::new(
                            2.0 * x / w as f32 - 1.0,
                            1.0 - 2.0 * y / h as f32,
                            depth,
                            1.0,
                        );
                    world.truncate() / world.w
                };
                let near = at(1.0);
                let ray = at(0.0) - near;
                let t = (fixture::POOL.level - near.y) / ray.y;
                let point = near + ray * t;
                t > 0.0
                    && (fixture::POOL.min[0]..=fixture::POOL.max[0]).contains(&point.x)
                    && (fixture::POOL.min[1]..=fixture::POOL.max[1]).contains(&point.z)
            })
            .collect()
    }

    #[test]
    fn quality_controls_change_only_the_reflection_on_both_recipes() {
        let Some(gpu) = thrum_test_support::gpu("mirror view knobs", Some(wgpu::Backend::Metal))
        else {
            return;
        };
        let (device, queue) = gpu.device(
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        );
        let output = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("quality witness output"),
                size: wgpu::Extent3d {
                    width: EXTENT[0],
                    height: EXTENT[1],
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
            view: &output,
            extent: EXTENT,
        };
        let witness = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("main draw bytes"),
            size: 65536,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        for recipe in ["portable", "apple-tile"] {
            let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1)
                .with_recipe_override(recipe)
                .expect("recipe");
            let (scene, source) = fixture::scene(6, EXTENT);
            let fixture_view = scene.view;
            let mask = pool_pixels(Mat4::from_cols_array_2d(&fixture_view.view_proj_columns()));
            let mut host = pollster::block_on(FrameHost::new(
                &caps,
                &queue,
                FrameConfig {
                    render_extent: EXTENT,
                    surface: Some(SurfaceProfile {
                        format: OUTPUT_FORMAT,
                        extent: EXTENT,
                    }),
                },
                StillWorld::default(),
                source,
                scene,
                1.0,
            ))
            .expect("host");
            let observation = Arc::new(Mutex::new(Observation::default()));
            let viewport_report = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("reflection viewport witness"),
                size: VIEWPORT_REPORT_BYTES,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            host.set_scene_contribution(Box::new(PoolWitness {
                pool: Pool::new(fixture::POOL),
                slot: None,
                observation: Arc::clone(&observation),
                viewport_probe: None,
                viewport_report: viewport_report.clone(),
            }))
            .expect("prepare pool");
            host.enable_submitted_capture_from(
                1,
                CaptureLimits::default(),
                CaptureSource::SceneLinear,
            )
            .expect("linear tap");
            for _ in 0..4 {
                drop(sample(&mut host, &device, &surface, &witness));
            }
            let (before, baseline, main_draw) = sample(&mut host, &device, &surface, &witness);
            let objects_before = objects(&device);
            let pipelines = host.pipeline_creation_count();
            if pipelines.is_some() {
                assert!(objects_before[0] > 0 && objects_before[1] > 0);
                assert_ne!(main_draw.len(), 0);
            }
            let declarations = host.quality_knobs();
            assert_eq!(declarations.len(), 5);
            for knob in declarations {
                assert_ne!(knob.effect, "");
                assert!(
                    host.set_quality_knob(&knob.name, *knob.range.end() + 1)
                        .is_err()
                );
                let mut extremes = Vec::new();
                for value in [*knob.range.start(), *knob.range.end()] {
                    let name = knob.name.rsplit('/').next().unwrap();
                    let expected = Expected::endpoint(name, value);
                    host.set_quality_knob(&knob.name, value)
                        .expect("stage discovered knob");
                    let (pixels, report, main) = sample(&mut host, &device, &surface, &witness);
                    assert_eq!(
                        main, main_draw,
                        "{recipe} {}={value} changed main draw bytes",
                        knob.name
                    );
                    assert_eq!(report.camera_culling, baseline.camera_culling);
                    assert_eq!(report.uploaded_bytes, baseline.uploaded_bytes);
                    assert_eq!(host.pipeline_creation_count(), pipelines);
                    assert_eq!(objects(&device), objects_before);
                    let view = observation.lock().unwrap().view.expect("prepared view");
                    assert_eq!(
                        view.record.extent,
                        [
                            expected.viewport[0] as f32,
                            expected.viewport[1] as f32,
                            EXTENT[0] as f32,
                            EXTENT[1] as f32
                        ],
                        "{recipe} {name}={value}: active viewport in the full targets"
                    );
                    assert_eq!(view.record.options[2], expected.least);
                    assert_eq!(view.shadow_count, expected.count);
                    assert_eq!(view.shadow_reach, expected.reach);
                    let outside = before
                        .as_chunks::<8>()
                        .0
                        .iter()
                        .zip(pixels.as_chunks::<8>().0)
                        .zip(&mask)
                        .filter(|((a, b), inside)| !**inside && a != b)
                        .count();
                    assert_eq!(
                        outside, 0,
                        "{recipe} {}={value} changed outside the pool",
                        knob.name
                    );
                    // Recover the exact forward/FOV of the submitted main projection.
                    // The test independently specifies each secondary endpoint.
                    let matrix =
                        Mat4::from_cols_array_2d(&fixture_view.view_proj_columns()).transpose();
                    let forward = matrix.w_axis.truncate().normalize();
                    let vertical = matrix.y_axis.truncate();
                    let fov = (2.0
                        * (vertical - forward * vertical.dot(forward))
                            .length_recip()
                            .atan())
                    .to_degrees();
                    let eye = fixture_view.camera_position();
                    let camera = Camera::look_at(
                        eye,
                        (glam::Vec3::from_array(eye) + forward * 128.0).to_array(),
                        fov,
                    );
                    observation.lock().unwrap().reference = Some(expected.request(camera, eye));
                    let (reference, _, _) = sample(&mut host, &device, &surface, &witness);
                    observation.lock().unwrap().reference = None;
                    assert_eq!(
                        pixels
                            .as_chunks::<8>()
                            .0
                            .iter()
                            .zip(reference.as_chunks::<8>().0)
                            .zip(&mask)
                            .filter(|((a, b), inside)| **inside && a != b)
                            .count(),
                        0,
                        "{recipe} {name}={value}: pool differs from the independently requested endpoint"
                    );
                    extremes.push(pixels);
                }
                let changed = extremes[0]
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .zip(extremes[1].as_chunks::<8>().0)
                    .zip(&mask)
                    .filter(|((a, b), inside)| **inside && a != b)
                    .count();
                eprintln!(
                    "{recipe} {} {:?}: changed {changed} pool pixels / 0 outside",
                    knob.name, knob.range
                );
                assert!(
                    changed > 0,
                    "{recipe} {} must change the reflected result",
                    knob.name
                );
                host.set_quality_knob(&knob.name, knob.value)
                    .expect("restore default");
            }
            let (restored, _, _) = sample(&mut host, &device, &surface, &witness);
            assert_eq!(restored, before, "restore every setting byte for byte");
            let extent = host
                .quality_knobs()
                .into_iter()
                .find(|knob| knob.name.ends_with("view extent percent"))
                .unwrap();
            observation.lock().unwrap().measure_viewport = true;
            for value in [25, 100] {
                host.set_quality_knob(&extent.name, value)
                    .expect("set active viewport");
                drop(sample(&mut host, &device, &surface, &witness));
                let [width, height, inside, outside] =
                    read_viewport(&device, &queue, &viewport_report);
                assert_eq!([width, height], EXTENT);
                assert!(inside > 0, "{recipe}: empty reflection");
                assert_eq!(
                    outside > 0,
                    value == 100,
                    "{recipe}: pixels outside quarter viewport"
                );
                eprintln!(
                    "{recipe} {value}%: {width}x{height} targets, {inside} covered inside quarter viewport, {outside} outside"
                );
            }
        }
    }
}
