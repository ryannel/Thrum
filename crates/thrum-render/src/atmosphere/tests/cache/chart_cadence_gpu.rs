use super::*;

mod haze;
mod mixed_overlay;
use thrum_test_support::readback::{Mip, Readback};

fn sun(elevation: f32, illuminance: f32, colour: [f32; 3]) -> AtmosphereFrame {
    let (sin, cos) = elevation.to_radians().sin_cos();
    let colour = glam::Vec3::from_array(colour);
    let colour = (colour / colour.dot(crate::light::LUMA)).to_array();
    AtmosphereFrame::new([cos, sin, 0.0], illuminance, colour, 40.0, 1).unwrap()
}

fn moving(step: f32) -> AtmosphereFrame {
    sun(30.0 + step * 0.02, 90_000.0, [1.0, 0.92, 0.83])
}

struct Probe<'a> {
    scene: SyntheticScene,
    prepared: PreparedSynthetic,
    target: OffscreenTarget,
    caps: &'a DeviceCapabilities,
    queue: &'a wgpu::Queue,
}

impl<'a> Probe<'a> {
    async fn new(
        caps: &'a DeviceCapabilities,
        queue: &'a wgpu::Queue,
        frame: AtmosphereFrame,
        overlay: bool,
    ) -> Self {
        let mut scene = SyntheticScene::grid(1).unwrap();
        scene.sky_overlay = overlay;
        scene.sun_shadows = false;
        Self::prepare(caps, queue, frame, scene).await
    }

    async fn prepare(
        caps: &'a DeviceCapabilities,
        queue: &'a wgpu::Queue,
        frame: AtmosphereFrame,
        scene: SyntheticScene,
    ) -> Self {
        let prepared = PreparedSynthetic::prepare_with_capabilities(
            caps,
            queue,
            &scene,
            FrameLighting::Atmosphere(frame),
            1.0,
        )
        .await
        .unwrap();
        let target = OffscreenTarget::new(caps.device(), &prepared, 16, 16).unwrap();
        Self {
            scene,
            prepared,
            target,
            caps,
            queue,
        }
    }

    fn cadence(&mut self, cadence: u32) {
        self.prepared
            .set_atmosphere_quality("atmosphere.chart cadence", cadence)
            .unwrap()
            .unwrap();
        self.prepared.commit_atmosphere_quality();
    }

    fn update(&mut self, frame: AtmosphereFrame) {
        self.prepared
            .update_frame(
                self.queue,
                &mut self.target,
                &self.scene,
                FrameLighting::Atmosphere(frame),
                1.0,
            )
            .unwrap();
    }

    fn draw(&mut self, frame: AtmosphereFrame) -> Vec<String> {
        self.update(frame);
        submitted_names(&mut self.prepared, &self.target, self.caps, self.queue)
    }

    fn overlay(&self, rgba: [u16; 4]) {
        let target = self.prepared.overlay_witness();
        self.queue.write_texture(
            target.as_image_copy(),
            bytemuck::cast_slice(&vec![rgba; (target.width() * target.height() * 6) as usize]),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(target.width() * 8),
                rows_per_image: Some(target.height()),
            },
            target.size(),
        );
    }

    fn products(&self) -> Vec<Vec<u8>> {
        let device = self.caps.device();
        let (_, [_, _, sky, _]) = self.prepared.atmosphere_witness().unwrap();
        let (reflection, diffuse) = self.prepared.chart_schedule_witness();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut copies = vec![
            Readback::texture(device, &mut encoder, sky, Mip::level(0), ..),
            Readback::buffer(device, &mut encoder, diffuse, ..),
        ];
        copies.extend((0..7).map(|level| {
            Readback::texture(device, &mut encoder, reflection, Mip::level(level), ..)
        }));
        let submission = self.queue.submit([encoder.finish()]);
        copies
            .into_iter()
            .map(|copy| copy.wait(device, submission.clone()))
            .collect()
    }
}

