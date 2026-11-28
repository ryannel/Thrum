//! Observe production Sun visibility at world points against rendered caster depth.

use std::fmt::Write as _;

use super::{observer, *};
use crate::synthetic::{
    OffscreenTarget, PreparedSynthetic, SyntheticLight, SyntheticMeshChoice, SyntheticMeshes,
    SyntheticScene,
};

fn box_at(slot: u32, centre: Vec3, size: Vec3) -> InstanceRecord {
    let mut instance = InstanceRecord::identity(slot, 0);
    instance.current =
        Mat4::from_scale_rotation_translation(size * 0.5, glam::Quat::IDENTITY, centre)
            .to_cols_array_2d();
    instance.previous = instance.current;
    instance
}

fn scene(count: u32) -> SyntheticScene {
    let mut scene = SyntheticScene::grid(count).unwrap();
    scene.view = SyntheticView::perspective(
        [0.0; 3],
        [0.0, 0.0, -1.0],
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        4000.0,
    )
    .unwrap();
    scene.sun_shadows = true;
    scene
}

fn mark_box_rows_aloft(scene: &mut SyntheticScene, count: usize) {
    let mut geometry = layout::Geometry::default();
    geometry.box_at(Vec3::ZERO, Vec3::splat(2.0));
    let mut slots = vec![SyntheticMeshChoice::Cube; scene.instances.len()];
    slots[..count].fill(SyntheticMeshChoice::Mesh(0));
    scene.meshes = Some(std::sync::Arc::new(SyntheticMeshes {
        meshes: vec![geometry.mesh()],
        updates: None,
        slots,
    }));
    scene.aloft_mesh_rows = vec![0];
}

