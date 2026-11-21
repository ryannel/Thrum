//! The governor's API changes the pool in a running Metal frame.

mod view_knobs_gpu {
    #[cfg(target_os = "macos")]
    mod witness;

    #[cfg(target_os = "macos")]
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    #[cfg(target_os = "macos")]
    use glam::{Mat4, Vec4};
    #[cfg(target_os = "macos")]
    use mirror_scene::{Pool, fixture};
    #[cfg(target_os = "macos")]
    use thrum_app::sky::{Camera, StillWorld};
    #[cfg(target_os = "macos")]
    use thrum_render::{
        capabilities::DeviceCapabilities,
        contribution::{SecondaryViewRequest, ViewAir, ViewClipPlane},
        present::SurfaceProfile,
        runtime::{CaptureLimits, FrameConfig, FrameHost, FrameSample, SurfaceOutput},
        synthetic::{CaptureSource, OUTPUT_FORMAT},
    };
    #[cfg(target_os = "macos")]
    use witness::{Observation, PoolWitness, VIEWPORT_REPORT_BYTES, read_viewport};

    #[cfg(target_os = "macos")]
    const EXTENT: [u32; 2] = [384, 256];
    #[cfg(target_os = "macos")]
    type Host = FrameHost<StillWorld, fixture::Source>;

    #[derive(Clone, Copy)]
    #[cfg(target_os = "macos")]
    struct Expected {
        viewport: [u32; 2],
        far: f32,
        least: f32,
        count: u32,
        reach: f32,
    }

    #[cfg(target_os = "macos")]
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

