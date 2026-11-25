//! The island gale's highest surface across the whole throw reach.
use thrum_water_model::{CutConfig, Sheet, WholeCut, cut};
use water_scene::{
    floating::fallback,
    island::{Wind, surf_inputs, surf_water_profile},
};

#[test]
fn island_gale_coarse_to_fine_matches_the_whole_cut() {
    const REVIEW: [f64; 2] = [-142.176_859_373_527_12, 8.880_178_254_307_927];
    // WP13 revised the breaker section after the design review. Its original
    // column now has one surface. This column separates the two margin rules
    // on the shipped section.
    const PITCH: [f64; 2] = [-155.827_841_241_636_04, 45.897_123_314_728_31];
    const FOLD_TIME: f64 = 18.5;
    let (map, spectrum) = surf_inputs(Wind::Gale).expect("island gale");
    let field = fallback(&spectrum, surf_water_profile(&spectrum).waves.choppiness)
        .expect("valid long waves");
    let config = map.surf_profile().expect("surf profile").cut;

    let review_sheet = map.surf_sheet(&field, 0, 2.0).expect("review sheet");
    let reference = WholeCut::new(&review_sheet, &[REVIEW], config).expect("review reach");
    let expected = reference.column(REVIEW).expect("review column").height;
    let actual = map
        .surface(&field, REVIEW, 2.0)
        .expect("review query")
        .sample
        .height;
    assert!(
        (actual - expected).abs() <= 0.01,
        "review: {actual} vs {expected}"
    );
    drop(reference);

    let sheet = map
        .surf_sheet(&field, 0, FOLD_TIME)
        .expect("island surf sheet");
    let mut columns = vec![PITCH];
    'folds: for z in 140..=193 {
        for x in -672..=-603 {
            let a = [f64::from(x) * config.cell, f64::from(z) * config.cell];
            let [a, b, c] = [
                a,
                [a[0] + config.cell, a[1]],
                [a[0] + config.cell, a[1] + config.cell],
            ]
            .map(|p| sheet.forward(p).world);
            let det = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            if det < -1e-6 {
                columns.push(std::array::from_fn(|axis| {
                    (a[axis] + b[axis] + c[axis]) / 3.0
                }));
                if columns.len() == 25 {
                    break 'folds;
                }
            }
        }
    }
    assert_eq!(columns.len(), 25);
    let whole = WholeCut::new(&sheet, &columns, config).expect("whole reach");
    std::thread::scope(|scope| {
        for chunk in columns.chunks(4) {
            let (map, field, whole) = (&map, &field, &whole);
            scope.spawn(move || {
                for &p in chunk {
                    let expected = whole.column(p).expect("highest triangle, refined");
                    assert!(expected.hits >= 3, "{p:?}: {} hits", expected.hits);
                    let actual = map
                        .surface(field, p, FOLD_TIME)
                        .expect("public surf query")
                        .sample
                        .height;
                    assert!(
                        (actual - expected.height).abs() <= 0.01,
                        "{p:?}: {actual} vs {}",
                        expected.height
                    );
                }
            });
        }
    });
    let fixed = cut(
        &sheet,
        PITCH,
        CutConfig {
            bend_factor: 0.0,
            ..config
        },
    )
    .expect("fixed-margin answer");
    let expected = whole.column(PITCH).expect("pitch column");
    assert!(
        expected.height - fixed.height > 0.01,
        "pitch: {expected:?} fixed: {fixed:?}"
    );
}
