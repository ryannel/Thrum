//! Game-facing tick pages and asynchronous query answers.
mod gpu;
mod rain;

pub(super) use gpu::Readback;
pub(super) use gpu::validate;
use gpu::{Correction, ReadyBody};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex, Weak},
    time::Duration,
};
use thrum_render::extract::SourceTick;
use thrum_sea_model::LongWaves;
use thrum_water_model::{Map, Surface};

const QUERY_RECORDS: &str = include_str!("query/records.wgsl");

pub(super) fn sampling_source(rivers: bool, flow: bool) -> String {
    let point = include_str!("query/point.wgsl");
    let (point_fields, answer_fields, gains, read, sum, height) = if flow {
        (
            "    flow: vec4<f32>,\n",
            "    flow: array<vec4<f32>, 2>,\n    flow_velocity: array<vec4<f32>, 2>,\n",
            include_str!("query/flow_gains.wgsl"),
            include_str!("query/flow_read.wgsl"),
            include_str!("query/flow_sum.wgsl"),
            include_str!("query/flow_height.wgsl"),
        )
    } else {
        (
            "",
            "",
            include_str!("query/gains.wgsl"),
            "",
            "",
            include_str!("query/height.wgsl"),
        )
    };
    let carriage = if rivers {
        concat!(
            include_str!("map_gpu/river_carriage.wgsl"),
            include_str!("query/river.wgsl")
        )
    } else {
        include_str!("query/plain.wgsl")
    };
    format!(
        "{point}{point_fields}{QUERY_RECORDS}{answer_fields}{}{gains}{}{read}{}{sum}{}{height}{}{carriage}",
        include_str!("query/sampling.wgsl"),
        include_str!("query/sample.wgsl"),
        include_str!("query/wind.wgsl"),
        include_str!("query/wake.wgsl"),
        include_str!("query/answer.wgsl"),
    )
}

/// Query capacity in points and the maximum age in ticks since posting.
#[derive(Clone, Copy, Debug)]
pub struct QueryLimits {
    /// All points for a body must fit together. Zero disables the query.
    pub points: u32,
    /// Discard results older than this many ticks.
    pub max_age: u32,
}

/// A world-space column and horizontal footprint in metres.
#[derive(Clone, Copy, Debug)]
pub struct QueryPoint {
    /// World x and z.
    pub position: [f32; 2],
    /// Filter radius. Zero uses the base level of every cascade.
    pub radius: f32,
    /// Tick at which the game expects to use the answer. The oldest supported
    /// tick is three before posting and the newest is two after posting.
    pub use_tick: u64,
}

/// All requested points for one game-owned body.
#[derive(Clone, Debug)]
pub struct BodyQuery {
    /// Stable identity, used to allocate capacity in ascending order.
    pub id: u32,
    /// Incarnation of this ID. Increment on replacement.
    pub generation: u32,
    /// Points in the game's own order.
    pub points: Vec<QueryPoint>,
}

/// Result at the requested tick, before any correction by the receiving game.
#[derive(Clone, Copy, Debug)]
pub struct QueryAnswer {
    /// Index in the posted body's point list.
    pub index: usize,
    /// Requested column and use tick.
    pub point: QueryPoint,
    /// Owning water body, support and all kinematic quantities.
    pub surface: Surface,
    /// Source times in seconds for long waves, three cascades and wake.
    pub sources: [Option<f64>; 5],
    /// Whether each layer is supported at the requested use tick.
    pub supported: [bool; 5],
    /// Height contributed by the interaction waves.
    pub wake_height: f32,
    /// Heights contributed by the long waves and each cascade.
    pub wave_heights: [f32; 4],
    /// Ground height, if the map has a bed here.
    pub bed: Option<f32>,
}

