//! Paired wall-clock latency proof through the island game's fixed-update hook.
use super::{support::Fixture, surf_cost::breaker_fleet};
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use thrum_app::sky::Camera;
use thrum_render::{extract::SourceTick, runtime::SurfaceOutput};
use thrum_water::query::{BodyAnswers, QueryCutTap, QueryMailbox};
use water_scene::{
    floating::{Body, fallback},
    island::{Wind, build_surf, surf_inputs, surf_water_profile},
};

const STEP: Duration = Duration::from_nanos(16_666_667);
const MEASURE_TICKS: u32 = 600;
// Leave room for the 60 Hz tick queue while the full scene draws at 30 Hz.
const VIEW_INTERVAL: Duration = STEP.saturating_mul(2);
const WARM_TICKS: u32 = 32;
const AGE: usize = 8;
const DRAIN_TICKS: u32 = 8;
const TOLERANCE: f64 = 0.015;
// Take the first complete delivery within each predeclared window.
const WITNESSES: [usize; 3] = [50, 300, 550];
const WITNESS_WINDOW: usize = 10;
type Witnesses = [[Vec<BodyAnswers>; WITNESS_WINDOW]; WITNESSES.len()];
const CAMERA: Camera = Camera::look_at([-2776.0, 477.0, 25.0], [-126.0, 0.0, 25.0], 1.44);

struct GameTick {
    begin: Option<u64>,
    ticks: u32,
    ages: [usize; AGE + 1],
    nodes: Vec<Option<[u64; 2]>>,
    witness: Witnesses,
    latest: [Option<BodyAnswers>; 8],
    fallback: usize,
}

impl Default for GameTick {
    fn default() -> Self {
        Self {
            begin: None,
            ticks: 0,
            ages: [0; AGE + 1],
            nodes: vec![None; MEASURE_TICKS as usize],
            witness: std::array::from_fn(|_| std::array::from_fn(|_| Vec::new())),
            latest: std::array::from_fn(|_| None),
            fallback: 0,
        }
    }
}

impl GameTick {
    fn tick(
        &mut self,
        query: &QueryMailbox,
        cuts: &QueryCutTap,
        fleet: &[Body],
        endpoint: SourceTick,
        step: Duration,
    ) {
        let Some(begin) = self.begin else {
            return;
        };
        let tick = endpoint.tick;
        let sequence = self.ticks;
        self.ticks += 1;
        let measured = begin + u64::from(WARM_TICKS);
        for batch in cuts.drain() {
            assert_eq!(batch.endpoint.epoch, endpoint.epoch);
            assert_eq!(batch.endpoint.timeline, endpoint.timeline);
            if let Some(index) = batch
                .endpoint
                .tick
                .checked_sub(measured)
                .and_then(|n| usize::try_from(n).ok())
                .filter(|&n| n < self.nodes.len())
            {
                assert!(
                    self.nodes[index].replace(batch.nodes).is_none(),
                    "one batch per posted tick"
                );
            }
        }
        for reply in query.drain(endpoint, step) {
            if let Some(index) = reply
                .posted
                .checked_sub(measured)
                .and_then(|n| usize::try_from(n).ok())
                .filter(|&n| n < self.nodes.len())
            {
                let age = usize::try_from(tick - reply.posted).expect("reply age");
                self.ages[age] += reply.points.len();
                for (&start, window) in WITNESSES.iter().zip(&mut self.witness) {
                    if let Some(answers) = index.checked_sub(start).and_then(|i| window.get_mut(i))
                    {
                        answers.push(reply.clone());
                    }
                }
            }
            let id = reply.id as usize;
            self.latest[id] = Some(reply);
        }
        if (WARM_TICKS..WARM_TICKS + MEASURE_TICKS).contains(&sequence) {
            for answer in &self.latest {
                if answer
                    .as_ref()
                    .is_none_or(|answer| tick - answer.posted > AGE as u64)
                {
                    // A held float may reuse a resolved answer inside the declared
                    // age window. Beyond it the game needs a conformance sample.
                    self.fallback += 6;
                    query.report_unavailable();
                }
            }
        }
        if sequence < WARM_TICKS + MEASURE_TICKS {
            query.post(
                endpoint,
                fleet.iter().map(|body| body.request(tick, step)).collect(),
            );
        }
    }
}

