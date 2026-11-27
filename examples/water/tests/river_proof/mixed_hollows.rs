use crate::support::optics::{EXTENT, Rig, attach, camera, sun};
use std::sync::{Arc, Mutex};
use thrum_render::synthetic::DisplayOverlay;
use thrum_water_model::{
    Bed, Body, BodyKind, Chart, ChartSamples, FlowSample, Gauge, Hollows, Map, Profile, River,
    Stage,
};

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::print_stdout,
    reason = "bounded viewport pixels and recorded Metal depth measurements"
)]
pub(crate) fn check(gpu: &thrum_test_support::Gpu) {
    let size = [16, 9];
    let cells = size[0] * size[1];
    let mut heights = vec![8.0; cells];
    let mut footprint = vec![false; cells];
    for z in 2..=6 {
        for x in 10..=13 {
            heights[z * size[0] + x] = 0.0;
            footprint[z * size[0] + x] = true;
        }
    }
    let stage = Stage {
        min: 0.0,
        max: 0.0,
        value: 0.0,
    };
    let chart = Body {
        name: "Reach".into(),
        level: 0.0,
        stage,
        fetch: 100.0,
        gains: [0.0; 4],
        kind: BodyKind::Chart(Chart {
            levels: vec![3.0; cells],
            currents: vec![[0.0; 2]; cells],
            footprint,
            ends: vec![],
        }),
    };
    let puddle = Body {
        name: "Side puddle".into(),
        level: 0.0,
        stage,
        fetch: 1.0,
        gains: [0.0; 4],
        kind: BodyKind::Hollows(
            Hollows::new(
                [7.5, 3.0],
                0.25,
                [2, 2],
                vec![8.0; 4],
                vec![0; 4],
                vec![8.1],
            )
            .expect("side puddle"),
        ),
    };
    let baseline = Map::prepare(
        Bed::new([0.0; 2], 1.0, size, heights).expect("bed"),
        vec![chart, puddle],
        Profile {
            margin_height: 0.1,
            swash_ratio: 0.7,
        },
    )
    .expect("disjoint chart and hollows");
    let mut sampled = baseline.clone();
    let samples = [3.0, 3.5].map(|level| FlowSample {
        levels: vec![level; cells],
        currents: vec![[0.0; 2]; cells],
        loss: vec![0.0; cells],
        gains: vec![[0.0; 2]; cells],
    });
    sampled
        .add_river(River {
            name: "Two samples".into(),
            gauge: Gauge::Stage(Stage { max: 1.0, ..stage }),
            gauges: vec![0.0, 1.0],
            charts: vec![ChartSamples {
                body: 0,
                samples: samples.to_vec(),
                wetting: vec![vec![None; cells]],
            }],
        })
        .expect("sampled chart");
    sampled.set_gauge(0, 1.0).expect("upper sample");
    let forcing = water_scene::channel::forcing(water_scene::island::Wind::Trade);
    let spectrum = thrum_sea_model::Spectrum::build(&forcing).expect("trade spectrum");
    let view = camera([9.4, 14.0, 3.4], [9.4, 3.0, 3.5], 0.7, 1.0 / 32768.0)
        .with_temporal_inputs(true)
        .with_display_overlay(DisplayOverlay::TemporalInputs);
    let projection = glam::Mat4::from_cols_array_2d(&view.view_proj_columns());
    let [width, height] = EXTENT.map(|axis| u16::try_from(axis).expect("small test target"));
    for (name, map, chart_level) in [
        ("steady chart + hollows", baseline, 3.0),
        ("sampled chart + hollows", sampled, 3.5),
    ] {
        let mut rig = Rig::new(gpu, &view, sun(0.0, 0.0), &[], None);
        let scope = rig.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut profile = water_scene::profile(&spectrum);
        profile.optics = Some(thrum_water::Optics::default());
        let water =
            thrum_water::Water::mapped(Arc::new(Mutex::new(map)), spectrum.clone(), profile)
                .expect("mapped water");
        attach(&mut rig, water);
        rig.frame();
        if let Some(error) = pollster::block_on(scope.pop()) {
            panic!("{name}: {error}");
        }
        let depth = rig.submitted_depth();
        for (body, point) in [
            ("chart", [11.5, chart_level, 3.5]),
            ("hollows", [7.75, 8.1, 3.25]),
        ] {
            let ndc = projection.project_point3(glam::Vec3::from_array(point));
            assert!(ndc.x.abs() < 1.0 && ndc.y.abs() < 1.0, "{body} in view");
            let x = ((ndc.x + 1.0) * 0.5 * f32::from(width)) as usize;
            let y = ((1.0 - ndc.y) * 0.5 * f32::from(height)) as usize;
            let actual = depth[y * usize::from(width) + x];
            assert!(
                actual > 0.0 && (actual - ndc.z).abs() < 0.000_025,
                "{name}, {body}: submitted depth {actual:.8}, expected {:.8}",
                ndc.z,
            );
            println!(
                "{name}, {body}: submitted depth {actual:.8}, expected {:.8}",
                ndc.z
            );
        }
    }
}