/// Answers to one body's request, in the posted point order.
#[derive(Clone, Debug)]
pub struct BodyAnswers {
    /// Game body ID.
    pub id: u32,
    /// Game body generation.
    pub generation: u32,
    /// World endpoint on which this batch was posted.
    pub endpoint: SourceTick,
    /// Game tick at which this batch was posted.
    pub posted: u64,
    /// True when this result was previously delivered and is being held.
    pub held: bool,
    /// Coarse and fine node evaluations charged to this body.
    pub cut_nodes: [u64; 2],
    /// Answers, including dry points.
    pub points: Vec<QueryAnswer>,
}

/// Cumulative query counters for the scene status.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueryStats {
    /// Submitted batches.
    pub dispatches: u64,
    /// Lattice nodes evaluated by completed surf column cuts, excluding refinement.
    pub cut_nodes: u64,
    /// Coarse lattice nodes evaluated, counting shared nodes once.
    pub cut_coarse_nodes: u64,
    /// Fine lattice nodes evaluated, excluding refinement.
    pub cut_fine_nodes: u64,
    /// Body requests refused for capacity or invalid point data.
    pub refused: u64,
    /// Recordings skipped because all staging buffers were occupied.
    pub busy: u64,
    /// Points moved from their requested tick on use.
    pub late: u64,
    /// Body results dropped for age or failed sampling, and unavailable uses reported by the game.
    pub unavailable: u64,
    /// Waiting GPU readbacks. The query only uses submission callbacks.
    pub blocking_reads: u64,
    /// Completed body answers taken one tick after posting.
    pub age_one: u64,
    /// Completed body answers taken two ticks after posting.
    pub age_two: u64,
    /// Completed body answers taken later than two ticks within the age limit.
    pub age_older: u64,
    /// Station uses served by a previously delivered answer.
    pub held_points: u64,
}

const CUT_RECORD_LIMIT: usize = 64;

/// Node evaluations reported by one completed tick batch.
#[derive(Clone, Copy, Debug)]
pub struct QueryCutRecord {
    /// World endpoint at which the batch was posted.
    pub endpoint: SourceTick,
    /// Most recent game drain when the readback published the batch.
    pub publication_tick: u64,
    /// Coarse and fine node evaluations, counting shared nodes once.
    pub nodes: [u64; 2],
}

/// Submission of one posted query page.
#[derive(Clone, Copy, Debug)]
pub struct QuerySubmissionRecord {
    /// World endpoint at which the batch was posted.
    pub endpoint: SourceTick,
    /// Most recent game drain or post when the batch was submitted.
    pub submission_tick: u64,
}

#[derive(Default)]
struct QueryRecords {
    submitted: VecDeque<QuerySubmissionRecord>,
    completed: VecDeque<QueryCutRecord>,
}

struct HeldBody {
    answer: BodyAnswers,
    last_taken: u64,
}

fn record_event<T>(records: &mut VecDeque<T>, event: T) {
    if records.len() == CUT_RECORD_LIMIT {
        records.pop_front();
    }
    records.push_back(event);
}

/// Bounded observer of completed batches, independent of game answer delivery.
pub struct QueryCutTap(Arc<Mutex<QueryRecords>>);

impl QueryCutTap {
    /// Remove the available records. The tap retains the last 64 completions.
    /// # Panics
    /// Panics if the probe's lock is poisoned.
    pub fn drain(&self) -> Vec<QueryCutRecord> {
        self.0
            .lock()
            .expect("water cut records")
            .completed
            .drain(..)
            .collect()
    }

    /// Remove the available submission records, retaining at most 64.
    /// # Panics
    /// Panics if the probe's lock is poisoned.
    pub fn drain_submissions(&self) -> Vec<QuerySubmissionRecord> {
        self.0
            .lock()
            .expect("water query records")
            .submitted
            .drain(..)
            .collect()
    }
}

