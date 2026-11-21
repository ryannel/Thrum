//! The two air stretches composed against stored ordinary main-view colours.

use glam::Vec3;
use mirror_scene::{Pool, PoolPlane};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thrum_app::sky::{Camera, StillWorld};
use thrum_render::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    contribution::SceneContributions,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{CaptureLimits, FrameConfig, FrameHost, SceneFrame, SceneSource, SurfaceOutput},
    synthetic::{CaptureSource, FrameLighting, OUTPUT_FORMAT, SyntheticScene, SyntheticView},
};

mod air;
mod panel;
mod retire;
#[cfg(target_arch = "wasm32")]
mod view;

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
/// Compare stored colour and depth and validate the pool's two air stretches.
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
    let report = view::prove(&gpu.1, &gpu.2, &caps, &store).await;
    let margins = prove(&gpu.1, &gpu.2, &caps).await;
    Ok(format!(
        "{report}\nMarker actual/allowance max, missing/allowance min, double/allowance min: {margins:?}"
    ))
}

const SIDE: u32 = 65;
const CENTER: [usize; 2] = [32; 2];
const LEVEL: f32 = 50.0;
const FOV: f32 = 20.0;
const LUMINANCE: f32 = 40.0;
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

struct Fixture<'a> {
    device: &'a wgpu::Device,
    caps: &'a DeviceCapabilities,
    queue: &'a wgpu::Queue,
}

#[derive(Clone, Copy)]
struct Panel {
    center: Vec3,
    half: Vec3,
    radiance: f32,
}

impl Panel {
    #[expect(clippy::print_stderr, reason = "record the measured main-frame margin")]
    fn assert_above_frame(self, view: Camera) {
        let eye = Vec3::from_array(view.position);
        let forward = (Vec3::from_array(view.target) - eye).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward);
        let top = (view.vertical_fov_degrees.to_radians() * 0.5).tan();
        let margin = [-1.0, 1.0]
            .into_iter()
            .map(|sign| {
                // Both vertices on this row have the same projected height.
                let corner = self.center + Vec3::Z * self.half.z + Vec3::Y * sign * self.half.y;
                let offset = corner - eye;
                let depth = offset.dot(forward);
                assert!(depth > 0.0, "marker is in front of the camera");
                offset.dot(up) / (depth * top) - 1.0
            })
            .fold(f32::INFINITY, f32::min);
        assert!(
            margin > 0.0,
            "entire marker must project above the main frame: {margin}"
        );
        eprintln!(
            "marker={} lowest edge is {margin} NDC above the main frame",
            self.radiance
        );
    }
}

struct Image {
    bytes: Vec<u8>,
    scale: f32,
    quanta: [[f32; 4]; 2],
}

impl Image {
    fn pixel(&self, [x, y]: [usize; 2]) -> [f32; 3] {
        let offset = (y * SIDE as usize + x) * 8;
        std::array::from_fn(|c| {
            let bits = u16::from_le_bytes(
                self.bytes[offset + c * 2..offset + c * 2 + 2]
                    .try_into()
                    .expect("half"),
            );
            half::f16::from_bits(bits).to_f32()
        })
    }

    fn air_allowance(&self, channel: usize, radiance: f32) -> f32 {
        let [scatter, transmission] = self.quanta;
        // Four filtered reads in the plain decoder, with the volume's RGB
        // sky and night-alpha storage gains carried through the decode.
        4.0 * self.scale
            * (scatter[channel] * (100_000.0 / 1024.0)
                + transmission[3] / 65536.0
                + transmission[channel] * radiance)
    }
}