fn chart_names(names: &[String]) -> impl Iterator<Item = &str> {
    names.iter().map(String::as_str).filter(|name| {
        matches!(
            *name,
            "atmosphere sky view" | "atmosphere diffuse irradiance"
        ) || name.starts_with("atmosphere reflection mip ")
    })
}

fn ordered_names() -> Vec<&'static str> {
    [AtmosphereStage::SkyView, AtmosphereStage::Diffuse]
        .into_iter()
        .chain((0..7).map(AtmosphereStage::Reflection))
        .map(AtmosphereStage::name)
        .collect()
}

#[test]
fn four_moving_metal_submissions_match_the_first_input_control() {
    let Some(crate::test_device::Opened {
        gpu, device, queue, ..
    }) = crate::test_device::open(
        "Metal chart cadence snapshot",
        Some(wgpu::Backend::Metal),
        crate::capabilities::DeviceProfile::Portable,
        &crate::capabilities::DeviceRestriction::default(),
    )
    else {
        return;
    };
    crate::timestamp_permit::hold();
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        // A cloudy direct upload stays byte-identical across both arms.
        let mut spread = Probe::new(&caps, &queue, moving(0.0), true).await;
        let mut control = Probe::new(&caps, &queue, moving(1.0), true).await;
        for probe in [&spread, &control] {
            probe.overlay([0x5400, 0x5200, 0x5000, 0x3800]);
        }
        spread.draw(moving(0.0));
        control.draw(moving(1.0));
        let expected = control.products();
        spread.cadence(4);
        let expected_stages = ordered_names();
        for (cadence, first) in [(4_u32, 1_u32), (6, 5), (8, 11)] {
            spread.cadence(cadence);
            let mut stages = Vec::new();
            for step in first..first + cadence {
                let names = spread.draw(moving(step as f32));
                let frame_stages = chart_names(&names).collect::<Vec<_>>();
                assert!(frame_stages.len() <= 9_usize.div_ceil(cadence as usize));
                stages.extend(frame_stages.into_iter().map(str::to_owned));
            }
            assert_eq!(stages, expected_stages, "cadence {cadence}");
            if cadence == 4 {
                for (index, (actual, expected)) in
                    spread.products().iter().zip(&expected).enumerate()
                {
                    assert!(
                        actual == expected,
                        "chart {index} differs from the independent first-frame control"
                    );
                }
            }
        }
    });
}

#[test]
fn cadence_one_updates_diffuse_on_light_edits() {
    let Some(crate::test_device::Opened {
        gpu, device, queue, ..
    }) = crate::test_device::open(
        "Metal cadence-one lighting edits",
        Some(wgpu::Backend::Metal),
        crate::capabilities::DeviceProfile::Portable,
        &crate::capabilities::DeviceRestriction::default(),
    )
    else {
        return;
    };
    crate::timestamp_permit::hold();
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        let initial = moving(0.0);
        let mut probe = Probe::new(&caps, &queue, initial, false).await;
        probe.draw(initial);
        let products = probe.prepared.environment_products_witness(&queue).unwrap();
        let surface = crate::atmosphere::products::tests::observer::Probe::new(
            &device,
            &products,
            "",
            ["vec4(environment_diffuse(vec3(0.0, 1.0, 0.0)), 0.0)"],
        );
        let sample = || surface.sample(&device, &queue, &products, &[[0.0; 4]], 0)[0][0];
        let mut previous = sample();
        for frame in [
            sun(30.0, 45_000.0, [1.0, 0.92, 0.83]),
            sun(30.0, 45_000.0, [1.0, 0.45, 0.83]),
        ] {
            let names = probe.draw(frame);
            assert_eq!(
                chart_names(&names).collect::<Vec<_>>(),
                [AtmosphereStage::Diffuse.name()]
            );
            let diffuse = sample();
            for band in 0..3 {
                let previous = previous[band];
                let current = diffuse[band];
                assert!(current.is_finite() && current > 0.0);
                assert!((current - previous).abs() > previous * 0.01);
            }
            previous = diffuse;
        }
    });
}