fn arm(gpu: &thrum_test_support::Gpu, extent: [u32; 2], fleet: Vec<Body>) -> GameArm {
    let (game, report, _, probes) = build_surf(Wind::Gale, true).expect("island gale");
    let query = probes.expect("island body query").query;
    let cuts = query.tap_cuts();
    let stats = Arc::new(Mutex::new(GameTick::default()));
    let game = game
        .with_presentation_camera(|_| CAMERA)
        .with_fixed_update_endpoint({
            let stats = Arc::clone(&stats);
            let query = query.clone();
            move |endpoint, step| {
                stats
                    .lock()
                    .expect("game tick")
                    .tick(&query, &cuts, &fleet, endpoint, step);
            }
        });
    let mut fixture = Fixture::from_game(gpu, extent, game, report);
    fixture.warm_cooked();
    let warm_tick = fixture
        .host
        .last_submitted_clock()
        .expect("warm clock")
        .tick;
    let warm_time = STEP * (u32::try_from(warm_tick).expect("warm tick") + 1);
    let warm_frame = fixture.frame(warm_time.as_secs_f64());
    let warm_profile = super::sample_json(&warm_frame);
    let warm_wall_ms = fixture.last_elapsed.as_secs_f64() * 1000.0;
    fixture.host.set_timing_requested(false);
    let view = fixture
        .output
        .create_view(&wgpu::TextureViewDescriptor::default());
    let output = SurfaceOutput {
        view: &view,
        extent,
    };
    let observation = fixture
        .host
        .last_submitted_clock()
        .expect("warmed scene")
        .tick;
    let start = STEP * (u32::try_from(observation).expect("warm step") + 1);
    stats.lock().expect("game tick").begin = Some(observation + 1);
    let wall = Instant::now();
    let mut submitted = 0;
    let mut dropped = 0;
    let mut completed = 0;
    let mut last_draw = None;
    loop {
        while fixture
            .host
            .poll_completed()
            .expect("nonblocking completion")
            .is_some()
        {
            completed += 1;
        }
        let elapsed = wall.elapsed();
        let opportunity = elapsed.as_nanos() / VIEW_INTERVAL.as_nanos();
        // Admit one frame per opportunity and keep at most one outstanding.
        // When there is no free slot, the fixed clock still runs. Nonblocking
        // sampling of completions avoids rounding every frame up to a tick.
        let drive = if last_draw != Some(opportunity) && fixture.host.pending_frames() == 0 {
            last_draw = Some(opportunity);
            fixture.host.drive(start + elapsed, Some(&output))
        } else {
            // An advance that can reach the device still asks for tick work.
            fixture.host.drive(start + elapsed, None)
        }
        .expect("running scene");
        assert!(drive.overload.is_none(), "{drive:?}");
        submitted += usize::from(drive.submitted);
        dropped += usize::from(drive.dropped_view);
        if stats.lock().expect("game tick").ticks >= WARM_TICKS + MEASURE_TICKS + DRAIN_TICKS {
            break;
        }
        assert!(
            wall.elapsed() < Duration::from_secs(40),
            "wall-clock ticks did not advance"
        );
        std::thread::sleep(Duration::from_micros(500));
    }
    let wall_seconds = wall.elapsed().as_secs_f64();
    let mailbox = query.stats();
    let tick_submissions = fixture.host.tick_submission_stats();
    let stats = Arc::try_unwrap({
        drop(fixture);
        stats
    })
    .unwrap_or_else(|_| panic!("fixed update still owns the game tick record"))
    .into_inner()
    .expect("game tick");
    let completed_batch_ticks = stats.nodes.iter().flatten().count();
    let nodes: Vec<_> = stats
        .nodes
        .iter()
        .map(|nodes| nodes.unwrap_or_default())
        .collect();
    GameArm {
        record: json!({
            "extent":extent, "ticks":MEASURE_TICKS, "warm_ticks":WARM_TICKS, "wall_seconds":wall_seconds,
            "render_in_flight":1, "view_opportunity_hz":30, "warm_profile":warm_profile, "warm_wall_ms":warm_wall_ms, "frames_submitted":submitted, "frames_completed":completed,
            "dropped_views":dropped, "age":stats.ages, "nodes":nodes,
            "completed_batch_ticks":completed_batch_ticks,
            "fallback":stats.fallback, "unavailable":mailbox.unavailable,
            "blocking_reads":mailbox.blocking_reads, "busy":mailbox.busy,
            "dispatches":mailbox.dispatches, "refused":mailbox.refused,
            "tick_submissions":tick_submissions.submitted,
            "tick_refused_bound":tick_submissions.refused_bound,
            "completed_cut_nodes":[mailbox.cut_coarse_nodes, mailbox.cut_fine_nodes],
        }),
        witness: stats.witness,
    }
}