impl Fixture<'_> {
    async fn draw(&self, camera: Camera, panel: Panel, exposure: f32, pool: Option<Pool>) -> Image {
        let view = SyntheticView::perspective(
            camera.position,
            camera.target,
            [0.0, 1.0, 0.0],
            camera.vertical_fov_degrees.to_radians(),
            1.0,
            0.1,
            32_000.0,
        )
        .expect("view")
        .with_incident_exposure(0.0, exposure, exposure)
        .expect("exposure");
        let light = FrameLighting::Atmosphere(
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
        let mut scene = SyntheticScene::grid(1).expect("marker scene");
        scene.view = view;
        scene.instances[0].deactivate();
        scene.sun_shadows = false;
        let mut host = Box::new(
            Box::pin(FrameHost::new(
                self.caps,
                self.queue,
                FrameConfig {
                    render_extent: [SIDE; 2],
                    surface: Some(SurfaceProfile {
                        format: OUTPUT_FORMAT,
                        extent: [SIDE; 2],
                    }),
                },
                StillWorld::default(),
                Source {
                    frame: SceneFrame {
                        view,
                        lighting: light,
                    },
                },
                scene,
                1.0,
            ))
            .await
            .expect("host"),
        );
        let mut parts: Vec<Box<dyn thrum_render::contribution::SceneContribution>> =
            vec![Box::new(panel::PanelDraw::new(panel))];
        #[cfg_attr(
            target_arch = "wasm32",
            expect(
                clippy::arc_with_non_send_sync,
                reason = "the same witness also runs on native Metal"
            )
        )]
        let air = Arc::new(Mutex::new(None));
        if let Some(pool) = pool {
            parts.push(Box::new(pool));
        } else {
            let front = panel.center
                + if panel.half.y < panel.half.z {
                    Vec3::Y * panel.half.y
                } else {
                    Vec3::Z * panel.half.z
                };
            let distance = (front - Vec3::from_array(camera.position)).length();
            // Across this test's geometry, layer bounds 31.25 and 125 m
            // bracket the 30, 100 and 130 m samples in successive pairs.
            let layer = if distance < 31.25 {
                0
            } else if distance < 125.0 {
                1
            } else {
                2
            };
            parts.push(Box::new(air::Air::new(layer, Arc::clone(&air))));
        }
        host.set_scene_contribution(Box::new(SceneContributions::new(parts)))
            .expect("diagnostic scene");
        host.enable_submitted_capture_from(1, CaptureLimits::default(), CaptureSource::SceneLinear)
            .expect("stored scene colour tap");
        let output = self
            .device
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
        let air = air.lock().expect("witness").clone();
        let quanta = if let Some(buffer) = air {
            air::read(self.device, self.queue, &buffer).await
        } else {
            [[0.0; 4]; 2]
        };
        Image {
            bytes: image.rgba,
            scale: f32::from_bits(image.exposure_gpu_words.expect("submitted exposure")[0]),
            quanta,
        }
    }
}

fn step(value: f32) -> f32 {
    let bits = half::f16::from_f32(value.abs()).to_bits();
    half::f16::from_bits(bits + 1).to_f32() - half::f16::from_bits(bits).to_f32()
}

#[expect(
    clippy::print_stderr,
    reason = "report measured differences beside the rounding allowance"
)]
fn compare(
    black: &Image,
    white: &Image,
    view: &Image,
    pool: &Image,
    marker: f32,
    whole_path: Option<[f32; 3]>,
) -> [f32; 3] {
    let [b, w] = [black.pixel(CENTER), white.pixel(CENTER)];
    let center = view.pixel(CENTER);
    let mut margins = [0.0_f32, f32::INFINITY, f32::INFINITY];
    for (c, actual) in pool.pixel(CENTER).into_iter().enumerate() {
        let tc = (w[c] - b[c]) / (black.scale * LUMINANCE);
        let b_error = step(b[c]) + black.air_allowance(c, 0.0);
        let w_error = step(w[c]) + white.air_allowance(c, LUMINANCE);
        let tc_error = (b_error + w_error) / (black.scale * LUMINANCE);
        // The geometric centre lands exactly on a texel centre. Either axis
        // can round to either side, so include its immediate neighbours.
        let mut low = f32::INFINITY;
        let mut high = f32::NEG_INFINITY;
        let mut v_step = 0.0_f32;
        for y in 31..=33 {
            for x in 31..=33 {
                let value = view.pixel([x, y])[c];
                low = low.min(value);
                high = high.max(value);
                v_step = v_step.max(step(value));
            }
        }
        let v_error = v_step + view.air_allowance(c, marker);
        let bound = step(actual)
            + b_error
            + tc_error * (high + v_error)
            + tc.abs() * v_error
            + black.air_allowance(c, high / black.scale)
            + tc.abs() * view.air_allowance(c, marker);
        let expected = b[c] + tc * center[c];
        let composed = [b[c] + tc * low, b[c] + tc * high];
        let difference = (actual - actual.clamp(composed[0], composed[1])).abs();
        eprintln!(
            "scale={} marker={marker} channel={c} V={} range={low}..{high} B={} W={} actual={actual} expected={expected} centre delta={} range difference={difference} allowance={bound}",
            black.scale,
            center[c],
            b[c],
            w[c],
            (actual - expected).abs()
        );
        assert!(difference <= bound, "two stretches");
        margins[0] = margins[0].max(difference / bound);
        if let Some(whole_path) = whole_path {
            let missing = composed[0] - high;
            let double = b[c] + tc * whole_path[c] - composed[1];
            eprintln!(
                "wrong path differences missing={missing} double={double}, allowance={bound}"
            );
            assert!(missing > bound, "missing camera stretch");
            assert!(double > bound, "double camera stretch");
            margins[1] = margins[1].min(missing / bound);
            margins[2] = margins[2].min(double / bound);
        }
    }
    margins
}

