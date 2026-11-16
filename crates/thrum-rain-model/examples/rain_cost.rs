//! Cost of a hollows body of 640,000 cells: preparation (seconds, hollows, nodes, bytes)
//! and a fill step with every hollow holding water. Prints one JSON object. Run in a
//! release build by `scripts/rain_cost.py`.
use std::time::Instant;
use thrum_rain_model::{Canopy, Ground, GroundClass, Profile, Rain, RainState, TickReport, Tree};

const SIDE: usize = 800;
const CELL: f64 = 0.05;

/// 40 m square at 5 cm: a gentle fall, dips at three scales and fine relief.
fn bed() -> Ground {
    let side = SIDE as f64 * CELL;
    let heights = (0..SIDE * SIDE)
        .map(|i| {
            let (x, y) = (
                ((i % SIDE) as f64 + 0.5) * CELL,
                ((i / SIDE) as f64 + 0.5) * CELL,
            );
            let (u, v) = (2.0 * x / side - 1.0, 2.0 * y / side - 1.0);
            0.004 * x
                + 0.05 * (u.powi(6) + v.powi(6))
                + 0.03 * (x * 0.9).sin() * (y * 0.7).sin()
                + 0.012 * (x * 3.1 + 0.4).sin() * (y * 2.7).sin()
                + 0.004 * (x * 9.0 + y * 2.0).sin() * (y * 8.0).cos()
        })
        .collect();
    Ground::new([0.0, 0.0], CELL, [SIDE, SIDE], heights).expect("finite bed")
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn main() {
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let ground = bed();
    let profile = Profile::default();
    let mut prepare = Vec::new();
    let mut tree = None;
    for _ in 0..3 {
        let start = Instant::now();
        let t = Tree::prepare(&ground, &Canopy::open(), &profile).expect("within the profile");
        prepare.push(start.elapsed().as_secs_f64());
        tree = Some(t);
    }
    let tree = tree.expect("prepared");
    let hollows = tree.hollow_count();
    // Every hollow holding water: half of each one's own capacity.
    let mut state = RainState::new(&tree);
    for h in 0..hollows {
        let h = u32::try_from(h).expect("hollows fit");
        state
            .pour(&tree, h, 0.5 * tree.capacity(h))
            .expect("valid pour");
    }
    let holding = state.puddles().filter(|p| p.1 > 0.0).count();
    let rain = Rain {
        flux: 10.0 / 3600.0,
        drying: 0.0,
    };
    let class = GroundClass::bare(1.0 / 3600.0);
    let mut report = TickReport::default();
    let mut timing = |threads: usize| {
        let mut s = state.clone();
        let samples: Vec<f64> = (0..200)
            .map(|_| {
                let start = Instant::now();
                s.step(&tree, rain, class, 1.0 / 60.0, threads, &mut report)
                    .expect("valid step");
                start.elapsed().as_secs_f64()
            })
            .collect();
        median(samples)
    };
    let step_one = timing(1);
    let step_all = timing(threads);
    let record = format!(
        concat!(
            "{{\"cells\": {}, \"cell_m\": {}, \"prepare_seconds\": {:.6}, \"prepare_seconds_runs\": [{:.6}, {:.6}, {:.6}], ",
            "\"hollows\": {}, \"nodes\": {}, \"bytes\": {}, \"curve_bytes\": {}, \"curve_bytes_per_node\": {:.1}, ",
            "\"small_hollows\": {}, \"fill_step\": {{\"hollows_holding_water\": {}, \"puddles\": {}, \"tick_s\": {:.6}, ",
            "\"seconds_one_thread\": {:.9}, \"seconds_all_threads\": {:.9}, \"threads\": {}, \"samples\": 200}}}}"
        ),
        SIDE * SIDE,
        CELL,
        median(prepare.clone()),
        prepare[0],
        prepare[1],
        prepare[2],
        hollows,
        tree.node_count(),
        tree.bytes(),
        tree.curve_bytes(),
        tree.curve_bytes() as f64 / tree.node_count() as f64,
        tree.small_hollows().len(),
        holding,
        state.puddles().count(),
        1.0 / 60.0,
        step_one,
        step_all,
        threads,
    );
    #[expect(clippy::print_stdout, reason = "the record is this tool's output")]
    {
        println!("{record}");
    }
}
