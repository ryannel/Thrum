use super::support;

use std::{
    io::Write as _,
    time::{Duration, Instant},
};
use thrum_asset_gpu::CookedDrawPath::{Auto, Direct, Indirect, MeshStages};
use thrum_asset_gpu::{MaterialHook, MaterialOutcome};
use thrum_render::capabilities::Recipe;

#[test]
fn selection() {
    let fixture = support::Fixture::new("selection", false);
    for (enabled, recipe, expected_path, reason) in [
        (true, "apple-tile", MeshStages, None),
        (
            false,
            "apple-tile",
            Indirect,
            Some("device did not enable mesh-stages"),
        ),
        (true, "portable", Indirect, Some("recipe is portable")),
        (
            false,
            "portable",
            Indirect,
            Some("device did not enable mesh-stages"),
        ),
    ] {
        let gpu = support::Gpu::new(enabled, recipe, |_| {});
        let expected_recipe = if recipe == "apple-tile" {
            Recipe::AppleTile
        } else {
            Recipe::Portable
        };
        let mut reference = gpu.scene(&fixture, Auto, true, false);
        let expected = reference.frame();
        {
            let stats = reference.stats.lock().expect("cooked report");
            assert_eq!(stats.requested_draw_path, Some(Auto));
            assert_eq!(stats.draw_path, Some(Indirect));
            assert_eq!(stats.draw_fallback, None);
            assert_eq!(stats.mesh_stage_draws, 0);
        }
        let mut scene = gpu.scene(&fixture, MeshStages, true, false);
        let image = scene.frame();
        scene.report("selection");
        let stats = scene.stats.lock().expect("cooked report");
        assert_eq!(stats.recipe, Some(expected_recipe));
        assert_eq!(stats.requested_draw_path, Some(MeshStages));
        assert_eq!(stats.draw_path, Some(expected_path), "{stats:?}");
        assert_eq!(stats.draw_fallback.as_deref(), reason);
        if expected_path == MeshStages {
            assert_eq!(stats.mesh_stage_draws, fixture.placements.len());
            assert_eq!(stats.indexed_indirect_draws, 0);
            assert!(!stats.meshlet_compute_cull);
        } else {
            assert_eq!(stats.mesh_stage_draws, 0);
            assert!(stats.indexed_indirect_draws > 0);
            assert!(stats.meshlet_compute_cull);
            expected.equal(&image);
        }
    }
}

#[test]
fn prepared() {
    let gpu = support::Gpu::new(true, "apple-tile", |_| {});
    let fixture = support::Fixture::new("prepared", false);
    let mut scene = gpu.scene(&fixture, MeshStages, true, false);
    let ready = scene.host.creation_report();
    assert!(ready.ready);
    let row = ready
        .pipelines
        .iter()
        .find(|r| r.label == "contribution")
        .expect("prepared contribution pipeline ledger");
    assert!(
        row.render >= 2 && row.compute >= 1 && row.mesh == 1,
        "{ready}"
    );
    scene.frame();
    scene.frame();
    let playing = scene.host.creation_report();
    assert_eq!(ready.pipelines, playing.pipelines);
    assert_eq!(playing.violations, 0);
    writeln!(std::io::stdout(), "scene creation ledger {playing}").expect("ledger");
    let mut broken = gpu.scene(&fixture, MeshStages, true, true);
    let pixels = broken.frame();
    broken.report("pipeline refused");
    {
        let report = broken.stats.lock().expect("report");
        assert_eq!(report.draw_path, Some(Indirect));
        assert_eq!(report.mesh_stage_bytes, 0);
        assert!(
            report
                .draw_fallback
                .as_deref()
                .is_some_and(|reason| reason.contains("missing_mesh_entry"))
        );
    }
    let mut indexed = gpu.scene(&fixture, Auto, true, false);
    pixels.equal(&indexed.frame());
    let before_reload = scene.host.creation_report();
    let serial = scene
        .reload
        .request(edit("prepared-green", [0.0, 12.0, 0.0]));
    await_candidate_off_frame(&scene, &before_reload);
    discard_candidate(&mut scene, serial);
    scene.frame();
    assert_promoted(&scene, serial);
}

