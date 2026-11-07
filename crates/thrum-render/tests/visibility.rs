//! Camera frustum compaction and two-phase occlusion against the direct-draw
//! oracle: the indirect draws of the compacted lists must produce
//! byte-identical pixels.

use thrum_render::capabilities::DeviceCapabilities;
use thrum_render::synthetic::{
    CullMode, FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticLight, SyntheticScene,
    SyntheticView,
};
use thrum_render::timing::GpuTimings;

const EXTENT: [u32; 2] = [320, 180];

async fn device() -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("real adapter required for visibility image equivalence");
    #[cfg(target_os = "macos")]
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .expect("visibility device");
    (adapter, device, queue)
}

async fn render(
    capabilities: &DeviceCapabilities,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &SyntheticScene,
) -> Vec<u8> {
    let prepared = PreparedSynthetic::prepare_with_capabilities(
        capabilities,
        queue,
        scene,
        FrameLighting::Diagnostic(SyntheticLight::default()),
        1.0,
    )
    .await
    .expect("prepare");
    let target = OffscreenTarget::new(device, &prepared, EXTENT[0], EXTENT[1]).expect("target");
    let mut pixels = target.readback(device);
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode_with_readback(
            &mut encoder,
            capabilities.generation(),
            &target,
            None,
            &mut pixels,
        )
        .expect("encode");
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .expect("poll");
    assert!(scope.pop().await.is_none(), "no validation error");
    pixels
        .try_collect(capabilities.generation())
        .expect("collect")
        .expect("mapped pixels")
}

fn scene(view: SyntheticView, culling: CullMode, shadows: bool, hidden: usize) -> SyntheticScene {
    let mut scene = SyntheticScene::grid(10_000).expect("grid");
    scene.view = view;
    scene.camera_culling = culling;
    scene.sun_shadows = shadows;
    for (index, instance) in scene.instances.iter_mut().enumerate() {
        if index % hidden == 0 {
            instance.ids[3] = 0;
        }
        // Mirrored instances take the two-sided pipeline and a negative basis.
        if index % 11 == 0 {
            for row in 0..3 {
                instance.current[0][row] = -instance.current[0][row];
                instance.normal[0][row] = -instance.normal[0][row];
            }
        }
        // Moving instances: the cull and the vertex interpolate the same
        // translation between the previous and current endpoints.
        instance.previous = instance.current;
        if index % 3 == 0 {
            instance.previous[3][0] -= 0.05;
            instance.previous[3][2] += 0.02;
        }
    }
    scene
}

fn differing(a: &[u8], b: &[u8]) -> usize {
    assert_eq!(a.len(), b.len());
    a.as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0)
        .filter(|(a, b)| a != b)
        .count()
}

fn equivalent(view: SyntheticView, shadows: bool) -> usize {
    pollster::block_on(async {
        let (adapter, device, queue) = device().await;
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 41);
        let direct = render(
            &capabilities,
            &device,
            &queue,
            &scene(view, CullMode::Direct, shadows, 37),
        )
        .await;
        let culled = render(
            &capabilities,
            &device,
            &queue,
            &scene(view, CullMode::Frustum, shadows, 37),
        )
        .await;
        let changed = differing(&direct, &culled);
        assert_eq!(changed, 0, "frustum compaction changes {changed} pixels");
        // Coverage: pixels the instances touch, against the same view with
        // every instance hidden (the backdrop is not uniform).
        let empty = render(
            &capabilities,
            &device,
            &queue,
            &scene(view, CullMode::Direct, shadows, 1),
        )
        .await;
        differing(&direct, &empty)
    })
}

/// Instances wholly outside one clip plane at both motion endpoints, and
/// instances whose centre is strictly inside the frustum at both. Proves a
/// view really exercises culling rather than trivially keeping everything.
fn split(view: SyntheticView) -> (usize, usize) {
    let m = view.view_proj_columns();
    let clip = |p: [f32; 3]| -> [f32; 4] {
        std::array::from_fn(|row| {
            m[0][row] * p[0] + m[1][row] * p[1] + m[2][row] * p[2] + m[3][row]
        })
    };
    let planes = |c: [f32; 4]| {
        [
            c[3] + c[0],
            c[3] - c[0],
            c[3] + c[1],
            c[3] - c[1],
            c[3] - c[2],
            c[2],
        ]
    };
    let scene = scene(view, CullMode::Direct, false, 37);
    let (mut outside, mut inside) = (0, 0);
    for instance in &scene.instances {
        let mut points = Vec::new();
        for endpoint in [instance.previous, instance.current] {
            for corner in 0..8 {
                let sign = |bit: usize| if corner >> bit & 1 == 0 { -1.0 } else { 1.0 };
                points.push(std::array::from_fn(|row| {
                    endpoint[3][row]
                        + (0..3)
                            .map(|axis| instance.current[axis][row] * sign(axis))
                            .sum::<f32>()
                }));
            }
        }
        if (0..6).any(|plane| points.iter().all(|&p| planes(clip(p))[plane] < -1e-4)) {
            outside += 1;
        }
        let centres = [instance.previous[3], instance.current[3]];
        if centres
            .iter()
            .all(|c| planes(clip([c[0], c[1], c[2]])).iter().all(|&d| d > 1e-4))
        {
            inside += 1;
        }
    }
    (outside, inside)
}

