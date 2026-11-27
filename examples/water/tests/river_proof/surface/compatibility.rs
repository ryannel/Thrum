use thrum_water_model::{
    Bed, Body, BodyKind, Chart, ChartSamples, FlowSample, Gauge, Map, Profile, River, Stage,
};

use super::{EXTENT, TradeDepth, TradeView, Wind, channel};

const POINT: [f32; 2] = [3.25, 3.75];
const WIDTH: usize = 16;
const HEIGHT: usize = 9;
const CELLS: usize = WIDTH * HEIGHT;

#[expect(
    clippy::print_stdout,
    reason = "records paired Metal measurements for review"
)]
pub(crate) fn check(gpu: &thrum_test_support::Gpu) {
    let (mut steady, [first, second]) = fixture();
    steady.add_river(river(0, first, None)).expect("one sample");
    let mut mixed = steady.clone();
    let mut upper = second.clone();
    upper.levels.fill(3.5);
    mixed
        .add_river(river(1, second, Some(upper)))
        .expect("disconnected sampled river");
    let point = POINT.map(f64::from);
    assert_eq!(steady.sample(point), mixed.sample(point));

    let mut forcing = channel::forcing(Wind::Trade);
    forcing.wind_direction = 0.42;
    let spectrum = thrum_sea_model::Spectrum::build(&forcing).expect("trade spectrum");
    let run = |map| {
        let mut probe = TradeDepth::with_optics(gpu, map, spectrum.clone(), None);
        *probe.camera.lock().expect("chart camera") = TradeView::Above.camera(POINT, 3.0, 0);
        let mut heights = Vec::new();
        for tick in 1..=9 {
            probe.submit_tick(tick);
            let answer = probe.query_at(POINT);
            assert!(answer.surface.wet);
            assert_eq!(answer.surface.body, Some(0));
            heights.push(answer.surface.sample.height);
        }
        let [width, height] = EXTENT.map(|axis| axis as usize);
        let centre = (height / 2) * width + width / 2;
        assert!(
            probe.tap.read(&probe.scene)[centre] > 0.0,
            "water drew at centre"
        );
        (heights, probe.scene.image())
    };
    let (original, original_image) = run(steady);
    let (with_river, mixed_image) = run(mixed);
    let max_height_delta = original
        .iter()
        .zip(&with_river)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f64, f64::max);
    assert_eq!(original_image.len(), mixed_image.len());
    let max_pixel_delta = original_image
        .iter()
        .zip(mixed_image)
        .map(|(a, b)| a.abs_diff(b))
        .max()
        .expect("submitted water pixels");
    println!(
        "Metal one-sample chart: 9 matched ticks (1–9), height delta {max_height_delta:.7} m,          maximum submitted pixel channel delta {max_pixel_delta}"
    );
    assert!(
        max_height_delta < 1e-6,
        "one-sample public body query changed"
    );
    assert_eq!(max_pixel_delta, 0, "one-sample submitted shading changed");
}

fn fixture() -> (Map, [FlowSample; 2]) {
    let mut bed = vec![8.0; CELLS];
    let [first, second] = [(0, 2..=6), (1, 10..=13)].map(|(id, xs)| {
        let mut footprint = vec![false; CELLS];
        let mut currents = vec![[0.0; 2]; CELLS];
        for z in 2..=6 {
            for x in xs.clone() {
                let i = z * WIDTH + x;
                bed[i] = 0.0;
                footprint[i] = true;
                currents[i][0] = match (id, z, x) {
                    (0, 2..=3, 2..=4) => -1.0,
                    (0, 2..=3, _) => -4.0,
                    _ => 1.0,
                };
            }
        }
        let levels = vec![3.0; CELLS];
        let sample = FlowSample {
            levels: levels.clone(),
            currents: currents.clone(),
            loss: vec![0.0; CELLS],
            gains: vec![[0.0; 2]; CELLS],
        };
        let body = Body {
            name: format!("Reach {id}"),
            kind: BodyKind::Chart(Chart {
                levels,
                currents,
                footprint,
                ends: Vec::new(),
            }),
            level: 0.0,
            stage: stage(0.0),
            fetch: 20_000.0,
            gains: [0.0, 1.0, 1.0, 1.0],
        };
        (body, sample)
    });
    let map = Map::prepare(
        Bed::new([0.0; 2], 1.0, [WIDTH, HEIGHT], bed).expect("bed lattice"),
        vec![first.0, second.0],
        Profile {
            margin_height: 0.1,
            swash_ratio: 0.7,
        },
    )
    .expect("disconnected charts");
    (map, [first.1, second.1])
}

fn stage(max: f64) -> Stage {
    Stage {
        min: 0.0,
        max,
        value: 0.0,
    }
}

fn river(body: usize, lower: FlowSample, upper: Option<FlowSample>) -> River {
    let (gauges, samples, wetting) = if let Some(upper) = upper {
        (vec![0.0, 1.0], vec![lower, upper], vec![vec![None; CELLS]])
    } else {
        (vec![0.0], vec![lower], Vec::new())
    };
    River {
        name: format!("Watercourse {body}"),
        gauge: Gauge::Stage(stage(*gauges.last().expect("first gauge"))),
        gauges,
        charts: vec![ChartSamples {
            body,
            samples,
            wetting,
        }],
    }
}
