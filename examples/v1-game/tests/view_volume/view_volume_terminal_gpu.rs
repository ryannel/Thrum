//! Terminal composition through the actual scene sky and star draw.
mod far_terminal;
#[expect(dead_code, reason = "the shared fixture also reads cumulative cells")]
mod fixture;
#[expect(dead_code, reason = "this proof reads the terminal pair directly")]
mod medium_fixture;

use fixture::{SCATTER_GAIN, STORAGE_EXPOSURE, SUN_LUX};
use medium_fixture::render;
use thrum_render::atmosphere::AtmosphereFrame;

fn compare(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= expected.abs() * 0.003 + 0.000_000_2
}

#[test]
fn metal_medium_corrects_sky_and_sun_and_attenuates_added_starlight_once() {
    let Some(gpu) = fixture::Gpu::open(9, "terminal composition through a slab") else {
        return;
    };
    let mut scene = fixture::scene(false, false, false);
    scene.view = scene
        .view
        .with_celestial_basis(glam::Mat3::IDENTITY.to_cols_array_2d())
        .expect("stars");
    let frame = |sun, lux| {
        AtmosphereFrame::new(sun, lux, [1.0; 3], 2.0, 1)
            .expect("Sun")
            .with_view_volume()
            .with_nightglow()
    };
    let slab = "select(0.0, 1.0, x >= 25.3125 && x < 45.0)";
    let disk_lux = 12_700.0;
    let lit = frame([0.0, 0.0001, -1.0], disk_lux);
    let clear_sun = render(&gpu, &scene, lit, None);
    let zero = render(
        &gpu,
        &scene,
        lit.with_view_volume_medium(1),
        Some((slab, [0.0; 3])),
    );
    assert_eq!(clear_sun.image, zero.image);
    let sun = render(
        &gpu,
        &scene,
        lit.with_view_volume_medium(1),
        Some((slab, [0.035, 0.0, 0.0])),
    );
    let pair = sun.terminal();
    println!(
        "sun clear={:?}, attenuated={:?}, pair={pair:?}",
        clear_sun.center(),
        sun.center()
    );
    for channel in 0..3 {
        let delta = pair[channel] * disk_lux * STORAGE_EXPOSURE / SCATTER_GAIN;
        assert!(compare(
            sun.center()[channel] - delta,
            pair[3] * clear_sun.center()[channel]
        ));
        assert!(sun.center()[channel] < clear_sun.center()[channel]);
    }
    const STAR: [f32; 3] = [0.0, 0.6, -0.8];
    scene.view = thrum_render::synthetic::SyntheticView::perspective(
        [0.0, 2.0, 0.0],
        [0.0, 2.6, -0.8],
        [0.0, 1.0, 0.0],
        0.75,
        33.0 / 19.0,
        0.1,
        2000.0,
    )
    .expect("above-horizon star view")
    .with_exposure(0.001)
    .expect("fixed exposure")
    .with_celestial_basis(glam::Mat3::IDENTITY.to_cols_array_2d())
    .expect("star basis");
    let night = frame([0.0, -1.0, 0.0], SUN_LUX);
    let clear_sky = render(&gpu, &scene, night, None);
    scene.star_catalogue = Some(&[[STAR[0], STAR[1], STAR[2], 0.000_3]]);
    let clear_with_star = render(&gpu, &scene, night, None);
    let fogged = render(
        &gpu,
        &scene,
        night.with_view_volume_medium(2),
        Some((slab, [0.035, 0.0, 0.127])),
    );
    let pair = fogged.terminal();
    let channel = 1;
    let background = clear_sky.center()[channel];
    let star = clear_with_star.center()[channel] - background;
    let delta = pair[channel] * SUN_LUX * STORAGE_EXPOSURE / SCATTER_GAIN
        + medium_fixture::centre_layer(&fogged, fogged.grid[2] - 1, false)[3] * STORAGE_EXPOSURE;
    let expected = pair[3] * (background + star) + delta;
    println!(
        "night sky={background}, star={star}, delta={delta}, actual={}, expected={expected}",
        fogged.center()[channel]
    );
    assert!(
        star > 0.0 && background > 0.0,
        "sky={background}, star={star}"
    );
    assert!(
        delta >= 0.1 * expected,
        "delta={delta}, expected={expected}"
    );
    assert!(
        compare(fogged.center()[channel], expected),
        "actual={} expected={expected}",
        fogged.center()[channel]
    );
    assert!(
        !compare(fogged.center()[channel], expected + delta),
        "two corrections survived"
    );
    println!(
        "sky={background} star={star} R={} delta={delta} expected={expected}",
        pair[3]
    );
}