#[derive(Default)]
struct State {
    rain_tracks: Option<Arc<crate::map_gpu::WaveTracks>>,
    pages: VecDeque<Page>,
    maps: VecDeque<(u64, Arc<Map>)>,
    last_post: Option<u64>,
    last_drain: Option<u64>,
    world: Option<(u128, u64)>,
    ready: BTreeMap<(u32, u64), ReadyBody>,
    held: BTreeMap<u32, HeldBody>,
    generations: BTreeMap<u32, u32>,
    epoch: u64,
    stats: QueryStats,
    cut_tap: Weak<Mutex<QueryRecords>>,
}

impl State {
    fn enter_world(&mut self, endpoint: SourceTick) {
        let world = (endpoint.epoch, endpoint.timeline);
        if self.world != Some(world) {
            self.pages.clear();
            self.maps.clear();
            self.ready.clear();
            self.held.clear();
            self.generations.clear();
            self.last_post = None;
            self.last_drain = None;
            self.epoch += 1;
            self.world = Some(world);
        }
    }

    fn expire_held(&mut self, oldest: u64) {
        self.held.retain(|id, held| {
            held.answer.posted >= oldest
                && self.generations.get(id) == Some(&held.answer.generation)
        });
    }

    fn observed_tick(&self, posted: u64) -> u64 {
        self.last_drain.max(self.last_post).unwrap_or(posted)
    }

    fn expire_pages(&mut self, oldest: u64) {
        while self
            .pages
            .front()
            .is_some_and(|page| page.endpoint.tick < oldest)
        {
            let page = self.pages.pop_front().expect("expired query page");
            self.stats.unavailable += page.bodies.len() as u64;
        }
    }

    fn remember_map(&mut self, tick: u64, map: &Map, max_age: u32) {
        let index = self.maps.partition_point(|(previous, _)| *previous < tick);
        let same_tick = self
            .maps
            .get(index)
            .filter(|(previous, _)| *previous == tick);
        let previous = same_tick.or_else(|| index.checked_sub(1).and_then(|i| self.maps.get(i)));
        let snapshot = previous
            .filter(|(_, old)| map.query_revision() == old.query_revision())
            .map_or_else(|| Arc::new(map.clone()), |(_, old)| Arc::clone(old));
        if same_tick.is_some() {
            self.maps[index].1 = snapshot;
        } else {
            self.maps.insert(index, (tick, snapshot));
        }
        // Each live page can request up to three ticks before it was posted.
        // Keep the preceding map as well, for intervals with no publication.
        let newest = self.maps.back().expect("just remembered a map").0;
        let oldest = newest.saturating_sub(u64::from(max_age) + 3);
        let expired = self.maps.partition_point(|(tick, _)| *tick < oldest);
        self.maps.drain(..expired.saturating_sub(1));
    }
}

struct Page {
    endpoint: SourceTick,
    bodies: Vec<BodyQuery>,
}

/// Shared tick-page writer and answer mailbox. Neither operation waits for GPU work.
#[derive(Clone)]
pub struct QueryMailbox {
    state: Arc<Mutex<State>>,
    map: Arc<Mutex<Map>>,
    correction: Arc<Correction>,
    limits: QueryLimits,
    rain: Option<rain::QueryRain>,
}

impl QueryMailbox {
    pub(super) fn new(
        limits: QueryLimits,
        map: Arc<Mutex<Map>>,
        long_waves: LongWaves,
        heights: [f32; 4],
    ) -> Self {
        Self {
            state: Arc::default(),
            map,
            correction: Arc::new(Correction::new(long_waves, heights)),
            limits,
            rain: None,
        }
    }

    pub(super) fn with_rain(mut self, variances: [f64; 4], period: Option<Duration>) -> Self {
        self.rain = Some(rain::QueryRain {
            variances,
            period: period.unwrap_or(Duration::from_secs(4)).as_secs_f64(),
        });
        self
    }

    pub(super) fn set_rain_tracks(&self, map: Option<&crate::map_gpu::MapGpu>) {
        if self.rain.is_some() {
            self.state.lock().expect("water query mailbox").rain_tracks =
                map.map(crate::map_gpu::MapGpu::wave_tracks);
        }
    }