fn view(camera: [f32; 3], target: [f32; 3]) -> SyntheticView {
    SyntheticView::perspective(camera, target, [0.0, 1.0, 0.0], 1.0, 16.0 / 9.0, 0.01, 10.0)
        .expect("view")
}

#[test]
fn partial_visibility_matches_direct_draw() {
    let (outside, inside) = split(view([0.4, 0.1, 0.9], [0.7, 0.0, 0.5]));
    assert!(
        outside > 1_000 && inside > 100,
        "culls {outside}, keeps {inside}"
    );
    let covered = equivalent(view([0.4, 0.1, 0.9], [0.7, 0.0, 0.5]), false);
    assert!(covered > 1_000, "scene draws: {covered} pixels");
}

#[test]
fn full_and_empty_visibility_match_direct_draw() {
    assert!(equivalent(view([0.0, 0.0, 3.0], [0.0, 0.0, 0.5]), false) > 1_000);
    assert_eq!(equivalent(view([0.0, 0.0, 3.0], [0.0, 0.0, 6.0]), false), 0);
}

#[test]
fn near_plane_crossings_match_direct_draw() {
    // The camera sits inside the cube layer, so the near plane cuts through
    // instances beside it while the grid recedes at a grazing angle.
    let (outside, inside) = split(view([-0.2, -0.1, 0.503], [0.6, 0.3, 0.497]));
    assert!(
        outside > 1_000 && inside > 100,
        "culls {outside}, keeps {inside}"
    );
    let covered = equivalent(view([-0.2, -0.1, 0.503], [0.6, 0.3, 0.497]), false);
    assert!(covered > 1_000, "scene draws: {covered} pixels");
}

#[test]
fn offscreen_sun_casters_still_shadow_visible_receivers() {
    let covered = equivalent(view([0.4, 0.1, 0.9], [0.7, 0.0, 0.5]), true);
    assert!(covered > 1_000, "scene draws: {covered} pixels");
}

#[test]
fn frustum_pass_is_timed_within_the_scene_pass_budget() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: DeviceCapabilities::requested_timing_features(&adapter),
                ..Default::default()
            })
            .await
            .expect("timestamp device");
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 41);
        let view = view([0.4, 0.1, 0.9], [0.7, 0.0, 0.5]);
        let mut counts = Vec::new();
        for culling in [CullMode::Direct, CullMode::Frustum] {
            let prepared = PreparedSynthetic::prepare_with_capabilities(
                &capabilities,
                &queue,
                &scene(view, culling, true, 37),
                FrameLighting::Diagnostic(SyntheticLight::default()),
                1.0,
            )
            .await
            .expect("prepare");
            let target = OffscreenTarget::new(&device, &prepared, 64, 36).expect("target");
            let mut timings = GpuTimings::new(&capabilities, &queue, prepared.scene_pass_count())
                .expect("timestamp batch");
            #[cfg(target_os = "macos")]
            assert!(timings.is_enabled(), "Metal timestamps required");
            if !timings.is_enabled() {
                return;
            }
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            prepared
                .encode(&mut encoder, 41, &target, Some(&mut timings))
                .expect("every pass fits the advertised budget");
            timings.finish_render(41, &encoder).expect("close batch");
            let submitted = queue.submit([encoder.finish()]);
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submitted),
                    timeout: Some(std::time::Duration::from_secs(10)),
                })
                .expect("poll");
            let mut resolve =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            assert!(timings.resolve_if_ready(41, &mut resolve).expect("resolve"));
            let submitted = queue.submit([resolve.finish()]);
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submitted),
                    timeout: Some(std::time::Duration::from_secs(10)),
                })
                .expect("poll");
            let passes = timings
                .try_collect(41)
                .expect("valid timestamps")
                .expect("complete timestamps");
            let named = passes
                .iter()
                .any(|pass| pass.name.as_ref() == "camera visibility");
            assert_eq!(named, culling == CullMode::Frustum, "{culling:?}");
            counts.push((prepared.scene_pass_count(), passes.len()));
        }
        assert_eq!(counts[1].0, counts[0].0 + 1, "{counts:?}");
        assert_eq!(counts[1].1, counts[0].1 + 1, "{counts:?}");
    });
}

