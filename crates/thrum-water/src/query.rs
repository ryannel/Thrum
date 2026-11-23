//! Game-facing tick pages and asynchronous query answers.
mod gpu;

pub(super) use gpu::Readback;
pub(super) use gpu::validate;
use gpu::{Correction, ReadyBody};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_sea_model::LongWaves;
use thrum_water_model::{Map, Surface};

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
    /// Game tick at which this batch was posted.
    pub posted: u64,
    /// Answers, including dry points.
    pub points: Vec<QueryAnswer>,
}

/// Cumulative query counters for the scene status.
#[derive(Clone, Copy, Debug, Default)]
pub struct QueryStats {
    /// Submitted batches.
    pub dispatches: u64,
    /// Lattice nodes evaluated by completed surf column cuts, excluding refinement.
    pub cut_nodes: u64,
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
}

#[derive(Default)]
struct State {
    pages: VecDeque<Page>,
    maps: VecDeque<(u64, Arc<Map>)>,
    last_post: Option<u64>,
    ready: BTreeMap<(u32, u64), ReadyBody>,
    generations: BTreeMap<u32, u32>,
    epoch: u64,
    stats: QueryStats,
}

impl State {
    fn remember_map(&mut self, tick: u64, map: &Map, max_age: u32) {
        let index = self.maps.partition_point(|(previous, _)| *previous < tick);
        let same_tick = self
            .maps
            .get(index)
            .filter(|(previous, _)| *previous == tick);
        let previous = same_tick.or_else(|| index.checked_sub(1).and_then(|i| self.maps.get(i)));
        let snapshot = previous
            .filter(|(_, old)| old.spatial_revision() == map.spatial_revision())
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
    tick: u64,
    bodies: Vec<BodyQuery>,
}

/// Shared tick-page writer and answer mailbox. Neither operation waits for GPU work.
#[derive(Clone)]
pub struct QueryMailbox {
    state: Arc<Mutex<State>>,
    map: Arc<Mutex<Map>>,
    correction: Arc<Correction>,
    limits: QueryLimits,
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
        }
    }

    /// Enqueue a complete tick's requests and live generations. Requests for the
    /// same tick are immutable once posted. Call after releasing the water map lock.
    /// # Panics
    /// Panics if a thread panicked while mutating the mailbox.
    pub fn post(&self, tick: u64, mut bodies: Vec<BodyQuery>) {
        let map = self.map.lock().expect("water query map");
        let mut state = self.state.lock().expect("water query mailbox");
        if let Some(previous) = state.last_post {
            if previous == tick {
                return;
            }
            if previous > tick {
                state.pages.clear();
                state.maps.clear();
                state.ready.clear();
                state.epoch += 1;
            }
        }
        // Static maps share one snapshot, even across a frame that catches up
        // multiple ticks. A changed revision keeps its own connectivity and rows.
        state.remember_map(tick, &map, self.limits.max_age);
        state.last_post = Some(tick);
        drop(map);
        let oldest_page = tick.saturating_sub(u64::from(self.limits.max_age));
        while state
            .pages
            .front()
            .is_some_and(|page| page.tick < oldest_page)
        {
            let page = state.pages.pop_front().expect("expired query page");
            state.stats.unavailable += page.bodies.len() as u64;
        }
        bodies.sort_by_key(|body| (body.id, Reverse(body.generation)));
        let submitted = bodies.len();
        bodies.dedup_by_key(|body| body.id);
        let mut refused = (submitted - bodies.len()) as u64;
        state.generations = bodies
            .iter()
            .map(|body| (body.id, body.generation))
            .collect();
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
            state.pages.push_back(Page { tick, bodies });
        }
    }

    /// Take one usable completed result for each live body. Prefer the older
    /// result within the normal one-to-two tick window so the newer result can
    /// serve the next tick. A newer result supersedes one outside that window.
    /// Correct height once from the requested tick, and reject expired results.
    /// Call after releasing the water map lock, once this tick's stage is known.
    /// # Panics
    /// Panics if a thread panicked while mutating the mailbox.
    pub fn drain(&self, tick: u64, step: Duration) -> Vec<BodyAnswers> {
        let map = self.map.lock().expect("water query map");
        let mut state = self.state.lock().expect("water query mailbox");
        state.remember_map(tick, &map, self.limits.max_age);
        let mut answers: Vec<BodyAnswers> = Vec::new();
        let mut ready = std::mem::take(&mut state.ready).into_iter().peekable();
        while let Some(((id, posted), mut body)) = ready.next() {
            if state.generations.get(&id) != Some(&body.answers.generation) {
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
            let mut body = body.answers;
            match age {
                1 => state.stats.age_one += 1,
                2 => state.stats.age_two += 1,
                _ => state.stats.age_older += 1,
            }
            for answer in &mut body.points {
                if tick != answer.point.use_tick {
                    let dt = seconds_between(tick, answer.point.use_tick, step);
                    let sample = &mut answer.surface.sample;
                    sample.height = sample.height_dt.mul_add(dt, sample.height);
                    answer.surface.wet = answer.surface.body.is_some()
                        && answer.bed.is_none_or(|bed| sample.height > f64::from(bed));
                    state.stats.late += 1;
                }
            }
            answers.push(body);
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

    pub(super) fn reset_readback(&self) {
        let mut state = self.state.lock().expect("water query mailbox");
        state.epoch += 1;
        state.ready.clear();
    }
}

fn seconds_between(tick: u64, reference: u64, step: Duration) -> f64 {
    let delta = tick.abs_diff(reference) as f64 * step.as_secs_f64();
    if tick < reference { -delta } else { delta }
}
