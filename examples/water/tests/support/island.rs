#![allow(dead_code, reason = "shared by the stage and body proofs")]
use std::path::PathBuf;
use thrum_app::sky::STORE_VARIABLE;
use thrum_cook::{CookConfig, Cooker, Durability, Quality};
use water_scene::{
    SharedMap,
    island::{CAMERAS, Wind},
};

use crate::support::Fixture;

pub(crate) const EXTENT: [u32; 2] = [1056, 768];

pub(crate) fn with_cooked_island(name: &str, test: impl FnOnce()) {
    if std::env::var_os("THRUM_ISLAND_TEST_CHILD").is_some() {
        test();
        return;
    }
    // Set the ordinary game store in a fresh process. The test runtime may
    // have other threads, so never mutate this process's environment.
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scratch = thrum_test_support::Scratch::new(&format!("island-{name}"));
    let store = scratch.join("store");
    let config = CookConfig {
        source: source.join("assets"),
        store: store.clone(),
        profiles: Vec::new(),
        quality: Quality::Fast,
        durability: Durability::Process,
    };
    let report = Cooker::open(config)
        .expect("island cooker")
        .cook_all()
        .expect("cook island");
    assert!(report.failed.is_empty(), "{report:?}");
    let status = std::process::Command::new(std::env::current_exe().expect("current test"))
        .args(["--exact", name, "--nocapture"])
        .env(STORE_VARIABLE, store)
        .env("THRUM_ISLAND_TEST_CHILD", "1")
        .status()
        .expect("test child");
    assert!(status.success(), "{name}: {status}");
}

pub(crate) fn fixture(gpu: &thrum_test_support::Gpu, draw_water: bool) -> (Fixture, SharedMap) {
    let (game, report, map) =
        water_scene::island::build(Wind::Calm, draw_water).expect("island game");
    let mut fixture = Fixture::from_game(gpu, EXTENT, game, report);
    fixture.parameters(r#"{"camera.island":4}"#);
    fixture.warm_cooked();
    (fixture, map)
}

pub(crate) fn view_proj(index: usize) -> glam::Mat4 {
    let c = CAMERAS[index];
    let view = thrum_render::synthetic::SyntheticView::perspective(
        c.position,
        c.target,
        [0.0, 1.0, 0.0],
        c.vertical_fov_degrees.to_radians(),
        EXTENT[0] as f32 / EXTENT[1] as f32,
        0.1,
        30_000.0,
    )
    .expect("top-down");
    glam::Mat4::from_cols_array_2d(&view.view_proj_columns())
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "proof landmarks are inside the frame"
)]
pub(crate) fn project(matrix: glam::Mat4, point: [f32; 3]) -> usize {
    let p = matrix.project_point3(glam::Vec3::from_array(point));
    let x = (p.x.midpoint(1.0) * EXTENT[0] as f32).floor() as usize;
    let y = ((1.0 - p.y) * 0.5 * EXTENT[1] as f32).floor() as usize;
    assert!(x < EXTENT[0] as usize && y < EXTENT[1] as usize);
    y * EXTENT[0] as usize + x
}

pub(crate) fn water(pixel: [f32; 4]) -> bool {
    pixel[0] > 0.1 && pixel[1] > 0.0
}
