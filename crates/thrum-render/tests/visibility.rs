//! Camera frustum compaction against the direct-draw oracle: the indirect
//! draw of the compacted list must produce byte-identical pixels.

use thrum_render::capabilities::DeviceCapabilities;
use thrum_render::synthetic::{
    CullMode, FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticLight, SyntheticScene,
    SyntheticView,
};

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
