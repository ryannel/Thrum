//! Actual resident cooked meshlets in a requested slot and an ordinary main view.
mod support;

use support::{Fixture, Witness};
use thrum_asset_gpu::CookedDrawPath;
use thrum_render::{
    capabilities::DeviceCapabilities,
    contribution::{SecondaryViewRequest, ViewAir, ViewClipPlane},
};

#[test]
fn a_cooked_view_matches_the_ordinary_camera_on_each_draw_path() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "cooked secondary view",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let native = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    for (caps, path) in [
        (native.clone(), CookedDrawPath::Indirect),
        (
            native.clone().without_indirect_execution(),
            CookedDrawPath::Direct,
        ),
    ] {
        prove_path(&device, &queue, &caps, path);
    }
}

fn prove_path(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    path: CookedDrawPath,
) {
    let fixture = Fixture {
        caps,
        device,
        queue,
    };
    let camera = support::camera([0.4, 1.7, 5.0], [0.0, 0.75, 0.0]);
    let main = support::camera([0.4, 3.7, 5.0], [2.0, 3.7, 8.0]);
    for cascades in [0, 1, 2] {
        let requested = SecondaryViewRequest::new(&camera, [32, 32], None, ViewAir::Camera)
            .expect("request")
            .with_shadows(cascades, 128.0)
            .expect("profile");
        let expected = fixture.draw(
            &camera
                .with_sun_shadow_profile(cascades, 128.0)
                .expect("profile"),
            32,
            Witness::Main,
        );
        let actual = fixture.draw(&main, 64, Witness::View(Box::new(requested)));
        assert_eq!(actual.path, path);
        assert_eq!(expected.path, path);
        assert_eq!(actual.secondary, [(0, path)]);
        assert_eq!(expected.secondary, []);
        compare(&expected.pixels, &actual.pixels);
    }
    let clip = ViewClipPlane::new([0.0, 1.0, 0.0, -50.0]).expect("plane");
    let clipped =
        SecondaryViewRequest::new(&camera, [32, 32], Some(clip), ViewAir::Camera).expect("clip");
    let empty = fixture.draw(&main, 64, Witness::View(Box::new(clipped)));
    assert!(empty.pixels.iter().all(|pixel| pixel[4] == 0.0));
    let request =
        SecondaryViewRequest::new(&camera, [32, 32], None, ViewAir::Camera).expect("request");
    let too_small = fixture.draw(
        &main,
        64,
        Witness::View(Box::new(request.with_least_pixels(128.0).expect("size"))),
    );
    assert_eq!(
        too_small.pixels.iter().all(|pixel| pixel[4] == 0.0),
        path == CookedDrawPath::Indirect
    );
    let no_slot = fixture.draw(&camera, 32, Witness::Main);
    let idle = fixture.draw(&camera, 32, Witness::Idle);
    assert_eq!(no_slot.pixels, idle.pixels);
    assert_eq!(idle.secondary, []);
    assert_eq!(
        no_slot.attachment_pipelines,
        no_slot
            .attachment_pipelines
            .map(|_| if path == CookedDrawPath::Indirect {
                5
            } else {
                4
            }),
        "ordinary mesh, shadow and volume, the readback observer and the indirect-only meshlet cull",
    );
    let away =
        SecondaryViewRequest::new(&main, [32, 32], None, ViewAir::Camera).expect("away view");
    let away = fixture.draw(&main, 64, Witness::View(Box::new(away)));
    assert!(
        away.pixels.iter().all(|pixel| pixel[4] == 0.0),
        "secondary frustum faces away"
    );
}

#[expect(
    clippy::print_stderr,
    reason = "report both stored-format maxima in evidence"
)]
fn compare(expected: &[[f32; 8]], actual: &[[f32; 8]]) {
    let mut covered = 0;
    let mut maximum = [0, 0];
    for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
        assert_eq!(actual[3] > 0.0, expected[4] > 0.0, "coverage {index}");
        if expected[4] == 0.0 {
            continue;
        }
        covered += 1;
        for c in 0..3 {
            let steps = half::f16::from_f32(expected[c])
                .to_bits()
                .abs_diff(half::f16::from_f32(actual[c]).to_bits());
            maximum[0] = maximum[0].max(u32::from(steps));
            assert!(
                steps <= 1,
                "pixel {index} channel {c}: {} vs {}",
                expected[c],
                actual[c]
            );
        }
        let steps = expected[4].to_bits().abs_diff(actual[4].to_bits());
        maximum[1] = maximum[1].max(steps);
        assert!(steps <= 1, "pixel {index} depth");
    }
    assert!(covered > 10);
    eprintln!("{covered} covered pixels, max colour/depth steps {maximum:?}");
}
