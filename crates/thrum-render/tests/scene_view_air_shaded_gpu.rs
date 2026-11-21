//! Shaded panels use the ordinary main view at the pixel's plane crossing as
//! their reference. Stored aerial texels supply rounding, never an integrator.
mod support;
use glam::{Mat4, Vec3, Vec4};
use support::*;
use thrum_render::{
    abi::InstanceRecord,
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    contribution::{SecondaryViewRequest, ViewAir, ViewClipPlane},
    synthetic::{FrameLighting, SyntheticScene, SyntheticView},
};

fn camera(eye: Vec3, pitch: f32, extent: [u32; 2]) -> SyntheticView {
    let direction = Vec3::new(0.0, pitch.sin(), -pitch.cos());
    SyntheticView::perspective(
        eye.to_array(),
        (eye + direction * 8192.0).to_array(),
        Vec3::Y.to_array(),
        60.0_f32.to_radians(),
        extent[0] as f32 / extent[1] as f32,
        0.1,
        10000.0,
    )
    .expect("camera")
}
fn light(height: f32, sun: Vec3) -> FrameLighting {
    FrameLighting::Atmosphere(
        AtmosphereFrame::new(sun.to_array(), 100_000.0, [1.0; 3], height, 1).expect("air"),
    )
}
fn pixel_direction(view: &SyntheticView, uv: [f32; 2]) -> Vec3 {
    let eye = Vec3::from_array(view.camera_position());
    let inverse = Mat4::from_cols_array_2d(&view.view_proj_columns()).inverse();
    let far = inverse * Vec4::new(uv[0] * 2.0 - 1.0, 1.0 - uv[1] * 2.0, 0.0, 1.0);
    (far.truncate() - eye * far.w).normalize()
}
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
fn step(value: f32) -> f32 {
    let bits = half::f16::from_f32(value.abs()).to_bits();
    half::f16::from_bits(bits + 1).to_f32() - half::f16::from_bits(bits).to_f32()
}
fn allowance(
    reference: [f32; 4],
    raw: &[f32],
    stretch: f32,
    uv: [f32; 2],
    radiance: f32,
    reads: u32,
) -> [f32; 3] {
    let mut scatter_q = [0.0_f32; 3];
    let mut trans_q = [0.0_f32; 3];
    for i in 1..=8 {
        let distance = stretch * (i as f32 / 8.0).powi(2);
        let (_, s) = sample(raw, 0, distance, uv, false);
        let (_, t) = sample(raw, 1, distance, uv, false);
        for c in 0..3 {
            scatter_q[c] = scatter_q[c].max(s[c]);
            trans_q[c] = trans_q[c].max(t[c]);
        }
    }
    std::array::from_fn(|c| {
        step(reference[c])
            + reads as f32 * (scatter_q[c] * 100_000.0 / 1024.0 + trans_q[c] * radiance) / 1024.0
    })
}
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "positive clamped distances and UVs address 32x32x33 texels"
)]
fn sample(
    volume: &[f32],
    texture: usize,
    distance: f32,
    uv: [f32; 2],
    empty: bool,
) -> ([f32; 3], [f32; 3]) {
    let low = (((distance / 32000.0).sqrt() * 32.0).floor() as usize).min(31);
    let first = 32000.0 * (low as f32 / 32.0).powi(2);
    let second = 32000.0 * ((low + 1) as f32 / 32.0).powi(2);
    let blend = ((distance - first) / (second - first)).clamp(0.0, 1.0);
    let xy = uv.map(|v| (v * 32.0 - 0.5).clamp(0.0, 31.0));
    let mut value = [0.0; 3];
    let mut quantum = [0.0_f32; 3];
    for (layer, wz) in [(low, 1.0 - blend), (low + 1, blend)] {
        for (y, wy) in [
            (xy[1] as usize, 1.0 - xy[1].fract()),
            ((xy[1] as usize + 1).min(31), xy[1].fract()),
        ] {
            for (x, wx) in [
                (xy[0] as usize, 1.0 - xy[0].fract()),
                ((xy[0] as usize + 1).min(31), xy[0].fract()),
            ] {
                for c in 0..3 {
                    let v =
                        volume[texture * 32 * 32 * 33 * 4 + (layer * 32 * 32 + y * 32 + x) * 4 + c];
                    quantum[c] = quantum[c].max(step(v));
                    if !empty || y < 16 {
                        value[c] += v * wx * wy * wz;
                    }
                }
            }
        }
    }
    (value, quantum)
}
async fn main_pixel(
    device: &wgpu::Device,
    cap: &DeviceCapabilities,
    queue: &wgpu::Queue,
    scene: SyntheticScene,
    sun: Vec3,
    extent: [u32; 2],
    index: usize,
) -> [f32; 4] {
    let lighting = light(scene.view.camera_position()[1], sun);
    let mut host = host_with_extent(cap, queue, scene, lighting, extent).await;
    let (probe, control) = MainObserver::new(extent);
    host.set_scene_contribution(Box::new(probe))
        .expect("ordinary observer");
    host.capture_pixels().expect("ordinary main draw");
    pixel(
        &output(device, queue, &control.lock().expect("control").outputs[0]),
        index,
    )
    .0
}
async fn volume(
    device: &wgpu::Device,
    cap: &DeviceCapabilities,
    queue: &wgpu::Queue,
    scene: SyntheticScene,
    sun: Vec3,
    view: &SyntheticView,
) -> Vec<f32> {
    let lighting = light(scene.view.camera_position()[1], sun);
    let mut host = host_with_extent(cap, queue, scene, lighting, AIR_EXTENT).await;
    let request = SecondaryViewRequest::new(
        view,
        AIR_EXTENT,
        Some(ViewClipPlane::new([0.0, 1.0, 0.0, -50.0]).expect("plane")),
        ViewAir::Plane,
    )
    .expect("volume request");
    let (probe, control) = AirProbe::new(Some(&request));
    host.set_scene_contribution(Box::new(probe))
        .expect("volume observer");
    host.capture_pixels().expect("raw volume");
    air_output(device, queue, &control.lock().expect("control").outputs[1])
}