    /// Enqueue a complete tick's requests and live generations. Requests for the
    /// same tick are immutable once posted. Call after releasing the water map lock.
    /// # Panics
    /// Panics if a thread panicked while mutating the mailbox.
    pub fn post(&self, endpoint: SourceTick, mut bodies: Vec<BodyQuery>) {
        let tick = endpoint.tick;
        let map = self.map.lock().expect("water query map");
        let mut state = self.state.lock().expect("water query mailbox");
        state.enter_world(endpoint);
        if state.last_post.is_some_and(|previous| previous >= tick) {
            return;
        }
        // Unchanged domains share one snapshot. Hollow levels can change without moving the bed.
        state.remember_map(tick, &map, self.limits.max_age);
        state.last_post = Some(tick);
        drop(map);
        let oldest_page = tick.saturating_sub(u64::from(self.limits.max_age));
        state.expire_pages(oldest_page);
        bodies.sort_by_key(|body| (body.id, Reverse(body.generation)));
        let submitted = bodies.len();
        bodies.dedup_by_key(|body| body.id);
        let mut refused = (submitted - bodies.len()) as u64;
        state.generations = bodies
            .iter()
            .map(|body| (body.id, body.generation))
            .collect();
        state.expire_held(oldest_page);
        let State {
            ready,
            generations,
            stats,
            ..
        } = &mut *state;
        ready.retain(|(id, posted), answer| {
            if generations.get(id) != Some(&answer.answers.generation) {
                return false;
            }
            if *posted < oldest_page {
                stats.unavailable += 1;
                return false;
            }
            true
        });
        let mut available = self.limits.points as usize;
        bodies.retain(|body| {
            let valid = body.points.len() <= available
                && body.points.iter().all(|point| {
                    point.radius.is_finite()
                        && point.radius >= 0.0
                        && point.position.iter().all(|value| value.is_finite())
                        && point.use_tick >= tick.saturating_sub(3)
                        && point.use_tick <= tick.saturating_add(2)
                });
            if valid {
                available -= body.points.len();
            } else {
                refused += 1;
            }
            valid && !body.points.is_empty()
        });
        state.stats.refused += refused;
        if !bodies.is_empty() {
            state.pages.push_back(Page { endpoint, bodies });
        }
    }