fn observe(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &SyntheticScene,
    sun: Vec3,
    receivers: &[Vec3],
) -> Vec<f32> {
    let light = SyntheticLight {
        direction_to_light: sun.to_array(),
        irradiance: [1.0; 3],
    };
    let prepared = pollster::block_on(PreparedSynthetic::prepare_with_light(
        device, queue, 29, scene, light,
    ))
    .unwrap();
    let target = OffscreenTarget::new(device, &prepared, 64, 64).unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared.encode(&mut encoder, 29, &target, None).unwrap();
    queue.submit([encoder.finish()]);

    let mut source = format!(
        "@group(2) @binding(6) var<storage,read_write> result: array<f32>;
         const points = array<vec3<f32>, {}>(",
        receivers.len(),
    );
    for point in receivers {
        write!(source, "vec3({:?},{:?},{:?}),", point.x, point.y, point.z).unwrap();
    }
    source.push_str(
        "); @compute @workgroup_size(64)
        fn observe(@builtin(global_invocation_id) id:vec3<u32>) {
            if id.x >= arrayLength(&result) { return; }
            let receiver = ShadowReceiver(points[id.x], vec3(1.0, 0.0, 0.0),
                vec3(0.0, 0.0, 1.0), vec3(0.0, 1.0, 0.0));
            result[id.x] = mapped_sun_visibility(vec3(0.0), receiver);
        }",
    );
    let samples = &prepared.shadow.as_ref().unwrap().fitted.samples;
    observer::read_visibility(
        device,
        queue,
        &observer::pipeline(device, &source),
        bytemuck::cast_ref(samples),
        &target.shadow.as_ref().unwrap().array_view,
        [sun.x, sun.y, sun.z, 0.0],
        u32::try_from(receivers.len()).unwrap(),
    )
}

#[test]
fn sun_cascades_gpu_handover_keeps_wide_casters_and_fades_fine_only_casters() {
    let Some((device, queue)) = observer::device("nested Sun handover", None) else {
        return;
    };
    let count = SUN_CASCADE_COUNT - 1;
    let mut scene = scene(2 * count);
    let fit = FitInputs::new(&scene.view, (&[], None), Vec3::Y)
        .unwrap()
        .fit(None)
        .unwrap();
    let mut receivers = Vec::new();
    for layer in 0..count {
        let [radius, texel, _, _] = fit.samples.cascade_metrics[layer as usize];
        for (is_wide, sign) in [false, true].into_iter().zip([-1.0, 1.0]) {
            let x = sign * (radius * 0.2 / texel).round() * texel;
            let width = if is_wide { 8.0 } else { 1.95 } * texel;
            let index = 2 * layer + u32::from(is_wide);
            scene.instances[index as usize] = box_at(
                index,
                Vec3::new(x, width * 0.5, -0.87 * radius),
                Vec3::new(width, width, 0.61 * radius),
            );
            for fraction in [0.69, 0.81, 0.88, 0.95, 1.05] {
                let d = radius * fraction;
                let z = (d * d - x * x).sqrt();
                receivers.push(Vec3::new(x, 0.0, -z));
            }
        }
    }
    let observed = observe(&device, &queue, &scene, Vec3::Y, &receivers);
    for (index, row) in observed.as_chunks::<10>().0.iter().enumerate() {
        let (fine, wide) = row.split_at(5);
        assert!(
            wide.iter().all(|&value| value < 0.15),
            "cascade {index}: {wide:?}"
        );
        assert!(
            fine[0] < 0.75 && fine[4] > 0.999,
            "cascade {index}: {fine:?}"
        );
        assert!(
            fine.windows(2).all(|adjacent| adjacent[0] < adjacent[1]),
            "cascade {index}: {fine:?}"
        );
    }
}

#[test]
fn sun_cascades_gpu_far_penumbra_and_near_contact_use_different_maps() {
    let Some((device, queue)) = observer::device("Sun blocker scales", None) else {
        return;
    };
    let mut scene = scene(2);
    mark_box_rows_aloft(&mut scene, 1);
    let (y, z) = 10.0_f32.to_radians().sin_cos();
    let sun = Vec3::new(0.0, y, z);
    scene.instances[0] = box_at(
        0,
        Vec3::new(19.1, 260.0, 260.0 * z / y),
        Vec3::new(16.0, 8.0, 60.0),
    );
    scene.instances[1] = box_at(1, Vec3::new(-1.0, 0.6, 0.0), Vec3::new(0.30, 1.2, 0.30));
    let mut points: Vec<_> = (0..16)
        .map(|i| Vec3::new(3.6 + i as f32, 0.0, 0.0))
        .collect();
    points.extend([Vec3::new(-1.0, 0.0, 0.0), Vec3::new(-0.40, 0.0, 0.0)]);
    assert!(points[4].length() < 8.0);
    let visibility = observe(&device, &queue, &scene, sun, &points);
    let edge = &visibility[..16];
    assert!(
        (0.5..0.95).contains(&edge[4]),
        "outside geometric edge: {edge:?}"
    );
    assert!(
        edge.windows(2).all(|p| (p[0] - p[1]).abs() <= 0.2),
        "edge profile: {edge:?}"
    );
    assert!(visibility[16] < 0.3, "near post shadow: {visibility:?}");
    scene.instances[1].deactivate();
    let control = observe(&device, &queue, &scene, sun, &points[17..]);
    assert!((visibility[17] - control[0]).abs() <= 0.05);
}

#[test]
fn sun_cascades_gpu_low_sun_reach_is_radial_and_fades_to_height_map() {
    let Some((device, queue)) = observer::device("radial Sun reach", None) else {
        return;
    };
    let mut scene = scene(5);
    mark_box_rows_aloft(&mut scene, 5);
    let (y, z) = 5.0_f32.to_radians().sin_cos();
    let sun = Vec3::new(0.0, y, z);
    let along = [60.0, 85.0, 103.0, 120.0, 500.0];
    for (slot, distance) in along.into_iter().enumerate() {
        let centre = Vec3::Z * distance + 20.0 * sun;
        scene.instances[slot] = box_at(u32::try_from(slot).unwrap(), centre, Vec3::splat(2.0));
    }
    let points = along.map(|distance| Vec3::Z * distance);
    scene.view = scene.view.with_sun_shadow_profile(1, 128.0).unwrap();
    let received = observe(&device, &queue, &scene, sun, &points);
    scene.view = scene.view.with_sun_shadow_profile(0, 128.0).unwrap();
    let height = observe(&device, &queue, &scene, sun, &points);
    assert!(received[0] < 0.1, "{received:?}");
    assert!(
        received.windows(2).all(|row| row[0] < row[1]),
        "{received:?}"
    );
    assert_eq!(received[4], height[4]);
    assert!(height[4] > 0.999);
}

#[test]
fn sun_cascades_gpu_threshold_casts_at_quarter_texel_grid_placements() {
    let Some((device, queue)) = observer::device("steady Sun caster threshold", None) else {
        return;
    };
    let mut scene = scene(16);
    let fit = FitInputs::new(&scene.view, (&[], None), Vec3::Y)
        .unwrap()
        .fit(None)
        .unwrap();
    let texel = fit.samples.cascade_metrics[0][1];
    let size = fit.caster_view(0, MAPS).minimum_cast_width[0];
    let mut points = Vec::new();
    for (i, instance) in scene.instances.iter_mut().enumerate() {
        let x = (24.0 * (i as f32 - 8.0) + i as f32 * 0.25) * texel;
        points.push(Vec3::new(x, 0.0, -1.0));
        *instance = box_at(
            u32::try_from(i).unwrap(),
            Vec3::new(x, size * 0.5, -1.0),
            Vec3::splat(size),
        );
    }
    let held = observe(&device, &queue, &scene, Vec3::Y, &points);
    assert!(held.iter().all(|&value| value < 0.99), "{held:?}");
}

#[test]
fn sun_cascades_gpu_thin_post_with_wide_arms_is_excluded() {
    let Some((device, queue)) = observer::device("thin Sun caster", None) else {
        return;
    };
    let mut scene = scene(1);
    let mut post = layout::Geometry::default();
    post.box_at(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.30, 2.0, 0.30));
    for side in [-1.0, 1.0] {
        post.box_at(Vec3::new(0.8 * side, 1.6, 0.0), Vec3::new(1.4, 0.1, 0.1));
    }
    scene.meshes = Some(std::sync::Arc::new(SyntheticMeshes {
        meshes: vec![post.mesh()],
        updates: None,
        slots: vec![SyntheticMeshChoice::Mesh(0)],
    }));
    scene.instances[0].current[3] = [0.0, 0.0, -150.0, 1.0];
    scene.instances[0].previous = scene.instances[0].current;
    let point = Vec3::new(0.0, 0.0, -150.0);
    let lit = observe(&device, &queue, &scene, Vec3::Y, &[point]);
    assert!(lit[0] > 0.999, "{lit:?}");
}

