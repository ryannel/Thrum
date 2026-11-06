use super::{AtmosphereFrame, model};

#[test]
fn sample_admission_and_direct_transport_have_physical_limits() {
    for direction in [[0.0; 3], [f32::NAN, 1.0, 0.0]] {
        assert!(AtmosphereFrame::new(direction, 127_000.0, [1.0; 3], 2.0, 1).is_err());
    }
    for altitude in [-1.0, f32::INFINITY, 100_001.0] {
        assert!(AtmosphereFrame::new([0.0, 1.0, 0.0], 127_000.0, [1.0; 3], altitude, 1).is_err());
    }
    let frame = AtmosphereFrame::new([0.0, 4.0, 0.0], 127_000.0, [1.0; 3], 2.0, 1)
        .expect("valid Earth sample");
    assert_eq!(frame.toward_sun(), [0.0, 1.0, 0.0]);
    let noon = model::sun_transmittance(2.0, 1.0);
    let sunset = model::sun_transmittance(2.0, 0.02);
    assert!(noon.iter().all(|x| (0.0..1.0).contains(x)));
    assert!(noon[0] > noon[2]);
    assert!(sunset[0] / sunset[2] > noon[0] / noon[2]);
    assert_eq!(model::sun_transmittance(2.0, -1.0), [0.0; 3]);
    assert_eq!(model::sun_transmittance(100_000.0, 1.0), [1.0; 3]);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn portable_lut_graph_renders_live_sun_and_rejects_mode_changes() {
    use crate::capabilities::DeviceCapabilities;
    use crate::synthetic::{
        FrameLighting, OffscreenTarget, PreparedSynthetic, RenderError, SyntheticLight,
        SyntheticScene, SyntheticView,
    };

    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real adapter required for atmosphere regression");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("atmosphere device");
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 51);
        let mut scene = SyntheticScene::grid(1).expect("one geometry consumer");
        // The visible +Z face points away from the Sun and has no emission or
        // local lights: any nonblack result on it must come from diffuse sky.
        scene.instances[0].current = [
            [0.3, 0.0, 0.0, 0.0],
            [0.0, 0.3, 0.0, 0.0],
            [0.0, 0.0, 0.3, 0.0],
            [0.0, 0.0, 0.5, 1.0],
        ];
        scene.instances[0].previous = scene.instances[0].current;
        scene.instances[0].normal = [
            [1.0 / 0.3, 0.0, 0.0, 0.0],
            [0.0, 1.0 / 0.3, 0.0, 0.0],
            [0.0, 0.0, 1.0 / 0.3, 0.0],
        ];
        scene.materials[0].base_rgba = [0.8, 0.8, 0.8, 1.0];
        scene.view = SyntheticView::perspective(
            [0.0, 0.0, 2.0],
            [0.0, 0.1, 0.0],
            [0.0, 1.0, 0.0],
            1.4,
            2.0,
            0.1,
            20.0,
        )
        .expect("sky view");
        let noon =
            AtmosphereFrame::new([0.0, 0.5, -0.866], 127_000.0, [1.0; 3], 2.0, 1).expect("noon");
        let mut prepared = PreparedSynthetic::prepare_with_capabilities(
            &capabilities,
            &queue,
            &scene,
            FrameLighting::Atmosphere(noon),
            1.0 / 16_000.0,
        )
        .await
        .expect("all atmosphere shaders validate");
        assert_eq!(prepared.presentation_light_clamps(), 3);
        let mut target = OffscreenTarget::new(&device, &prepared, 128, 64).expect("sky target");
        let graph = prepared.graph(&target, false).expect("atmosphere closure");
        assert_eq!(graph.pipeline_pass_count(), 10);
        assert_eq!(prepared.scene_pass_count(), 10);
        assert!(matches!(
            prepared.update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Diagnostic(SyntheticLight::default()),
                1.0,
            ),
            Err(RenderError::InvalidLighting)
        ));
        let mut captures = Vec::new();
        for sample in [
            noon,
            noon,
            AtmosphereFrame::new([0.0, 0.5, -0.866], 63_500.0, [1.0; 3], 2.0, 1)
                .expect("changed intensity with cached sky"),
            AtmosphereFrame::new([0.0, 0.02, -1.0], 127_000.0, [1.0; 3], 2.0, 2).expect("sunset"),
        ] {
            prepared
                .update_frame(
                    &queue,
                    &mut target,
                    &scene,
                    FrameLighting::Atmosphere(sample),
                    1.0,
                )
                .expect("live atmosphere update");
            let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
            let mut readback = target.readback(&device);
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            prepared
                .encode_with_readback(&mut encoder, 51, &target, None, &mut readback)
                .expect("declared atmosphere passes encode");
            let submission = queue.submit([encoder.finish()]);
            prepared.commit_frame();
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: Some(std::time::Duration::from_secs(20)),
                })
                .expect("atmosphere GPU completion");
            assert!(scope.pop().await.is_none(), "no GPU resource hazard");
            let pixels = readback
                .try_collect(51)
                .expect("sky readback")
                .expect("completed pixels");
            let sky = &pixels[..128 * 16 * 4];
            assert!(sky.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
            assert!(
                sky.as_chunks::<4>()
                    .0
                    .iter()
                    .any(|p| u16::from(p[2]) > u16::from(p[0]) + 8)
            );
            captures.push(pixels);
        }
        assert_eq!(
            captures[0], captures[1],
            "cached GPU ambient survives CPU frame updates"
        );
        let centre = (35 * 128 + 64) * 4;
        assert!(
            captures[0][centre..centre + 3]
                .iter()
                .all(|channel| *channel > 8),
            "the Sun-facing-away neutral material receives diffuse sky light"
        );
        assert!(
            captures[2][centre] < captures[0][centre],
            "intensity changes refresh irradiance even with cached unit sky LUT"
        );
        let changed = captures[0]
            .as_chunks::<4>()
            .0
            .iter()
            .zip(captures[3].as_chunks::<4>().0.iter())
            .filter(|(a, b)| a[..3] != b[..3])
            .count();
        assert!(changed > 4_000, "live Sun must change the sky broadly");
    });
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn aerial_transport_changes_a_distant_surface_in_the_live_forward_pass() {
    use crate::capabilities::DeviceCapabilities;
    use crate::synthetic::{
        FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene, SyntheticView,
    };

    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real adapter required for aerial transport");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("aerial device");
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 52);
        // A 2 km observer can see the 10 km target over the curved ground;
        // a ground-level observer cannot, so its aerial integral stops early.
        let atmosphere = AtmosphereFrame::new([0.0, 0.6, -0.8], 127_000.0, [1.0; 3], 2_000.0, 1)
            .expect("fixed noon atmosphere");
        let mut centres = Vec::new();
        for distance in [100.0_f32, 10_000.0] {
            let mut scene = SyntheticScene::grid(1).expect("one neutral target");
            let scale = distance * 0.15;
            scene.instances[0].current = [
                [scale, 0.0, 0.0, 0.0],
                [0.0, scale, 0.0, 0.0],
                [0.0, 0.0, scale, 0.0],
                [0.0, 0.0, -distance, 1.0],
            ];
            scene.instances[0].previous = scene.instances[0].current;
            scene.instances[0].normal = [
                [1.0 / scale, 0.0, 0.0, 0.0],
                [0.0, 1.0 / scale, 0.0, 0.0],
                [0.0, 0.0, 1.0 / scale, 0.0],
            ];
            scene.materials[0].base_rgba = [0.8, 0.7, 0.6, 1.0];
            scene.view = SyntheticView::perspective(
                [0.0, 0.0, 2.0],
                [0.0, 0.0, -1.0],
                [0.0, 1.0, 0.0],
                1.4,
                2.0,
                0.1,
                50_000.0,
            )
            .expect("distant view");
            let mut prepared = PreparedSynthetic::prepare_with_capabilities(
                &capabilities,
                &queue,
                &scene,
                FrameLighting::Atmosphere(atmosphere),
                1.0 / 16_000.0,
            )
            .await
            .expect("aerial scene");
            let target = OffscreenTarget::new(&device, &prepared, 128, 64).expect("aerial target");
            let mut readback = target.readback(&device);
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            prepared
                .encode_with_readback(&mut encoder, 52, &target, None, &mut readback)
                .expect("aerial graph execution");
            let submission = queue.submit([encoder.finish()]);
            prepared.commit_frame();
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: Some(std::time::Duration::from_secs(20)),
                })
                .expect("aerial GPU completion");
            let pixels = readback
                .try_collect(52)
                .expect("aerial readback")
                .expect("mapped pixels");
            let horizon = &pixels[(32 * 128 + 24) * 4..][..3];
            assert!(
                u16::from(horizon[0]) < 2 * u16::from(horizon[1]),
                "the high Sun must not create a red band on the horizon: {horizon:?}"
            );
            let sun = &pixels[(3 * 128 + 64) * 4..][..3];
            assert!(
                sun.iter().all(|channel| *channel > 200),
                "the filtered solar disk must remain visible: {sun:?}"
            );
            if let Some(directory) = std::env::var_os("THRUM_G3S_AERIAL_CAPTURE_DIR") {
                let directory = std::path::PathBuf::from(directory);
                std::fs::create_dir_all(&directory).expect("aerial capture directory");
                let mut ppm = b"P6\n128 64\n255\n".to_vec();
                for pixel in pixels.as_chunks::<4>().0 {
                    ppm.extend_from_slice(&pixel[..3]);
                }
                std::fs::write(directory.join(format!("aerial-{distance:.0}.ppm")), ppm)
                    .expect("aerial visual capture");
            }
            centres.push(pixels[(32 * 128 + 64) * 4..(32 * 128 + 64) * 4 + 3].to_vec());
        }
        let near = &centres[0];
        let far = &centres[1];
        assert!(
            far[2] > near[2] + 4,
            "aerial distance must add blue sky radiance to the neutral forward surface: near={near:?}, far={far:?}"
        );
        assert!(
            i16::from(far[2]) - i16::from(far[0]) > i16::from(near[2]) - i16::from(near[0]) + 4,
            "aerial distance must increase the blue tint: near={near:?}, far={far:?}"
        );
    });
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn orthographic_sky_keeps_a_finite_solar_disk() {
    use crate::capabilities::DeviceCapabilities;
    use crate::synthetic::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene};

    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real adapter required for orthographic sky");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("orthographic device");
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 53);
        let mut scene = SyntheticScene::grid(1).expect("orthographic fixture");
        scene.instances[0].current[3][0] = 10.0;
        scene.instances[0].previous = scene.instances[0].current;
        let frame =
            AtmosphereFrame::new([0.007_812_5, 0.007_812_5, 1.0], 127_000.0, [1.0; 3], 2.0, 1)
                .expect("Sun aligned with the view");
        let mut prepared = PreparedSynthetic::prepare_with_capabilities(
            &capabilities,
            &queue,
            &scene,
            FrameLighting::Atmosphere(frame),
            1.0 / 16_000.0,
        )
        .await
        .expect("orthographic sky pipeline");
        let target = OffscreenTarget::new(&device, &prepared, 32, 32).expect("orthographic target");
        let mut readback = target.readback(&device);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared
            .encode_with_readback(&mut encoder, 53, &target, None, &mut readback)
            .expect("orthographic graph execution");
        let submission = queue.submit([encoder.finish()]);
        prepared.commit_frame();
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(20)),
            })
            .expect("orthographic GPU completion");
        let pixels = readback
            .try_collect(53)
            .expect("orthographic readback")
            .expect("mapped pixels");
        let centre = &pixels[(15 * 32 + 16) * 4..][..3];
        assert!(
            centre[0] > 200 && centre[1] > 150,
            "aligned orthographic Sun must light sky without undefined coverage: {centre:?}"
        );
    });
}