struct GameArm {
    record: Value,
    witness: Witnesses,
}

pub(super) fn run(gpu: &thrum_test_support::Gpu) -> Value {
    let extent = [192, 108];
    let (breakers, _) = breaker_fleet();
    let open = (0..8)
        .map(|id| {
            Body::new(
                id,
                water_scene::floating::Hull::Float,
                [-480.0, f64::from(id) * 4.0],
                0.0,
                0.0,
            )
        })
        .collect();
    let open = arm(gpu, extent, open);
    let breakers = arm(gpu, extent, breakers);
    // Evaluate f64 after both paced arms. Reading the reference can be slow;
    // it does not contend with the wall clock, rendering or delivery.
    let (map, spectrum) = surf_inputs(Wind::Gale).expect("gale inputs");
    let field = fallback(&spectrum, surf_water_profile(&spectrum).waves.choppiness).expect("model");
    let witness =
        breakers
            .witness
            .iter()
            .zip(WITNESSES)
            .map(|(window, start)| {
                let (sequence, fleet) = window
                    .iter()
                    .enumerate()
                    .find(|(_, fleet)| {
                        fleet.len() == 8 && fleet.iter().all(|body| body.points.len() == 6)
                    })
                    .map_or((start, &[][..]), |(index, fleet)| {
                        (start + index, fleet.as_slice())
                    });
                let mut maximum = 0.0_f64;
                let stations = fleet.iter().flat_map(|body| &body.points).map(|answer| {
            let time = answer.point.use_tick as f64 * STEP.as_secs_f64();
            let position = answer.point.position.map(f64::from);
            let expected = map.surface(&field, position, time).expect("model highest surface");
            // The long height is the actual cut at the use tick. The query adds
            // footprint-filtered short waves from the submitted frame separately.
            let height = f64::from(answer.wave_heights[0]);
            let error = (height - expected.sample.height).abs();
            maximum = maximum.max(error);
            json!({"position":position, "use_tick":answer.point.use_tick, "error_m":error})
        }).collect::<Vec<_>>();
                json!({"post_index":sequence, "stations":stations, "max_error_m":maximum})
            })
            .collect::<Vec<_>>();
    json!({"target":"mac-metal", "extent":extent, "step_hz":60, "maximum_age":AGE,
        "tolerance_m":TOLERANCE,
        "camera":{"position":CAMERA.position, "target":CAMERA.target, "fov_degrees":CAMERA.vertical_fov_degrees},
        "open_water":open.record, "breakers":breakers.record,
        "witness":witness})
}
