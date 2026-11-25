//! Paired wall-clock latency proof through the island game's fixed-update hook.
use super::{support::Fixture, surf_cost::breaker_fleet};
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use thrum_app::sky::Camera;
use thrum_render::{
    extract::SourceTick,
    runtime::{FrameWorld, SurfaceOutput},
};
use thrum_water::query::{BodyAnswers, QueryCutTap, QueryMailbox};
use water_scene::{
    floating::{Body, fallback, sample_answer},
    island::{SURF_VIEWS, Wind, build_surf, surf_inputs, surf_water_profile},
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
const CAMERA: Camera = SURF_VIEWS[0];

struct GameTick {
    begin: Option<u64>,
    ticks: u32,
    ages: [usize; AGE + 1],
    nodes: Vec<Option<[u64; 2]>>,
    witness: Witnesses,
    fallback: usize,
    batches: Vec<BatchTrace>,
    held_points: usize,
}

#[derive(Default)]
struct BatchTrace {
    submitted: Option<u64>,
    published: Option<u64>,
    delivered: [Option<u64>; 8],
}

impl Default for GameTick {
    fn default() -> Self {
        Self {
            begin: None,
            ticks: 0,
            ages: [0; AGE + 1],
            nodes: vec![None; MEASURE_TICKS as usize],
            witness: std::array::from_fn(|_| std::array::from_fn(|_| Vec::new())),
            fallback: 0,
            batches: (0..MEASURE_TICKS).map(|_| BatchTrace::default()).collect(),
            held_points: 0,
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
        self.observe(cuts, endpoint, measured);
        let answers = query.drain(endpoint, step);
        let in_measurement = (WARM_TICKS..WARM_TICKS + MEASURE_TICKS).contains(&sequence);
        for reply in &answers {
            let id = usize::try_from(reply.id).expect("fleet body ID");
            if reply.held {
                if in_measurement {
                    self.held_points += reply.points.len();
                }
                continue;
            }
            if let Some(index) = self.measured_index(reply.posted, measured) {
                self.batches[index].delivered[id] = Some(tick);
                let age = usize::try_from(tick - reply.posted).expect("reply age");
                self.ages[age] += reply.points.len();
                for (&start, window) in WITNESSES.iter().zip(&mut self.witness) {
                    if let Some(answers) = index.checked_sub(start).and_then(|i| window.get_mut(i))
                    {
                        answers.push(reply.clone());
                    }
                }
            }
        }
        let requests: Vec<_> = fleet.iter().map(|body| body.request(tick, step)).collect();
        if in_measurement {
            for (body, request) in fleet.iter().zip(&requests) {
                let answer = answers.iter().find(|answer| answer.id == request.id);
                for station in 0..request.points.len() {
                    if sample_answer(answer, station, body.station(station)).is_none() {
                        self.fallback += 1;
                        query.report_unavailable();
                    }
                }
            }
        }
        if sequence < WARM_TICKS + MEASURE_TICKS {
            query.post(endpoint, requests);
        }
    }

    fn measured_index(&self, tick: u64, measured: u64) -> Option<usize> {
        tick.checked_sub(measured)
            .and_then(|n| usize::try_from(n).ok())
            .filter(|&n| n < self.batches.len())
    }

    fn observe(&mut self, cuts: &QueryCutTap, endpoint: SourceTick, measured: u64) {
        for batch in cuts.drain_submissions() {
            assert_eq!(batch.endpoint.epoch, endpoint.epoch);
            assert_eq!(batch.endpoint.timeline, endpoint.timeline);
            if let Some(index) = self.measured_index(batch.endpoint.tick, measured) {
                assert!(
                    self.batches[index]
                        .submitted
                        .replace(batch.submission_tick)
                        .is_none()
                );
            }
        }
        for batch in cuts.drain() {
            assert_eq!(batch.endpoint.epoch, endpoint.epoch);
            assert_eq!(batch.endpoint.timeline, endpoint.timeline);
            if let Some(index) = self.measured_index(batch.endpoint.tick, measured) {
                self.batches[index].published = Some(batch.publication_tick);
                assert!(
                    self.nodes[index].replace(batch.nodes).is_none(),
                    "one batch per posted tick"
                );
            }
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
    let mut view_in_flight = None::<Instant>;
    let mut completion_ms = Vec::new();
    let mut views = Vec::new();
    let mut refusals = Vec::new();
    let mut last_draw = None;
    loop {
        while let Some(view) = fixture
            .host
            .poll_completed()
            .expect("nonblocking completion")
        {
            let began = view_in_flight.take().expect("one submitted view");
            completion_ms.push(began.elapsed().as_secs_f64() * 1000.0);
            let record: &mut Value = views.last_mut().expect("view receipt");
            record["completed_s"] = json!(wall.elapsed().as_secs_f64());
            record["submission"] = json!(view.submission_order);
            record["tick"] = json!(view.tick);
        }
        if stats.lock().expect("game tick").ticks >= WARM_TICKS + MEASURE_TICKS + DRAIN_TICKS {
            if view_in_flight.is_none() {
                break;
            }
            assert!(
                wall.elapsed() < Duration::from_secs(40),
                "last view did not complete"
            );
            std::thread::sleep(Duration::from_micros(500));
            continue;
        }
        let elapsed = wall.elapsed();
        let prior = fixture.host.tick_submission_stats();
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
        if drive.submitted {
            let submitted_at = Instant::now();
            view_in_flight = Some(submitted_at);
            views.push(json!({
                "drive_started_s": elapsed.as_secs_f64(),
                "submitted_by_s": submitted_at.duration_since(wall).as_secs_f64(),
            }));
        }
        let after = fixture.host.tick_submission_stats();
        if prior.refused_bound != after.refused_bound
            || prior.refused_permission != after.refused_permission
        {
            refusals.push(json!({
                "tick": fixture.host.world().source().tick,
                "bound": after.refused_bound - prior.refused_bound,
                "permission": after.refused_permission - prior.refused_permission,
            }));
        }
        submitted += usize::from(drive.submitted);
        dropped += usize::from(drive.dropped_view);
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
    let measured = observation + 1 + u64::from(WARM_TICKS);
    let batches: Vec<_> = (measured..)
        .zip(&stats.batches)
        .map(|(posted, batch)| {
            json!({"posted":posted, "submitted":batch.submitted,
                "published":batch.published, "first_drains":batch.delivered})
        })
        .collect();
    let nodes: Vec<_> = stats
        .nodes
        .iter()
        .map(|nodes| nodes.unwrap_or_default())
        .collect();
    GameArm {
        record: json!({
            "extent":extent, "ticks":MEASURE_TICKS, "warm_ticks":WARM_TICKS, "wall_seconds":wall_seconds,
            "render_in_flight":1, "view_opportunity_hz":30, "warm_profile":warm_profile,
            "warm_wall_ms":warm_wall_ms, "frames_submitted":submitted,
            "frames_completed":completion_ms.len(), "view_completion_ms":completion_ms,
            "views":views, "batches":batches, "tick_refusals":refusals,
            "held_points":stats.held_points,
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
    let mut record = json!({"target":"mac-metal", "extent":extent, "step_hz":60, "maximum_age":AGE,
        "tolerance_m":TOLERANCE,
        "camera":{"position":CAMERA.position, "target":CAMERA.target, "fov_degrees":CAMERA.vertical_fov_degrees},
        "open_water":open.record});
    if std::env::var("THRUM_WATER_SURF_ARM").is_ok_and(|arm| arm == "open-water") {
        return record;
    }
    let breakers = breaker_fleet();
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
    record["breakers"] = breakers.record;
    record["witness"] = json!(witness);
    record
}
