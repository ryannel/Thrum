//! The ordinary aerial pass supplies every reference texel, those of lines
//! that never cross the plane included. All reads use contribution bindings.
mod support;
use support::*;
use thrum_render::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    contribution::{SecondaryViewRequest, ViewAir, ViewClipPlane},
    synthetic::{FrameLighting, SyntheticScene, SyntheticView},
};

fn view(height: f32, pitch: f32) -> SyntheticView {
    SyntheticView::perspective(
        [0.0, height, 0.0],
        [0.0, height + pitch.sin(), -pitch.cos()],
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        5000.0,
    )
    .expect("camera")
}
fn light(height: f32) -> FrameLighting {
    FrameLighting::Atmosphere(
        AtmosphereFrame::new([0.5, 0.707, 0.5], 100_000.0, [1.0; 3], height, 1).expect("air"),
    )
}
#[test]
fn every_stored_plane_column_matches_the_ordinary_aerial_pass() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "secondary aerial volume",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    pollster::block_on(async {
        let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        for height in [0.0, 50.0] {
            for below in [2.0, 40.0] {
                for pitch in [0.0, 30.0_f32.to_radians()] {
                    let mut scene = SyntheticScene::grid(1).expect("scene");
                    scene.view = view(100.0, 0.0);
                    let mut actual = host_with_extent(
                        &capabilities,
                        &queue,
                        scene.clone(),
                        light(100.0),
                        AIR_EXTENT,
                    )
                    .await;
                    let (probe, outputs) = AirProbe::new(Some(
                        &SecondaryViewRequest::new(
                            &view(height - below, pitch),
                            AIR_EXTENT,
                            Some(ViewClipPlane::new([0.0, 1.0, 0.0, -height]).expect("plane")),
                            ViewAir::Plane,
                        )
                        .expect("ask"),
                    ));
                    actual
                        .set_scene_contribution(Box::new(probe))
                        .expect("attach actual");
                    actual.capture_pixels().expect("actual volume");
                    let actual = air_output(
                        &device,
                        &queue,
                        &outputs.lock().expect("actual outputs").outputs[1],
                    );
                    scene.view = view(height, pitch);
                    let mut reference =
                        host_with_extent(&capabilities, &queue, scene, light(height), AIR_EXTENT)
                            .await;
                    let (probe, outputs) = AirProbe::new(None);
                    reference
                        .set_scene_contribution(Box::new(probe))
                        .expect("ordinary observer");
                    reference.capture_pixels().expect("ordinary volume");
                    let reference = air_output(
                        &device,
                        &queue,
                        &outputs.lock().expect("reference outputs").outputs[0],
                    );
                    assert_eq!(actual.len(), reference.len());
                    assert_eq!(actual.len(), 270_336);
                    if height > 0.0 && pitch.to_bits() == 0.0_f32.to_bits() {
                        assert!(
                            reference[(32 * 32 + 24 * 32 + 16) * 4] > 0.0,
                            "never-crossing rows must contain scattering"
                        );
                    }
                    let mut maximum = 0.0_f32;
                    for (index, (&a, &e)) in actual.iter().zip(&reference).enumerate() {
                        let stored = half::f16::from_f32(e);
                        let step = (half::f16::from_bits(stored.to_bits() + 1).to_f32() - e).abs();
                        let difference = (a - e).abs();
                        maximum = maximum.max(difference / step);
                        let adjacent = half::f16::from_f32(a).to_bits().abs_diff(stored.to_bits());
                        assert!(
                            difference.to_bits() == 0 || adjacent <= 1,
                            "height={height} below={below} pitch={pitch} channel={index}: actual={a} reference={e} difference={difference} step={step}"
                        );
                    }
                    eprintln!(
                        "height={height} below={below} pitch={pitch}: {} stored channels, maximum {maximum} steps",
                        actual.len()
                    );
                }
            }
        }
    });
}