/// A live sequence: one prepared scene and target per mode, the same frame
/// updates, readback of every frame.
struct Sequence {
    prepared: PreparedSynthetic,
    target: OffscreenTarget,
}

impl Sequence {
    async fn new(
        capabilities: &DeviceCapabilities,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &SyntheticScene,
    ) -> Self {
        let prepared = PreparedSynthetic::prepare_with_capabilities(
            capabilities,
            queue,
            scene,
            FrameLighting::Diagnostic(SyntheticLight::default()),
            1.0,
        )
        .await
        .expect("prepare");
        let target = OffscreenTarget::new(device, &prepared, EXTENT[0], EXTENT[1]).expect("target");
        Self { prepared, target }
    }

    /// Update, encode and submit one frame; `None` drops the encoded frame
    /// unsubmitted, as a cancelled frame.
    async fn frame(
        &mut self,
        capabilities: &DeviceCapabilities,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        scene: &SyntheticScene,
        submit: bool,
    ) -> Option<Vec<u8>> {
        self.prepared
            .update_frame(
                queue,
                &mut self.target,
                scene,
                FrameLighting::Diagnostic(SyntheticLight::default()),
                0.5,
            )
            .expect("update");
        let mut pixels = self.target.readback(device);
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.prepared
            .encode_with_readback(
                &mut encoder,
                capabilities.generation(),
                &self.target,
                None,
                &mut pixels,
            )
            .expect("encode");
        assert!(scope.pop().await.is_none(), "no validation error");
        if !submit {
            return None;
        }
        let submission = queue.submit([encoder.finish()]);
        self.prepared.commit_frame();
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .expect("poll");
        Some(
            pixels
                .try_collect(capabilities.generation())
                .expect("collect")
                .expect("mapped pixels"),
        )
    }
}

/// Slot 1 becomes a wall across the grazing view, in front of most of the
/// grid, so the late phase really hides slots (counted in the crate's
/// camera-culling tests).
fn walled(mut scene: SyntheticScene) -> SyntheticScene {
    let wall = &mut scene.instances[1];
    let half = [0.005, 0.3, 0.2];
    for (axis, extent) in half.into_iter().enumerate() {
        for row in 0..3 {
            let unit = if axis == row { 1.0 } else { 0.0 };
            wall.current[axis][row] = unit * extent;
            wall.normal[axis][row] = unit / extent;
        }
    }
    wall.current[3] = [0.0, 0.05, 0.55, 1.0];
    wall.previous = wall.current;
    wall.ids[3] = 1;
    scene
}

#[test]
fn occlusion_matches_direct_draw_through_cuts_recycled_rows_and_a_dropped_frame() {
    pollster::block_on(async {
        let (adapter, device, queue) = device().await;
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 41);
        // Grazing views along the grid occlude most of it; one sits inside
        // the cube layer (near-plane crossings), one looks away (empty).
        let grazing = view([-0.2, 0.05, 0.6], [1.0, 0.0, 0.45]);
        let views = [
            grazing,
            grazing,
            view([1.2, 0.08, 0.6], [-0.2, 0.0, 0.45]),
            view([-0.2, -0.1, 0.503], [0.6, 0.3, 0.497]),
            view([0.0, 0.0, 3.0], [0.0, 0.0, 6.0]),
            grazing,
            grazing,
        ];
        let base = walled(scene(grazing, CullMode::Direct, false, 37));
        let mut direct = Sequence::new(&capabilities, &device, &queue, &base).await;
        let mut occluded = Sequence::new(
            &capabilities,
            &device,
            &queue,
            &walled(scene(grazing, CullMode::Occlusion, false, 37)),
        )
        .await;
        assert_eq!(occluded.prepared.camera_culling(), CullMode::Occlusion);
        let mut covered = Vec::new();
        for (index, camera) in views.into_iter().enumerate() {
            let mut frame = base.clone();
            frame.view = camera;
            if index == 5 {
                // Recycled rows: other objects in previously visible slots,
                // and newly hidden and shown rows.
                for slot in (0..frame.instances.len()).step_by(5) {
                    let other = frame.instances[(slot + 2_500) % frame.instances.len()].clone();
                    frame.instances[slot] = other;
                }
                let slots: Vec<u32> = (0..frame.instances.len())
                    .step_by(5)
                    .map(|slot| u32::try_from(slot).expect("slot"))
                    .collect();
                for sequence in [&mut direct, &mut occluded] {
                    sequence
                        .prepared
                        .upload_instances(&queue, &frame.instances, &slots)
                        .expect("upload");
                }
            }
            if index == 3 {
                // A cancelled frame: encoded with this view, never submitted.
                occluded
                    .frame(&capabilities, (&device, &queue), &frame, false)
                    .await;
            }
            let reference = direct
                .frame(&capabilities, (&device, &queue), &frame, true)
                .await
                .expect("direct pixels");
            let culled = occluded
                .frame(&capabilities, (&device, &queue), &frame, true)
                .await
                .expect("occlusion pixels");
            let changed = differing(&reference, &culled);
            assert_eq!(
                changed, 0,
                "frame {index}: occlusion changes {changed} pixels"
            );
            covered.push(reference);
        }
        let backdrop = render(
            &capabilities,
            &device,
            &queue,
            &scene(grazing, CullMode::Direct, false, 1),
        )
        .await;
        assert!(
            differing(&covered[0], &backdrop) > 1_000,
            "the grazing view draws"
        );
        assert!(
            covered
                .windows(2)
                .any(|pair| differing(&pair[0], &pair[1]) > 1_000),
            "the cuts change the image"
        );
    });
}

