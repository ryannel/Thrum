//! The pool prepares and draws on Metal, and in Chrome a secondary view equals
//! the main view.

mod retire;
#[cfg(target_arch = "wasm32")]
mod view;

#[cfg(target_arch = "wasm32")]
use thrum_render::capabilities::DeviceCapabilities;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(message: &str);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
/// Compare a secondary view's stored colour and depth with the main view's.
///
/// # Errors
/// Reports failure to create a browser WebGPU device.
///
/// # Panics
/// Panics if WebGPU selects a different recipe or a comparison fails.
pub async fn prove_views(store: String) -> Result<String, JsValue> {
    std::panic::set_hook(Box::new(|info| console_error(&info.to_string())));
    let gpu = thrum_test_support::browser_gpu()
        .await
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let caps = DeviceCapabilities::capture(&gpu.0.adapter, &gpu.1, 1);
    assert!(matches!(
        caps.recipe(),
        Ok(thrum_render::capabilities::Recipe::Portable)
    ));
    Ok(view::prove(&gpu.1, &gpu.2, &caps, &store).await)
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(
    not(target_os = "macos"),
    allow(dead_code, reason = "the pool's GPU test runs on Metal")
)]
mod draw {
    use mirror_scene::{Pool, PoolPlane};
    use std::time::Duration;
    use thrum_app::sky::{Camera, StillWorld};
    use thrum_render::{
        atmosphere::AtmosphereFrame,
        capabilities::DeviceCapabilities,
        extract::{ExtractReport, RenderExtractor, SourceTick},
        present::SurfaceProfile,
        runtime::{CaptureLimits, FrameConfig, FrameHost, SceneFrame, SceneSource, SurfaceOutput},
        synthetic::{CaptureSource, FrameLighting, OUTPUT_FORMAT, SyntheticScene, SyntheticView},
    };

    use super::retire;

    const SIDE: u32 = 33;
    const LEVEL: f32 = 50.0;
    const POOL: PoolPlane = PoolPlane {
        level: LEVEL,
        min: [-20.0; 2],
        max: [20.0; 2],
    };

    struct Source {
        frame: SceneFrame,
    }

    impl SceneSource<SourceTick> for Source {
        fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
            into.extract(*page, [0.0; 3], |_| {})
        }
        fn frame(
            &self,
            _: SourceTick,
            _: f32,
            _: [u32; 2],
            _: Option<FrameLighting>,
        ) -> Result<SceneFrame, String> {
            Ok(SceneFrame {
                view: self.frame.view,
                lighting: self.frame.lighting,
            })
        }
    }

    /// Stored scene-linear colour of one submitted frame, as RGB.
    async fn draw(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        caps: &DeviceCapabilities,
        camera: Camera,
        pool: Option<Pool>,
    ) -> Vec<[f32; 3]> {
        let view = SyntheticView::perspective(
            camera.position,
            camera.target,
            [0.0, 1.0, 0.0],
            camera.vertical_fov_degrees.to_radians(),
            1.0,
            0.1,
            32_000.0,
        )
        .expect("view");
        let lighting = FrameLighting::Atmosphere(
            AtmosphereFrame::new(
                [
                    std::f32::consts::FRAC_1_SQRT_2,
                    std::f32::consts::FRAC_1_SQRT_2,
                    0.0,
                ],
                100_000.0,
                [1.0; 3],
                camera.position[1],
                1,
            )
            .expect("crosswise Sun"),
        );
        let mut scene = SyntheticScene::grid(1).expect("empty scene");
        scene.view = view;
        scene.instances[0].deactivate();
        scene.sun_shadows = false;
        let mut host = Box::new(
            Box::pin(FrameHost::new(
                caps,
                queue,
                FrameConfig {
                    render_extent: [SIDE; 2],
                    surface: Some(SurfaceProfile {
                        format: OUTPUT_FORMAT,
                        extent: [SIDE; 2],
                    }),
                },
                StillWorld::default(),
                Source {
                    frame: SceneFrame { view, lighting },
                },
                scene,
                1.0,
            ))
            .await
            .expect("host"),
        );
        if let Some(pool) = pool {
            host.set_scene_contribution(Box::new(pool))
                .expect("the pool prepares");
        }
        host.enable_submitted_capture_from(1, CaptureLimits::default(), CaptureSource::SceneLinear)
            .expect("stored scene colour tap");
        let output = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("pool test presenter output"),
                size: wgpu::Extent3d {
                    width: SIDE,
                    height: SIDE,
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
        let report = host
            .drive(
                Duration::ZERO,
                Some(&SurfaceOutput {
                    view: &output,
                    extent: [SIDE; 2],
                }),
            )
            .expect("ordinary main submission");
        assert!(report.submitted);
        let deadline = retire::Instant::now() + Duration::from_secs(15);
        let image = loop {
            while host.poll_completed().expect("completed frame").is_some() {}
            if let Some(image) = host.try_submitted_image().expect("readback") {
                break image;
            }
            assert!(retire::Instant::now() < deadline, "missing submitted image");
            retire::pause().await;
        };
        assert_eq!(image.extent, [SIDE; 2]);
        image
            .rgba
            .as_chunks::<8>()
            .0
            .iter()
            .map(|texel| {
                std::array::from_fn(|c| {
                    half::f16::from_le_bytes([texel[c * 2], texel[c * 2 + 1]]).to_f32()
                })
            })
            .collect()
    }

    /// Looking down onto the pool, every pixel of the frame is pool surface.
    /// Drawn with and without the pool, the frame must change: a pool that is
    /// never prepared, or prepared but draws nothing, leaves it as it was.
    #[cfg(target_os = "macos")]
    #[test]
    fn the_pool_prepares_and_draws() {
        let Some(thrum_test_support::Opened {
            gpu, device, queue, ..
        }) = thrum_test_support::open(
            "mirror pool draw",
            Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        )
        else {
            return;
        };
        let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        // 30 m from the pool's centre, 30 degrees above it: a 20 degree frame
        // lies wholly inside the 40 m square.
        let angle = 30.0_f32.to_radians();
        let eye = [0.0, LEVEL + 30.0 * angle.sin(), 30.0 * angle.cos()];
        let camera = Camera::look_at(eye, [0.0, LEVEL, 0.0], 20.0);
        let (without, with) = pollster::block_on(async {
            (
                draw(&device, &queue, &caps, camera, None).await,
                draw(&device, &queue, &caps, camera, Some(Pool::new(POOL))).await,
            )
        });
        assert!(
            with.iter().flatten().all(|value| value.is_finite()),
            "the pool drew non-finite colour"
        );
        let changed = without
            .iter()
            .zip(&with)
            .filter(|(a, b)| {
                a.iter()
                    .zip(b.iter())
                    .any(|(a, b)| (a - b).abs() > 1e-3 * a.abs().max(b.abs()).max(1e-3))
            })
            .count();
        assert!(
            changed * 10 >= without.len() * 9,
            "the pool changed only {changed} of {} pixels",
            without.len()
        );
    }
}