#[test]
fn sun_cascades_gpu_live_view_and_sun_match_a_cold_frame() {
    use crate::synthetic::{CaptureSource, FrameLighting};
    let Some((device, queue)) = observer::device("Sun cascades have no history", None) else {
        return;
    };
    let mut scene = scene(2);
    scene.instances[0] = box_at(0, Vec3::new(0.0, -0.5, -12.0), Vec3::new(40.0, 1.0, 40.0));
    scene.instances[1] = box_at(1, Vec3::new(1.0, 2.0, -12.0), Vec3::new(2.0, 4.0, 2.0));
    let light_before = SyntheticLight {
        direction_to_light: Vec3::new(-1.0, 2.0, 2.0).normalize().to_array(),
        irradiance: [1.0; 3],
    };
    let light_after = SyntheticLight {
        direction_to_light: Vec3::new(2.0, 3.0, 1.0).normalize().to_array(),
        irradiance: [1.0; 3],
    };
    let prepare = |scene: &SyntheticScene, light| {
        pollster::block_on(PreparedSynthetic::prepare_with_light(
            &device, &queue, 30, scene, light,
        ))
        .unwrap()
    };
    let target = |prepared: &PreparedSynthetic| {
        OffscreenTarget::with_capture_source(&device, prepared, 64, 64, CaptureSource::SceneLinear)
            .unwrap()
    };
    let draw = |prepared: &PreparedSynthetic, target: &OffscreenTarget| {
        let mut result = target.readback(&device);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared
            .encode_with_readback(&mut encoder, 30, target, None, &mut result)
            .unwrap();
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(8)),
            })
            .unwrap();
        let mut bytes = Vec::new();
        assert!(result.try_collect_into(30, &mut bytes).unwrap());
        bytes
    };
    let mut live = prepare(&scene, light_before);
    let mut live_target = target(&live);
    let prior = draw(&live, &live_target);
    scene.view = SyntheticView::perspective(
        [3.0, 7.0, 0.0],
        [0.0, 0.0, -12.0],
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        4000.0,
    )
    .unwrap();
    live.update_frame(
        &queue,
        &mut live_target,
        &scene,
        FrameLighting::Diagnostic(light_after),
        1.0,
    )
    .unwrap();
    let current = draw(&live, &live_target);
    let cold = prepare(&scene, light_after);
    assert_ne!(prior, current);
    assert_eq!(current, draw(&cold, &target(&cold)));
}