    #[cfg(target_os = "macos")]
    fn sample(
        host: &mut Host,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
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
        let (mut image, mut sample) = (None, None);
        let (frame, report) = thrum_test_support::wait::until(
            "missing submitted output",
            Duration::from_secs(15),
            || {
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(Duration::from_secs(15)),
                    })
                    .expect("Metal wait");
                sample = sample
                    .take()
                    .or_else(|| host.poll_completed().expect("retire"));
                image = image
                    .take()
                    .or_else(|| host.try_submitted_image().expect("tap"));
                if image.is_some() && sample.is_some() {
                    image.take().zip(sample.take())
                } else {
                    None
                }
            },
        );
        assert_eq!(frame.sample.submission_order, report.submission_order);
        let data = frame.rgba.clone();
        host.recycle_submitted_image(frame);
        let witness_bytes = witnessed.map_or_else(Vec::new, |size| {
            thrum_test_support::readback::buffer(device, queue, witness, 0..size)
        });
        (data, report, witness_bytes)
    }

    #[cfg(target_os = "macos")]
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

    #[cfg(target_os = "macos")]
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

    #[cfg(target_os = "macos")]
    fn output_view(device: &wgpu::Device) -> wgpu::TextureView {
        device
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
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    #[expect(clippy::print_stderr, reason = "the GPU witness is saved as evidence")]
    #[cfg(target_os = "macos")]
    fn verify_viewport(
        host: &mut Host,
        observation: &Mutex<Observation>,
        surface: &SurfaceOutput<'_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        witness: &wgpu::Buffer,
        viewport_report: &wgpu::Buffer,
        recipe: &str,
    ) {
        let extent = host
            .quality_knobs()
            .into_iter()
            .find(|knob| knob.name.ends_with("view extent percent"))
            .expect("extent control");
        observation.lock().expect("observation").measure_viewport = true;
        for value in [25, 100] {
            host.set_quality_knob(&extent.name, value)
                .expect("set active viewport");
            drop(sample(host, device, queue, surface, witness));
            let [width, height, inside, outside] = read_viewport(device, queue, viewport_report);
            assert_eq!([width, height], EXTENT);
            assert!(inside + outside > 0, "{recipe}: empty reflection");
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

    #[cfg(target_os = "macos")]
    fn submitted_camera(projection: Mat4, eye: [f32; 3]) -> Camera {
        // Use the main projection's exact forward and FOV so the optics
        // match bitwise. Specify the secondary controls independently.
        let rows = projection.transpose();
        let forward = rows.w_axis.truncate().normalize();
        let vertical = rows.y_axis.truncate();
        let fov = (2.0
            * (vertical - forward * vertical.dot(forward))
                .length_recip()
                .atan())
        .to_degrees();
        Camera::look_at(
            eye,
            (glam::Vec3::from_array(eye) + forward * 128.0).to_array(),
            fov,
        )
    }

    thrum_test_support::macos_gpu_test! {
    fn quality_controls_change_only_the_reflection_on_both_recipes() {
        let Some(thrum_test_support::Opened {
            gpu, device, queue, ..
        }) = thrum_test_support::open(
            "mirror view knobs",
            Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        )
        else {
            return;
        };
        let output = output_view(&device);
        let surface = SurfaceOutput {
            view: &output,
            extent: EXTENT,
        };
        let witness = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("main draw bytes"),
            size: 65536,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        for recipe in ["portable", "apple-tile"] {
            let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1)
                .with_recipe_override(recipe)
                .expect("recipe");
            let (scene, source) = fixture::scene(6, EXTENT);
            let projection = Mat4::from_cols_array_2d(&scene.view.view_proj_columns());
            let mask = pool_pixels(projection);
            let eye = scene.view.camera_position();
            let camera = submitted_camera(projection, eye);
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
            host.set_scene_contribution(Box::new(PoolWitness::new(
                EXTENT,
                Pool::new(fixture::POOL),
                Arc::clone(&observation),
                viewport_report.clone(),
            )))
            .expect("prepare pool");
            host.enable_submitted_capture_from(
                1,
                CaptureLimits::default(),
                CaptureSource::SceneLinear,
            )
            .expect("linear tap");
            for _ in 0..4 {
                drop(sample(&mut host, &device, &queue, &surface, &witness));
            }
            let (before, baseline, main_draw) = sample(&mut host, &device, &queue, &surface, &witness);
            let objects_before = objects(&device);
            let pipelines = host.pipeline_creation_count();
            if pipelines.is_some() {
                assert!(objects_before[0] > 0 && objects_before[1] > 0);
                assert_ne!(main_draw.len(), 0);
            }
            let declarations = host.quality_knobs();
            assert_eq!(declarations.len(), 5);
            for knob in declarations {
                let name = knob.name.rsplit('/').next().expect("control name");
                let (range, default) = match name {
                    "view extent percent" => (25..=100, 100),
                    "view far metres" => (64..=32_000, 32_000),
                    "view least pixels" => (0..=16, 0),
                    "view Sun cascade count" => (0..=2, 1),
                    "view Sun reach metres" => (16..=512, 128),
                    _ => panic!("unexpected quality control {name}"),
                };
                assert_eq!(knob.range, range, "{name}");
                assert_eq!(knob.value, default, "{name}");
                assert_ne!(knob.effect, "");
                assert!(
                    host.set_quality_knob(&knob.name, *knob.range.end() + 1)
                        .is_err()
                );
                let mut extremes = Vec::new();
                for value in [*knob.range.start(), *knob.range.end()] {
                    let expected = Expected::endpoint(name, value);
                    host.set_quality_knob(&knob.name, value)
                        .expect("stage discovered knob");
                    let (pixels, report, main) = sample(&mut host, &device, &queue, &surface, &witness);
                    assert_eq!(
                        main, main_draw,
                        "{recipe} {}={value} changed main draw bytes",
                        knob.name
                    );
                    assert_eq!(report.camera_culling, baseline.camera_culling);
                    assert_eq!(report.uploaded_bytes, baseline.uploaded_bytes);
                    assert_eq!(host.pipeline_creation_count(), pipelines);
                    assert_eq!(objects(&device), objects_before);
                    let view = observation
                        .lock()
                        .expect("observation")
                        .view
                        .expect("prepared view");
                    assert_eq!(
                        view.record.extent.map(f32::to_bits),
                        [
                            expected.viewport[0] as f32,
                            expected.viewport[1] as f32,
                            EXTENT[0] as f32,
                            EXTENT[1] as f32
                        ]
                        .map(f32::to_bits),
                        "{recipe} {name}={value}: active viewport in the full targets"
                    );
                    assert_eq!(view.record.options[2].to_bits(), expected.least.to_bits());
                    assert_eq!(view.shadow_count, expected.count);
                    assert_eq!(view.shadow_reach.to_bits(), expected.reach.to_bits());
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
                    observation.lock().expect("observation").reference =
                        Some(expected.request(camera, eye));
                    let (reference, _, _) = sample(&mut host, &device, &queue, &surface, &witness);
                    observation.lock().expect("observation").reference = None;
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
            let (restored, _, _) = sample(&mut host, &device, &queue, &surface, &witness);
            assert_eq!(restored, before, "restore every setting byte for byte");
            verify_viewport(
                &mut host,
                &observation,
                &surface,
                &device,
                &queue,
                &witness,
                &viewport_report,
                recipe,
            );
        }
    }
    }
}
