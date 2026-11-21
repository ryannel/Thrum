//! CPU ray/box visibility classifies real receiver pixels. Small near casters
//! need the view's cascades; distant casters need the shared height field.
mod support;
use glam::{Mat4, Vec3, Vec4};
use support::*;
use thrum_render::{
    abi::InstanceRecord,
    capabilities::DeviceCapabilities,
    contribution::{SecondaryViewRecord, SecondaryViewRequest, ViewAir},
    synthetic::{FrameLighting, SyntheticLight, SyntheticScene, SyntheticView},
};

fn instance(slot: u32, center: Vec3, half: Vec3) -> InstanceRecord {
    let mut value = InstanceRecord::identity(slot, 0);
    for axis in 0..3 {
        value.current[axis][axis] = half[axis];
        value.normal[axis][axis] = half[axis].recip();
        value.current[3][axis] = center[axis];
    }
    value.previous = value.current;
    value
}
fn camera(forward: f32) -> SyntheticView {
    SyntheticView::perspective(
        [0.0, 1000.0, 0.0],
        [0.0, 0.0, forward * 3000.0],
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        5000.0,
    )
    .expect("camera")
}
fn blocked(point: Vec3, sun: Vec3, center: Vec3, half: Vec3) -> bool {
    let low = (center - half - point) / sun;
    let high = (center + half - point) / sun;
    low.min(high).max_element().max(0.0) < low.max(high).min_element()
}
async fn draw(
    device: &wgpu::Device,
    cap: &DeviceCapabilities,
    queue: &wgpu::Queue,
    scene: SyntheticScene,
    count: u32,
) -> Vec<u8> {
    let mut host = host_with_extent(
        cap,
        queue,
        scene,
        FrameLighting::Diagnostic(SyntheticLight {
            direction_to_light: Vec3::ONE.normalize().to_array(),
            irradiance: [1.0; 3],
        }),
        [256, 256],
    )
    .await;
    let (observer, control) = Observer::new(1);
    control.lock().expect("control").requests[0] = Some(
        SecondaryViewRequest::new(&camera(-1.0), [256, 256], None, ViewAir::Camera)
            .expect("request")
            .with_shadows(count, 64.0)
            .expect("Sun profile"),
    );
    host.set_scene_contribution(Box::new(observer))
        .expect("observer");
    host.capture_pixels().expect("view");
    output(device, queue, &control.lock().expect("control").outputs[0])
}
#[test]
fn own_cascades_and_world_height_field_shadow_receivers_outside_the_main_frustum() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "secondary Sun visibility",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    pollster::block_on(async {
        let cap = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        let sun = Vec3::ONE.normalize();
        let casters = [
            (Vec3::new(0.35, 995.35, -14.65), Vec3::splat(0.2)),
            (Vec3::new(650.0, 150.0, -2850.0), Vec3::splat(100.0)),
        ];
        let mut scene = SyntheticScene::grid(4).expect("scene");
        scene.view = camera(1.0);
        scene.sun_shadows = true;
        scene.materials[0].base_rgba = [0.8, 0.8, 0.8, 1.0];
        scene.instances = vec![
            instance(0, Vec3::new(0.0, 994.5, -15.0), Vec3::new(0.9, 0.5, 0.9)),
            instance(
                1,
                Vec3::new(500.0, -0.5, -3000.0),
                Vec3::new(500.0, 0.5, 500.0),
            ),
            instance(2, casters[0].0, casters[0].1),
            instance(3, casters[1].0, casters[1].1),
        ];
        let cascade = draw(&device, &cap, &queue, scene.clone(), 2).await;
        let height = draw(&device, &cap, &queue, scene.clone(), 0).await;
        scene.sun_shadows = false;
        let clear = draw(&device, &cap, &queue, scene, 0).await;
        let record: SecondaryViewRecord = bytemuck::pod_read_unaligned(&cascade[..RECORD_BYTES]);
        let inverse = Mat4::from_cols_array_2d(&record.inverse_view_proj);
        let mut samples = [0; 3];
        for i in 0..256 * 256 {
            let (colour, depth) = pixel(&cascade, i);
            if colour[3] == 0.0 {
                continue;
            }
            let h = inverse
                * Vec4::new(
                    (i % 256) as f32 / 128.0 + 0.5 / 128.0 - 1.0,
                    1.0 - ((i / 256) as f32 + 0.5) / 128.0,
                    depth,
                    1.0,
                );
            let p = h.truncate() / h.w;
            let near = (p.y - 995.0).abs() < 0.01;
            let far = p.y.abs() < 0.1;
            if !near && !far {
                continue;
            }
            let caster = casters[usize::from(far)];
            let margin = if near { 0.06 } else { 50.0 };
            let shadow = blocked(p, sun, caster.0, caster.1 - Vec3::splat(margin));
            let lit = !blocked(p, sun, caster.0, caster.1 + Vec3::splat(margin));
            let unshadowed = pixel(&clear, i).0[0];
            let height_only = pixel(&height, i).0[0];
            if shadow && near {
                assert!(
                    colour[0] < unshadowed * 0.7,
                    "CPU blocked near receiver {p:?}: {} / {unshadowed}",
                    colour[0]
                );
                assert!(
                    (height_only - unshadowed).abs() < unshadowed * 0.05,
                    "small caster is below height-field resolution"
                );
                samples[0] += 1;
            } else if shadow && far {
                assert!(
                    colour[0] < unshadowed * 0.7 && height_only < unshadowed * 0.7,
                    "height-map shadow at {p:?}"
                );
                samples[1] += 1;
            } else if lit && far {
                assert!(
                    (height_only - unshadowed).abs() < unshadowed * 0.05,
                    "CPU visible far receiver {p:?}"
                );
                samples[2] += 1;
            }
        }
        eprintln!("CPU classified near blocked / far blocked / far visible pixels: {samples:?}");
        assert!(
            samples.iter().all(|&n| n > 2),
            "all three receiver populations exercised"
        );
    });
}
