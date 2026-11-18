//! The governor's API changes the pool in a running Metal frame.

#[cfg(target_os = "macos")]
mod metal {

    use std::time::{Duration, Instant};

    use glam::{Mat4, Vec4};
    use mirror_scene::{Pool, fixture};
    use thrum_app::sky::StillWorld;
    use thrum_render::{
        capabilities::DeviceCapabilities,
        present::SurfaceProfile,
        runtime::{CaptureLimits, FrameConfig, FrameHost, FrameSample, SurfaceOutput},
        synthetic::{CaptureSource, OUTPUT_FORMAT},
    };

    const EXTENT: [u32; 2] = [384, 256];
    type Host = FrameHost<StillWorld, fixture::Source>;

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
            let mask = pool_pixels(Mat4::from_cols_array_2d(&scene.view.view_proj_columns()));
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
            host.set_scene_contribution(Box::new(Pool::new(fixture::POOL)))
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
        }
    }
}