#[expect(
    clippy::print_stderr,
    reason = "retained proof prints measured changes beside the rounding allowance"
)]
fn check_casters(results: &[([f32; 4], [f32; 4])], allowance: [f32; 3]) {
    for (c, bound) in allowance.into_iter().enumerate() {
        let view_change = results[0].0[c] - results[1].0[c];
        let reference_change = results[0].1[c] - results[1].1[c];
        let before_view = (results[0].0[c] - results[2].0[c]).abs();
        let before_reference = (results[0].1[c] - results[2].1[c]).abs();
        eprintln!(
            "caster darkening view={view_change} reference={reference_change}; before-crossing change view={before_view} reference={before_reference}; allowance={bound}"
        );
        assert!(
            view_change > bound && reference_change > bound,
            "caster beyond crossing darkens both"
        );
        assert!(
            before_view <= bound && before_reference <= bound,
            "caster before crossing leaves both alone"
        );
    }
}
#[expect(
    clippy::print_stderr,
    reason = "retained negative controls print their difference and allowance"
)]
fn check_wrong_stretch(
    actual: [f32; 4],
    wrong: [f32; 4],
    before: Option<[f32; 4]>,
    limit: [f32; 3],
) {
    for (c, bound) in limit.into_iter().enumerate() {
        let difference = (actual[c] - wrong[c]).abs();
        eprintln!(
            "wrong whole-camera stretch channel={c}: difference={difference} allowance={bound}"
        );
        assert!(
            difference > bound,
            "whole-camera air must miss: channel {c}"
        );
        if let Some(shaded) = before {
            let change = (wrong[c] - shaded[c]).abs();
            eprintln!(
                "before-only caster darkens the whole-camera reference by {change}; crossing allowance={bound}"
            );
            assert!(
                change > bound,
                "the before-only caster must shade the stretch omitted by the view"
            );
        }
    }
}
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one proof walks every plane case through the crossing and the wrong-stretch controls"
)]
fn plane_panels_match_the_main_camera_at_the_crossing_and_reject_wrong_stretches() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "plane shaded air",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    pollster::block_on(async {
        let cap = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
        for (below, distance, horizon) in [
            (2.0, 20.0, false),
            (2.0, 3000.0, false),
            (40.0, 20.0, false),
            (40.0, 3000.0, false),
            (2.0, 1000.0, true),
        ] {
            let extent = if horizon { [65, 331] } else { [65, 65] };
            let xy = if horizon { [32, 164] } else { [32, 32] };
            let uv = [
                (xy[0] as f32 + 0.5) / extent[0] as f32,
                (xy[1] as f32 + 0.5) / extent[1] as f32,
            ];
            let eye = Vec3::new(0.0, 50.0 - below, 0.0);
            let pitch = if horizon { 0.0 } else { 10.0_f32.to_radians() };
            let view = camera(eye, pitch, extent);
            let direction = pixel_direction(&view, uv);
            let crossing = eye + direction * (below / direction.y);
            let plane = ViewClipPlane::new([0.0, 1.0, 0.0, -50.0]).expect("plane");
            let request = SecondaryViewRequest::new(&view, extent, Some(plane), ViewAir::Plane)
                .expect("request")
                .with_shadows(0, 128.0)
                .expect("profile");
            for (sun_index, sun) in [
                Vec3::new(1.0, 1.0, 0.0).normalize(),
                Vec3::new(
                    0.0,
                    10.0_f32.to_radians().sin(),
                    10.0_f32.to_radians().cos(),
                ),
            ]
            .into_iter()
            .enumerate()
            {
                for radiance in [0.0, 100.0] {
                    let mut scene = SyntheticScene::grid(1).expect("scene");
                    scene.view = view;
                    scene.instances = vec![instance(
                        0,
                        crossing + direction * distance,
                        Vec3::new(distance * 0.1, distance * 0.1, 0.01),
                    )];
                    scene.materials[0].base_rgba = [0.0, 0.0, 0.0, 1.0];
                    scene.materials[0].surface[2] = 1.0;
                    scene.materials[0].emissive_cutoff = [radiance, radiance, radiance, 0.0];
                    let mut reference_scene = scene.clone();
                    reference_scene.view = camera(crossing, pitch, extent);
                    let expected = main_pixel(
                        &device,
                        &cap,
                        &queue,
                        reference_scene.clone(),
                        sun,
                        extent,
                        (xy[1] * extent[0] + xy[0]) as usize,
                    )
                    .await;
                    let raw = volume(&device, &cap, &queue, scene.clone(), sun, &view).await;
                    // The front surface is 1 cm before the panel centre in Z.
                    let stretch = distance - 0.01 / direction.z.abs();
                    let limit = allowance(expected, &raw, stretch, uv, radiance, 4);
                    let mut results = Vec::new();
                    let mut before_camera = None;
                    // The shadowed cases use a substantial stretch so a caster
                    // and its 64 m shadow-height cells can lie wholly on one side.
                    let casters = if below > 39.0 && distance > 2999.0 {
                        3
                    } else {
                        1
                    };
                    for caster in 0..casters {
                        let mut variant = scene.clone();
                        variant.sun_shadows = caster != 0;
                        if caster != 0 {
                            let midpoint = if caster == 1 {
                                crossing + direction * 1500.0
                            } else {
                                eye + direction * 126.0
                            };
                            let half = if caster == 1 {
                                Vec3::new(128.0, 80.0, 200.0)
                            } else {
                                Vec3::new(64.0, 16.0, 40.0)
                            };
                            variant
                                .instances
                                .push(instance(1, midpoint + sun * 512.0, half));
                        }
                        let mut host = host_with_extent(
                            &cap,
                            &queue,
                            variant.clone(),
                            light(eye.y, sun),
                            extent,
                        )
                        .await;
                        let (observer, control) = Observer::new(1);
                        control.lock().expect("control").requests[0] = Some(request);
                        host.set_scene_contribution(Box::new(observer))
                            .expect("view observer");
                        host.capture_pixels().expect("panel view");
                        let actual = pixel(
                            &output(
                                &device,
                                &queue,
                                &control.lock().expect("control").outputs[0],
                            ),
                            (xy[1] * extent[0] + xy[0]) as usize,
                        )
                        .0;
                        assert_eq!(actual[3], 1.0, "covered panel");
                        if caster == 2 && radiance == 0.0 {
                            before_camera = Some(
                                main_pixel(
                                    &device,
                                    &cap,
                                    &queue,
                                    variant.clone(),
                                    sun,
                                    extent,
                                    (xy[1] * extent[0] + xy[0]) as usize,
                                )
                                .await,
                            );
                        }
                        variant.view = reference_scene.view;
                        let reference = if caster == 0 {
                            expected
                        } else {
                            main_pixel(
                                &device,
                                &cap,
                                &queue,
                                variant,
                                sun,
                                extent,
                                (xy[1] * extent[0] + xy[0]) as usize,
                            )
                            .await
                        };
                        let allowance = allowance(
                            reference,
                            &raw,
                            stretch,
                            uv,
                            radiance,
                            if caster == 0 { 4 } else { 18 },
                        );
                        for c in 0..3 {
                            let difference = (actual[c] - reference[c]).abs();
                            eprintln!(
                                "below={below} stretch={distance} horizon={horizon} sun={sun_index} radiance={radiance} caster={caster} channel={c}: difference={difference} allowance={}",
                                allowance[c]
                            );
                            assert!(difference <= allowance[c], "ordinary crossing reference");
                        }
                        results.push((actual, reference));
                    }
                    if casters == 3 && radiance == 0.0 {
                        check_casters(
                            &results,
                            allowance(expected, &raw, stretch, uv, radiance, 18),
                        );
                    }
                    if radiance == 0.0 && !(below < 3.0 && distance > 2999.0) {
                        let wrong = main_pixel(
                            &device,
                            &cap,
                            &queue,
                            scene,
                            sun,
                            extent,
                            (xy[1] * extent[0] + xy[0]) as usize,
                        )
                        .await;
                        check_wrong_stretch(results[0].0, wrong, before_camera, limit);
                    }
                    if horizon && radiance == 0.0 {
                        let (empty, _) = sample(&raw, 0, stretch, uv, true);
                        for c in 0..3 {
                            let wrong = empty[c] * 100_000.0 / (1024.0 * 1024.0);
                            let difference = (results[0].0[c] - wrong).abs();
                            eprintln!(
                                "emptied never-crossing row channel={c}: difference={difference} allowance={}",
                                limit[c]
                            );
                            assert!(difference > limit[c], "empty never-crossing row must miss");
                        }
                    }
                }
            }
        }
    });
}