#[test]
fn eligibility() {
    let fixture = support::Fixture::new("eligibility", true);
    for reason in [
        "max_storage_buffer_binding_size",
        "max_task_workgroup_total_count",
    ] {
        let gpu = support::Gpu::new(true, "apple-tile", |limits| {
            if reason == "max_storage_buffer_binding_size" {
                limits.max_storage_buffer_binding_size = 160 << 10;
            } else {
                limits.max_task_workgroup_total_count = 1;
            }
        });
        let mut scene = gpu.scene(&fixture, MeshStages, false, false);
        let image = scene.frame();
        scene.report(reason);
        let mut reference = gpu.scene(&fixture, Auto, false, false);
        let reference = reference.frame();
        let stats = scene.stats.lock().expect("stats");
        assert_eq!(stats.draw_path, Some(MeshStages), "{stats:?}");
        assert_eq!(stats.mesh_stage_draws, 1);
        assert!(stats.indexed_indirect_draws > 1 && stats.meshlet_compute_cull);
        assert_eq!(
            stats.mesh_stage_exclusions,
            [("meshes/large".into(), reason)]
        );
        assert_eq!(stats.mesh_stage_vertex_streams, 2);
        image.save("eligibility", &format!("{reason}-mixed"));
        reference.save("eligibility", &format!("{reason}-indexed"));
    }
}

#[test]
fn residency() {
    let fixture = support::Fixture::new("residency", true);
    let ordinary = support::Gpu::new(false, "apple-tile", |_| {});
    let enabled = support::Gpu::new(true, "apple-tile", |_| {});
    for path in [Auto, Direct, Indirect] {
        let reference = ordinary.scene(&fixture, path, true, false);
        let candidate = enabled.scene(&fixture, path, true, false);
        let a = reference.stats.lock().expect("ordinary");
        let b = candidate.stats.lock().expect("enabled");
        assert_eq!(a.mesh_buffer_bytes, b.mesh_buffer_bytes);
        assert_eq!(a.swaps[0].uploaded_bytes, b.swaps[0].uploaded_bytes);
        assert_eq!(a.mesh_stage_bytes + b.mesh_stage_bytes, 0);
        assert_eq!(a.mesh_stage_vertex_streams + b.mesh_stage_vertex_streams, 0);
    }
    let reference = ordinary.scene(&fixture, Auto, true, false);
    let stage = enabled.scene(&fixture, MeshStages, true, false);
    stage.report("meshlet residency");
    let a = reference.stats.lock().expect("ordinary");
    let b = stage.stats.lock().expect("enabled");
    assert_eq!(b.draw_path, Some(MeshStages), "{b:?}");
    let expected_storage: u64 = fixture
        .placements
        .iter()
        .map(|placement| {
            let mesh = fixture.mesh(&placement.mesh);
            [
                size_of_val(mesh.meshlets()),
                size_of_val(mesh.meshlet_vertices()),
                size_of_val(mesh.meshlet_triangles()),
            ]
            .into_iter()
            .map(|size| (size as u64).next_multiple_of(4).max(16))
            .sum::<u64>()
        })
        .sum();
    assert_eq!(b.mesh_stage_bytes, expected_storage);
    assert_eq!(b.mesh_stage_vertex_streams, 4);
    assert_eq!(
        b.mesh_buffer_bytes,
        a.mesh_buffer_bytes + b.mesh_stage_bytes
    );
    assert_eq!(
        b.swaps[0].uploaded_bytes,
        a.swaps[0].uploaded_bytes + expected_storage
    );
    assert!(b.swaps[0].upload_frames > 1);
    assert!(b.max_frame_copy_bytes > 0 && b.max_frame_copy_bytes <= thrum_asset_gpu::COPY_UNIT);
}

#[test]
fn parity() {
    let fixture = support::Fixture::new("parity", false);
    let gpu = support::Gpu::new(true, "apple-tile", |_| {});
    let mut mesh = gpu.scene(&fixture, MeshStages, false, false);
    let mut indexed = gpu.scene(&fixture, Indirect, false, false);
    for (label, eye, cut) in [
        ("warm", [0.0, 3.1, 4.6], true),
        ("history", [0.22, 3.1, 4.6], false),
        ("cut", [-0.62, 2.8, 4.2], true),
    ] {
        let view = support::camera(eye, [0.0, 0.0, 0.0]);
        for (arm, scene) in [("mesh", &mut mesh), ("indexed", &mut indexed)] {
            scene.camera(&view, cut);
            let image = scene.frame();
            scene.report(label);
            assert_eq!(
                scene.stats.lock().expect("stats").last_frame_history,
                Some(!cut)
            );
            assert_eq!(
                scene.stats.lock().expect("stats").mesh_stage_draws,
                usize::from(arm == "mesh")
            );
            image.save("parity", &format!("{label}-{arm}"));
        }
    }
    let clear = support::Fixture::new("parity-clear", false);
    let mut clear_sun = gpu.scene(&clear, Indirect, false, false);
    clear_sun.camera(&support::camera([0.0, 3.1, 4.6], [0.0, 0.0, 0.0]), true);
    clear_sun.frame().save("parity", "no-shadow");
}

