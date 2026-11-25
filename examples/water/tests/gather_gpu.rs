//! Water admission on the Metal frame graph, with and without an opaque body.
mod support;

#[cfg(target_os = "macos")]
use support::optics::{Panel, Rig, attach, camera, sun, water};

thrum_test_support::macos_gpu_test! {
fn gather_admits_visible_bodies_and_preserves_the_waterless_graph() {
    let Some(gpu) = thrum_test_support::gpu("gather", Some(wgpu::Backend::Metal)) else { return };
    let view = camera([0.0, 2.0, 0.0], [0.0, 0.0, 8.0], 0.7, 1.0/1024.0);
    let panel = Panel { centre: [0.0, 1.0, 8.0], half_size: [1.2, 0.8, 0.08], radiant: true };
    let outside = Panel { centre: [20.0, 2.0, 0.0], half_size: [1.0; 3], radiant: true };
    let mut rig = Rig::new(&gpu, &view, sun(2.0, 0.000_01), &[panel, outside], None);
    let graph = |rig: &Rig| rig.host.scene_graph().expect("graph").pass_names()
        .map(str::to_owned).collect::<Vec<_>>();
    let baseline = graph(&rig);
    let storage = rig.host.storage_report();
    let opaque = attach(&mut rig, water(None, true));
    assert!(graph(&rig).iter().any(|p| p.contains("water gather")));
    assert!(graph(&rig).iter().all(|p| !p.contains("effect refraction copy")));
    rig.host.set_timing_requested(true);
    let timing = rig.frame().gpu_pass_ns.expect("Metal pass timings");
    assert!(timing.passes().iter().all(|pass| !pass.name.contains("water gather")));
    let (_, empty) = rig.scene_colour();

    assert!(opaque.publish(&[outside.bounds()]));
    let timing = rig.frame().gpu_pass_ns.expect("Metal timings with an off-screen body");
    assert!(timing.passes().iter().all(|pass| !pass.name.contains("water gather")));
    assert_eq!(rig.scene_colour().1, empty);
    assert!(opaque.publish(&[outside.bounds(), panel.bounds()]));
    let timing = rig.frame().gpu_pass_ns.expect("Metal pass timings with a visible receiver");
    assert!(timing.passes().iter().any(|pass| pass.name.contains("water gather")));
    let (_, reflected) = rig.scene_colour();
    let changed = empty.iter().zip(&reflected)
        .filter(|(unregistered, registered)| registered[0] > unregistered[0] + 0.02)
        .count();
    assert!(changed > 14, "a registered panel must appear below its source rectangle: {changed} pixels");
    assert!(opaque.publish(&[]));
    assert_eq!(rig.scene_colour().1, empty, "no source may read the previous frame's capture");
    rig.host.set_scene_contribution(Box::<thrum_render::contribution::SceneContributions>::default()).expect("detach");
    assert_eq!(graph(&rig), baseline);
    assert_eq!(rig.host.storage_report(), storage);
}
}

thrum_test_support::macos_gpu_test! {
fn scenery_in_front_of_a_registered_reflection_stays_in_front() {
    let Some(gpu) = thrum_test_support::gpu("reflection layering", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let view = camera([0.0, 2.0, 0.0], [0.0, 0.0, 8.0], 0.7, 1.0 / 1024.0);
    // The centre ray reflects from z=8. Its slope is 1:4, so it meets
    // the low unregistered panel before the registered panel. The direct
    // camera ray to the taller panel passes above the low one.
    let near = Panel { centre: [0.0, 0.5, 10.0], half_size: [1.5, 0.16, 0.06], radiant: false };
    let far = Panel { centre: [0.0, 1.0, 12.0], half_size: [1.2, 0.35, 0.06], radiant: true };
    let reflected_radiance = |panels: &[Panel]| {
        let mut rig = Rig::new(&gpu, &view, sun(2.0, 0.000_01), panels, None);
        let bodies = attach(&mut rig, water(Some((-0.5, [-16.0, -2.0], 32.0)), true));
        assert!(bodies.publish(&[far.bounds()]));
        let (scale, image) = rig.scene_colour();
        let [width, height] = support::optics::EXTENT.map(|size| usize::try_from(size).expect("extent"));
        image[(height / 2) * width + width / 2][0] / scale
    };
    let mut dark = far;
    dark.radiant = false;
    let unobstructed = reflected_radiance(&[far]) - reflected_radiance(&[dark]);
    let obstructed = reflected_radiance(&[near, far]) - reflected_radiance(&[near, dark]);
    assert!(unobstructed > 100.0, "registered panel signal: {unobstructed}");
    assert!(
        obstructed.abs() < 0.15 * unobstructed,
        "nearer scenery must cover the registered reflection: {obstructed} of {unobstructed}"
    );
}
}
