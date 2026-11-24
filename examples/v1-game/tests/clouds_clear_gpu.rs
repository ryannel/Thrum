//! Coverage zero leaves the rendered scene byte-for-byte unchanged.

thrum_test_support::macos_gpu_test! {
    fn selecting_a_clear_sky_preserves_every_pixel() {
        use thrum_render::runtime::FrameConfig;
        use v1_game::{CloudSelection, GameHost as _, GameSetup, SceneCamera, clouds::WeatherPreset};

        let Some(gpu) = thrum_test_support::open(
            "clear weather pixels",
            Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        ) else {
            return;
        };
        let extent = [160, 90];
        let config = FrameConfig { render_extent: extent, surface: None };
        let camera = SceneCamera::perspective(
            [0.0, 40.0, 0.0], [0.0, 80.0, -200.0], 0.9, 1.0, 200_000.0,
        ).expect("view includes the ground and sky");
        let frames = pollster::block_on(async {
            let mut frames = Vec::new();
            for clouds in [CloudSelection::Off, CloudSelection::Sky] {
                let mut host = GameSetup {
                    clouds,
                    cloud_weather: WeatherPreset::Clear.parameters(),
                    ..GameSetup::default()
                }.prepare(&gpu.device, &gpu.queue, &gpu.capabilities, config)
                    .await.expect("game setup");
                host.set_scene_camera(camera).expect("matched camera");
                host.set_time_of_day(0.5).expect("matched Sun");
                let mut pixels = Vec::new();
                for tick in 0..=3 {
                    host.drive(std::time::Duration::from_millis(tick * 17), None)
                        .expect("advance weather");
                    pixels.push(host.capture_pixels().expect("read the finished frame"));
                }
                if clouds == CloudSelection::Sky {
                    assert!(host.cloud_sky_products().is_some(), "Sky trace submitted");
                }
                frames.push(pixels);
            }
            frames
        });
        for (tick, (absent, clear)) in frames[0].iter().zip(&frames[1]).enumerate() {
            assert_eq!(absent.len(), extent.into_iter().product::<u32>() as usize * 4);
            let differing = absent.iter().zip(clear).filter(|(a, b)| a != b).count();
            assert_eq!(differing, 0, "different RGBA bytes at frame {tick}");
        }
    }
}