#[test]
fn cull_equal() {
    let fixture = support::Fixture::new("cull-equal", false);
    let gpu = support::Gpu::new(true, "apple-tile", |_| {});
    let geometry = fixture.mesh("meshes/small");
    let mut culled = gpu.scene(&fixture, MeshStages, true, false);
    let mut unculled = gpu.scene(&fixture, MeshStages, false, false);
    let mut rejected = [0_u32; 7];
    let mut cut_by_plane = [0_u32; 5];
    for (frame, (eye, target, cut)) in [
        ([0.0, 0.0, 0.0], [0.0, 0.0, -1.0], true),
        ([0.04, -0.03, -0.9], [0.22, 0.11, -3.0], false),
        ([0.0, 0.0, -2.98], [-0.22, 0.0, -3.8], false),
        ([0.0, 0.0, -3.2], [-0.32, 0.0, -4.0], false),
        ([0.0, 0.1, -4.9], [0.0, 0.1, -3.0], true),
        ([0.18, 0.09, -4.8], [0.0, -0.08, -3.0], false),
    ]
    .into_iter()
    .enumerate()
    {
        let view = support::camera(eye, target);
        let cuts = intersections(&view, &geometry);
        let mut images = Vec::new();
        for scene in [&mut culled, &mut unculled] {
            scene.camera(&view, cut);
            scene.clear_rejections();
            images.push(scene.frame());
            let stats = scene.stats.lock().expect("stats");
            assert_eq!(stats.mesh_stage_draws, 1, "{stats:?}");
            assert_eq!(stats.indexed_indirect_draws, 0);
            assert_eq!(stats.last_frame_history, Some(!cut));
        }
        images[0].equal(&images[1]);
        let on = culled.rejections();
        let off = unculled.rejections();
        assert_eq!(on.iter().sum::<u32>() as usize, geometry.meshlets().len());
        assert_eq!(off[0] as usize, geometry.meshlets().len());
        assert_eq!(off[1..], [0; 6]);
        for (total, count) in rejected.iter_mut().zip(on) {
            *total += count;
        }
        for (total, count) in cut_by_plane.iter_mut().zip(cuts) {
            *total += count;
        }
        writeln!(std::io::stdout(), "frame {frame} cut={cut} bytes equal, task counts [kept,left,right,bottom,top,near,cone]={on:?}; meshlets crossing planes={cuts:?}").expect("cull evidence");
        images[0].save("cull-equal", &format!("frame-{frame}"));
    }
    assert!(rejected.into_iter().all(|count| count > 0), "{rejected:?}");
    assert!(
        cut_by_plane.into_iter().all(|count| count > 0),
        "{cut_by_plane:?}"
    );
}

fn intersections(
    view: &thrum_render::synthetic::SyntheticView,
    geometry: &thrum_asset::MeshProduct,
) -> [u32; 5] {
    let matrix = glam::Mat4::from_cols_array_2d(&view.view_proj_columns());
    let row = matrix.row(3);
    let planes = [
        row + matrix.row(0),
        row - matrix.row(0),
        row + matrix.row(1),
        row - matrix.row(1),
        row - matrix.row(2),
    ];
    planes.map(|plane| {
        u32::try_from(
            geometry
                .meshlets()
                .iter()
                .filter(|meshlet| {
                    let start = meshlet.vertex_offset as usize;
                    let end = start + meshlet.vertex_count as usize;
                    let distances = geometry.meshlet_vertices()[start..end]
                        .iter()
                        .map(|&index| {
                            let [x, y, z] = geometry.positions()[index as usize];
                            plane.dot(glam::vec4(x, y, z, 1.0))
                        });
                    distances.clone().any(|d| d < 0.0) && distances.into_iter().any(|d| d > 0.0)
                })
                .count(),
        )
        .expect("fixture meshlet count")
    })
}

