//! Cooked pattern editing, submission and a replacement on the actual Metal scene.
#[cfg(target_os = "macos")]
use crate::support;

use std::path::PathBuf;
use thrum_cook::{CookConfig, Cooker, Durability, Quality};
use water_scene::{
    floating::{WakeDemo, WakeView, WhitewaterDemo, build_wake},
    island::Wind,
};

// A 4×4 all-low rank pattern. The old ranked image and this edit both go
// through the normal cooker and loader.
const LOW_RANK: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 4, 0, 0, 0, 4, 8, 6, 0,
    0, 0, 169, 241, 158, 126, 0, 0, 0, 12, 73, 68, 65, 84, 120, 156, 99, 96, 160, 28, 0, 0, 0, 68,
    0, 1, 215, 227, 72, 253, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

fn cooker(root: &std::path::Path) -> Cooker {
    Cooker::open(CookConfig {
        source: root.join("assets"),
        store: root.join("store"),
        profiles: Vec::new(),
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("open foam cooker")
}

thrum_test_support::macos_gpu_test! {
fn cooked_pattern_is_lent_and_invalidates_at_submission() {
    if let Some(root) = std::env::var_os("THRUM_FOAM_LEASE_TEST") {
        running_scene(&PathBuf::from(root));
        return;
    }
    let root = thrum_test_support::Scratch::new("foam-lease");
    let textures = root.join("assets/textures");
    std::fs::create_dir_all(&textures).expect("owned asset root");
    std::fs::write(
        textures.join("foam-rank.png"),
        include_bytes!("../assets/textures/foam-rank.png"),
    )
    .expect("copy cooked pattern source");
    let first = cooker(root.path()).cook_all().expect("initial cook");
    assert!(first.failed.is_empty(), "{first:?}");
    let status = std::process::Command::new(std::env::current_exe().expect("lease test binary"))
        .args([
            "--exact",
            // The test's name in the package's `it` binary.
            "foam_lease_gpu::cooked_pattern_is_lent_and_invalidates_at_submission",
            "--nocapture",
        ])
        .env(thrum_app::sky::STORE_VARIABLE, root.join("store"))
        .env("THRUM_FOAM_LEASE_TEST", root.path())
        .output()
        .expect("run lease proof against isolated store");
    let stdout = String::from_utf8_lossy(&status.stdout);
    let stderr = String::from_utf8_lossy(&status.stderr);
    assert!(
        status.status.success(),
        "{}\n{stdout}\n{stderr}",
        status.status
    );
    // libtest passes when no test matches `--exact`, so a renamed test would otherwise run nothing.
    assert!(
        stdout.contains("running 1 test"),
        "the lease proof's child ran no test of that name\n{stdout}"
    );
}
}

#[expect(
    clippy::print_stderr,
    reason = "keep submitted lease transitions in the target evidence"
)]
#[cfg(target_os = "macos")]
fn running_scene(root: &std::path::Path) {
    let Some(gpu) = thrum_test_support::gpu("foam pattern lease", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let scene = build_wake(
        Wind::Calm,
        false,
        6.0,
        std::f32::consts::PI,
        WakeDemo {
            whitewater: WhitewaterDemo::Cooked,
            disabled: true,
            view: WakeView::Overhead,
            ..WakeDemo::default()
        },
    )
    .expect("leased craft scene");
    let mut fixture = support::Fixture::from_game(&gpu, [384, 240], scene.game, scene.report);
    fixture.frame_without_timings(0.0);
    let report = std::sync::Arc::clone(&fixture.report);
    assert_eq!(report.lock().expect("water report").foam_pattern, None);
    fixture.warm_cooked();
    let first = report.lock().expect("water report").clone();
    assert!(first.foam_pattern.is_some());
    assert_eq!(first.foam_pattern_invalidations, 1);
    for tick in (12..=240).step_by(6) {
        fixture.frame_without_timings(f64::from(tick) * 0.016_666_667);
    }
    let time = 240.0 * 0.016_666_667;
    let old = fixture.image();

    std::fs::write(root.join("assets/textures/foam-rank.png"), LOW_RANK).expect("edit pattern");
    let changed = cooker(root).cook_all().expect("cook edited pattern");
    assert!(changed.failed.is_empty(), "{changed:?}");
    let mut next = first.clone();
    for _ in 0..90 {
        fixture.frame_without_timings(time);
        next = report.lock().expect("water report").clone();
        if next.foam_pattern != first.foam_pattern {
            break;
        }
    }
    assert!(next.foam_pattern.is_some() && next.foam_pattern != first.foam_pattern);
    assert_eq!(next.foam_pattern_invalidations, 2);
    let edited = fixture.image();
    let changed_pixels = old
        .as_chunks::<4>()
        .0
        .iter()
        .zip(edited.as_chunks::<4>().0)
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        changed_pixels > 100,
        "leased edit changed {changed_pixels} pixels"
    );
    for _ in 0..4 {
        fixture.frame_without_timings(time);
    }
    assert_eq!(
        report
            .lock()
            .expect("water report")
            .foam_pattern_invalidations,
        2
    );

    fixture.replace_device(&gpu);
    fixture.frame_without_timings(time + 0.1);
    assert_eq!(report.lock().expect("water report").foam_pattern, None);
    let after_loss = fixture.image();
    assert!(
        after_loss
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[..3].iter().any(|v| *v > 24))
    );
    eprintln!(
        "Metal cooked texture serials {:?} → {:?}, one invalidation per residency change, {changed_pixels} changed pixels at one time; the replacement submitted with no pattern",
        first.foam_pattern, next.foam_pattern
    );
}