    /// Take one usable completed result for each live body. Prefer the older
    /// result within the normal one-to-two tick window so the newer result can
    /// serve the next tick. A newer result supersedes one outside that window.
    /// If there is no new result for a body, carry its previously delivered
    /// result from the requested tick until it expires, once in a tick where
    /// the body has received nothing. Later drains can still take fresh results.
    /// Call after releasing the water map lock, once this tick's stage is known.
    /// # Panics
    /// Panics if a thread panicked while mutating the mailbox.
    pub fn drain(&self, endpoint: SourceTick, step: Duration) -> Vec<BodyAnswers> {
        let tick = endpoint.tick;
        let map = self.map.lock().expect("water query map");
        let mut state = self.state.lock().expect("water query mailbox");
        state.enter_world(endpoint);
        state.last_drain = state.last_drain.max(Some(tick));
        let oldest = tick.saturating_sub(u64::from(self.limits.max_age));
        state.expire_pages(oldest);
        state.expire_held(oldest);
        state.remember_map(tick, &map, self.limits.max_age);
        let mut answers: Vec<BodyAnswers> = Vec::new();
        let mut ready = std::mem::take(&mut state.ready).into_iter().peekable();
        while let Some(((id, posted), mut body)) = ready.next() {
            if state.generations.get(&id) != Some(&body.answers.generation)
                || state
                    .held
                    .get(&id)
                    .is_some_and(|held| held.answer.posted >= posted)
            {
                continue;
            }
            let age = tick.saturating_sub(posted);
            if age > u64::from(self.limits.max_age) {
                state.stats.unavailable += 1;
                continue;
            }
            // A hitch can fill the queue with valid but older answers. Once
            // newer answers arrive, recover the normal latency rather than
            // retaining an age-three backlog indefinitely.
            if age > 2
                && ready
                    .peek()
                    .is_some_and(|((next_id, next_tick), _)| *next_id == id && *next_tick < tick)
            {
                continue;
            }
            if tick <= posted || answers.last().is_some_and(|answer| answer.id == id) {
                state.ready.insert((id, posted), body);
                continue;
            }
            if self
                .correction
                .resolve(&mut body, |use_tick| {
                    if use_tick >= tick {
                        return &map;
                    }
                    let index = state.maps.partition_point(|(saved, _)| *saved <= use_tick);
                    index
                        .checked_sub(1)
                        .map_or(&map, |index| &state.maps[index].1)
                })
                .is_none()
            {
                state.stats.unavailable += 1;
                continue;
            }
            let mut taken = body.answers.clone();
            match age {
                1 => state.stats.age_one += 1,
                2 => state.stats.age_two += 1,
                _ => state.stats.age_older += 1,
            }
            state.stats.late += carry_heights(&mut taken, tick, step);
            state.held.insert(
                id,
                HeldBody {
                    answer: body.answers,
                    last_taken: tick,
                },
            );
            answers.push(taken);
        }
        let State {
            held,
            stats,
            rain_tracks,
            ..
        } = &mut *state;
        let new_answers = answers.len();
        for body in held.values_mut() {
            if tick <= body.last_taken {
                continue;
            }
            let mut taken = body.answer.clone();
            taken.held = true;
            body.last_taken = tick;
            carry_heights(&mut taken, tick, step);
            stats.held_points += taken.points.len() as u64;
            answers.push(taken);
        }
        if answers.len() != new_answers {
            answers.sort_unstable_by_key(|answer| answer.id);
        }
        if !answers.is_empty()
            && let Some(rain) = &self.rain
        {
            rain.add(&mut answers, &map, rain_tracks.as_deref(), tick, step);
        }
        answers
    }

    /// Count one use without a GPU answer, for example when using the CPU fallback.
    /// # Panics
    /// Panics if the mailbox lock was poisoned.
    pub fn report_unavailable(&self) {
        self.state
            .lock()
            .expect("water query mailbox")
            .stats
            .unavailable += 1;
    }

    /// Read cumulative query counters for display or diagnostics.
    /// # Panics
    /// Panics if the mailbox lock was poisoned.
    pub fn stats(&self) -> QueryStats {
        self.state.lock().expect("water query mailbox").stats
    }

    /// Observe cut counts at GPU completion, even if the game supersedes an answer.
    /// # Panics
    /// Panics if the mailbox lock is poisoned.
    pub fn tap_cuts(&self) -> QueryCutTap {
        let mut state = self.state.lock().expect("water query mailbox");
        let records = state.cut_tap.upgrade().unwrap_or_else(|| {
            let records = Arc::new(Mutex::new(QueryRecords::default()));
            state.cut_tap = Arc::downgrade(&records);
            records
        });
        QueryCutTap(records)
    }

    pub(super) fn reset_readback(&self) {
        let mut state = self.state.lock().expect("water query mailbox");
        state.epoch += 1;
        state.ready.clear();
        state.held.clear();
    }
}

fn carry_heights(body: &mut BodyAnswers, tick: u64, step: Duration) -> u64 {
    let mut carried = 0;
    for answer in &mut body.points {
        let dt = seconds_between(tick, answer.point.use_tick, step);
        if dt != 0.0 {
            let sample = &mut answer.surface.sample;
            sample.height = sample.height_dt.mul_add(dt, sample.height);
            answer.surface.wet = answer.surface.body.is_some()
                && answer.bed.is_none_or(|bed| sample.height > f64::from(bed));
            carried += 1;
        }
    }
    carried
}

fn seconds_between(tick: u64, reference: u64, step: Duration) -> f64 {
    let delta = tick.abs_diff(reference) as f64 * step.as_secs_f64();
    if tick < reference { -delta } else { delta }
}