fn edit(name: &str, [r, g, b]: [f32; 3]) -> MaterialHook {
    MaterialHook {
        name: name.into(),
        source: format!(
            "fn cooked_surface(given: CookedSurface, point: CookedSurfacePoint) -> CookedSurface {{
            var edited = given;
            edited.emissive = vec3({r:.3}, {g:.3}, {b:.3});
            return edited;
        }}"
        ),
    }
}

fn await_candidate_off_frame(
    scene: &support::Scene<'_>,
    old: &thrum_render::runtime::CreationReport,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let total = |report: &thrum_render::runtime::CreationReport| {
        report
            .pipelines
            .iter()
            .filter(|row| row.label == "material-reload")
            .map(|row| row.mesh)
            .sum::<u64>()
    };
    loop {
        let prepared = scene.host.creation_report();
        if total(&prepared) > total(old) {
            assert_eq!(prepared.frames, old.frames);
            assert_eq!(prepared.violations, 0);
            writeln!(
                std::io::stdout(),
                "material prepared outside frame {prepared}"
            )
            .expect("ledger");
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the requested material candidate did not prepare"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn assert_promoted(scene: &support::Scene<'_>, serial: u64) {
    let outcomes = scene.reload.take_outcomes();
    assert!(
        matches!(outcomes.as_slice(), [MaterialOutcome::Promoted {serial: result, ..}] if *result == serial),
        "expected installation on this submission: {outcomes:?}"
    );
    writeln!(
        std::io::stdout(),
        "submission {} {outcomes:?}",
        scene.host.creation_report().frames
    )
    .expect("material outcome");
}

#[test]
fn material() {
    let fixture = support::Fixture::new("material", true);
    let gpu = support::Gpu::new(true, "apple-tile", |limits| {
        limits.max_task_workgroup_total_count = 1;
    });
    let mut scene = gpu.scene(&fixture, MeshStages, false, false);
    scene.frame().save("material", "original");
    scene.report("mixed material scene");
    let before = scene.host.creation_report();
    let red = scene
        .reload
        .request(edit("accepted-red", [1200.0, 0.0, 0.0]));
    await_candidate_off_frame(&scene, &before);
    discard_candidate(&mut scene, red);
    let red_image = scene.frame();
    assert_promoted(&scene, red);
    red_image.save("material", "accepted");
    let invalid = scene.reload.request(MaterialHook {
        name: "refused.wgsl".into(),
        source: "this is not a shader".into(),
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let refused = scene.frame();
        assert_eq!(refused.colour, red_image.colour);
        let outcomes = scene.reload.take_outcomes();
        if !outcomes.is_empty() {
            assert!(
                matches!(outcomes.as_slice(), [MaterialOutcome::Rejected {serial, diagnostics}]
                if *serial == invalid && diagnostics.iter().any(|d| d.file == "refused.wgsl"))
            );
            writeln!(std::io::stdout(), "{outcomes:?}").expect("diagnostics");
            refused.save("material", "refused");
            break;
        }
        assert!(
            Instant::now() < deadline,
            "expected invalid hook diagnostics"
        );
    }
    let before = scene.host.creation_report();
    let blue = scene
        .reload
        .request(edit("candidate-blue", [0.0, 0.0, 1200.0]));
    await_candidate_off_frame(&scene, &before);
    discard_candidate(&mut scene, blue);
    scene.frame().save("material", "after-discard");
    assert_promoted(&scene, blue);
    assert_eq!(scene.host.creation_report().violations, 0);
}

fn discard_candidate(scene: &mut support::Scene<'_>, serial: u64) {
    let submitted = scene.stats.lock().expect("stats").submitted_frames;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        scene.discard();
        let stats = scene.stats.lock().expect("stats");
        assert_eq!(stats.submitted_frames, submitted);
        assert_eq!(scene.reload.take_outcomes(), []);
        if stats.recorded_material == Some(serial) {
            assert_eq!(scene.reload.ready(), Some(serial));
            writeln!(
                std::io::stdout(),
                "discarded encoding of candidate {serial} after submission {submitted}"
            )
            .expect("discard receipt");
            return;
        }
        assert!(
            Instant::now() < deadline,
            "candidate did not reach a discarded recording"
        );
    }
}