#[cfg(target_os = "macos")]
#[test]
fn the_pool_composes_both_stretches_once() {
    let Some(gpu) =
        thrum_test_support::gpu("pool transport composition", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let report = pollster::block_on(prove(&device, &queue, &caps));
    eprintln!("worst composition difference relative to its allowance: {report:?}");
}

async fn prove(device: &wgpu::Device, queue: &wgpu::Queue, caps: &DeviceCapabilities) -> [f32; 3] {
    let fixture = Fixture {
        device,
        caps,
        queue,
    };
    let angle = 10.0_f32.to_radians();
    let ray = Vec3::new(0.0, angle.sin(), -angle.cos());
    let crossing = Vec3::new(0.0, LEVEL, 0.0);
    let eye = crossing + Vec3::new(0.0, ray.y, -ray.z) * 30.0;
    let virtual_eye = crossing - ray * 30.0;
    let main = Camera::look_at(eye.to_array(), crossing.to_array(), FOV);
    let reference = Camera::look_at(
        crossing.to_array(),
        (crossing + ray * 100.0).to_array(),
        FOV,
    );
    let mirror = Camera::look_at(
        virtual_eye.to_array(),
        (crossing + ray * 100.0).to_array(),
        FOV,
    );
    let swatch = Panel {
        center: crossing - Vec3::Y * 0.0625,
        half: Vec3::new(10.0, 0.0625, 10.0),
        radiance: 0.0,
    };
    let mixed = [0.0, LUMINANCE].map(|radiance| Panel {
        center: crossing + ray * 100.0 - Vec3::Z * 0.0625,
        half: Vec3::new(10.0, 10.0, 0.0625),
        radiance,
    });
    for marker in mixed {
        marker.assert_above_frame(main);
    }
    let mut scales = Vec::new();
    let mut worst = [0.0_f32, f32::INFINITY, f32::INFINITY];
    for exposure in [6.0, 2.0] {
        let black = fixture.draw(main, swatch, exposure, None).await;
        let white = fixture
            .draw(
                main,
                Panel {
                    radiance: LUMINANCE,
                    ..swatch
                },
                exposure,
                None,
            )
            .await;
        scales.push(black.scale);
        for marker in mixed {
            let v = fixture.draw(reference, marker, exposure, None).await;
            let pool = fixture
                .draw(main, marker, exposure, Some(Pool::new(POOL)))
                .await;
            let whole_path = if marker.radiance == 0.0 {
                Some(
                    fixture
                        .draw(mirror, marker, exposure, None)
                        .await
                        .pixel(CENTER),
                )
            } else {
                None
            };
            let errors = compare(&black, &white, &v, &pool, marker.radiance, whole_path);
            worst[0] = worst[0].max(errors[0]);
            worst[1] = worst[1].min(errors[1]);
            worst[2] = worst[2].min(errors[2]);
        }
    }
    assert!((scales[1] / scales[0] - 16.0).abs() < 1e-5);
    worst
}