/// Root's equal-depth witness: two slots at exactly the same pose with
/// different materials. Direct draws in slot order with a strict reverse-Z
/// test, so the lower slot wins every tie. `entering` is the slot that starts
/// outside the view and moves onto the other's pose, so only the staying slot
/// is in Occlusion's prepass.
async fn equal_depth_case(entering: usize) -> (Vec<u8>, Vec<u8>) {
    let (adapter, device, queue) = device().await;
    let capabilities = DeviceCapabilities::capture(&adapter, &device, 43);
    let camera = view([0.0, 0.0, -1.5], [0.0, 0.0, 0.5]);
    let mut base = SyntheticScene::grid(2).expect("grid");
    base.view = camera;
    let staying = 1 - entering;
    base.instances[staying].current[3] = [0.0, 0.0, 0.5, 1.0];
    base.instances[staying].previous = base.instances[staying].current;
    base.instances[entering].current[3] = [40.0, 0.0, 0.5, 1.0];
    base.instances[entering].previous = base.instances[entering].current;
    let mut direct = Sequence::new(&capabilities, &device, &queue, &base).await;
    let mut culled_scene = base.clone();
    culled_scene.camera_culling = CullMode::Occlusion;
    let mut occluded = Sequence::new(&capabilities, &device, &queue, &culled_scene).await;
    // Two frames with only the staying slot in view: it is visible history.
    for _ in 0..2 {
        for sequence in [&mut direct, &mut occluded] {
            sequence
                .frame(&capabilities, (&device, &queue), &base, true)
                .await;
        }
    }
    // The entering slot takes the staying slot's exact pose.
    let mut frame = base.clone();
    let mut moved = frame.instances[staying].clone();
    moved.ids = frame.instances[entering].ids;
    frame.instances[entering] = moved;
    let slot = u32::try_from(entering).expect("slot");
    for sequence in [&mut direct, &mut occluded] {
        sequence
            .prepared
            .upload_instances(&queue, &frame.instances, &[slot])
            .expect("upload");
    }
    let reference = direct
        .frame(&capabilities, (&device, &queue), &frame, true)
        .await
        .expect("direct pixels");
    let culled = occluded
        .frame(&capabilities, (&device, &queue), &frame, true)
        .await
        .expect("occlusion pixels");
    (reference, culled)
}

/// Root's equal-depth witness. M2's split forward pass drew the staying slot
/// early and the entering one late, so a lower slot entering lost the tie
/// (`[3216, 0]`). Occlusion now draws one list in slot order after the
/// prepass, so both orders keep Direct's winner.
#[test]
fn equal_depth_ties_keep_direct_order_whichever_slot_enters() {
    pollster::block_on(async {
        let mut changed = Vec::new();
        for entering in [0, 1] {
            let (reference, culled) = Box::pin(equal_depth_case(entering)).await;
            changed.push(differing(&reference, &culled));
        }
        eprintln!("pixels changed against Direct with slot 0, then slot 1, entering: {changed:?}");
        assert_eq!(
            changed,
            [0, 0],
            "both orders keep Direct's equal-depth winner"
        );
    });
}
