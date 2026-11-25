use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use crowd_2d_view::{
    CrowdView,
    proof::{BodyRect, ORDER_BAND, rectangles},
};
use serde_json::{Value, json};
use thrum_app::offscreen::Offscreen;
use thrum_render::capabilities::DeviceCapabilities;

const COUNT: u32 = 10_000;
const TICKS: usize = 1_200;
const CAPACITY: usize = 10_128;

pub(super) fn record(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    capabilities: &DeviceCapabilities,
    output: &Path,
) -> Result<(), super::Failure> {
    let off = take(device, queue, capabilities, 0.0)?;
    let derived = off["proposed_band"]
        .as_f64()
        .ok_or("missing jitter calibration")?;
    let on = take(device, queue, capabilities, derived)?;
    fs::write(
        output,
        serde_json::to_vec(&json!({
            "target": "mac-metal", "adapter": format!("{:?}", capabilities.adapter()),
            "ticks": TICKS, "crowd_rows": COUNT, "chosen_band": ORDER_BAND, "derived_band": derived,
            "derivation": "90th percentile of tick-to-tick pair sort-difference changes at exact-sort reversals on consecutive overlapping ticks, rounded up to 0.01 world units. Reversal-conditioned samples avoid counting ordinary one-way motion as jitter.",
            "pair_rule": "Compare rank changes in renderer-retained slot order for all near same-layer pairs with both identities present on the previous tick. Independent sweep over present rectangles and prior ranks.",
            "overlap": "Positive screen overlap of two committed axis-aligned sprite rectangles clipped to the whole-crowd camera; both arms report true-order errors against the chosen positive band.",
            "off": off, "on": on,
        }))?,
    )?;
    if (ORDER_BAND - derived).abs() > 1e-10 {
        return Err(format!("set Crowd band to measured value {derived}").into());
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Overlap {
    tick: usize,
    reversal: usize,
}

#[derive(Clone, Copy)]
struct Positioned {
    identity: u128,
    rank: usize,
    sort: f64,
}

struct Record {
    tick: usize,
    current: Vec<Option<Positioned>>,
    previous: Vec<Option<Positioned>>,
    overlap: HashMap<[u128; 2], Overlap>,
    reversals: Vec<u64>,
    jitter: Vec<f64>,
    pairs: u64,
    returns: u64,
    returned_pairs: HashSet<[u128; 2]>,
    errors: Vec<f64>,
    worst: Value,
    worst_error: f64,
    near_changes: u64,
    positive_band: f64,
}

impl Record {
    fn new(band: f64) -> Self {
        Self {
            tick: 0,
            current: vec![None; CAPACITY],
            previous: vec![None; CAPACITY],
            overlap: HashMap::new(),
            reversals: vec![0; TICKS],
            jitter: Vec::new(),
            pairs: 0,
            returns: 0,
            returned_pairs: HashSet::new(),
            errors: Vec::new(),
            worst: Value::Null,
            worst_error: 0.0,
            near_changes: 0,
            positive_band: band,
        }
    }

    fn order(&mut self, slots: &[u32], rectangles: &[BodyRect]) -> Result<(), super::Failure> {
        if slots.len() != rectangles.len() {
            return Err("retained order is missing live bodies".into());
        }
        self.tick += 1;
        self.current.fill(None);
        let rank = slots
            .iter()
            .enumerate()
            .map(|(rank, slot)| (*slot as usize, rank));
        for (slot, rank) in rank {
            if self.current[slot]
                .replace(Positioned {
                    rank,
                    identity: 0,
                    sort: 0.0,
                })
                .is_some()
            {
                return Err("retained order contains a duplicate body".into());
            }
        }
        for body in rectangles {
            let item = self.current[body.slot]
                .as_mut()
                .ok_or("body is missing from order")?;
            item.identity = body.identity;
            item.sort = body.sort;
        }
        Ok(())
    }

    fn pair(&mut self, a: BodyRect, b: BodyRect, band: f64) -> Result<(), super::Failure> {
        if a.layer != b.layer {
            return Ok(());
        }
        let current_a = self.current[a.slot].expect("ranked body");
        let current_b = self.current[b.slot].expect("ranked body");
        let a_front = current_a.rank > current_b.rank;
        let sort_distance = a.sort - b.sort;
        let winning_lead = if a_front {
            sort_distance
        } else {
            -sort_distance
        };
        let dx = (a.centre[0] - b.centre[0]).abs();
        let dy = (a.centre[1] - b.centre[1]).abs();
        let sum = a.half + b.half;
        let reach = std::f64::consts::SQRT_2 * sum + 2.0 * band;
        let overlaps = dx < sum
            && dy < sum
            && a.centre[0] - a.half < 142.4
            && b.centre[0] - b.half < 142.4
            && a.centre[0] + a.half > -142.4
            && b.centre[0] + b.half > -142.4
            && a.centre[1] - a.half < 89.0
            && b.centre[1] - b.half < 89.0
            && a.centre[1] + a.half > -89.0
            && b.centre[1] + b.half > -89.0;
        let near = band > 0.0 && dx.mul_add(dx, dy * dy) <= reach * reach;
        if !overlaps && !near {
            return Ok(());
        }
        let previous_a = self.previous[a.slot].filter(|slot| slot.identity == a.identity);
        let previous_b = self.previous[b.slot].filter(|slot| slot.identity == b.identity);
        let changed = previous_a
            .zip(previous_b)
            .filter(|(a, b)| (a.rank > b.rank) != a_front);
        if near && changed.is_some() {
            self.near_changes += 1;
            if winning_lead <= band {
                return Err(format!(
                    "near pair {} / {} reversed with lead {} at tick {}",
                    a.identity, b.identity, winning_lead, self.tick
                )
                .into());
            }
        }
        if overlaps {
            self.pairs += 1;
            let mut pair = [a.identity, b.identity];
            pair.sort_unstable();
            let history = self.overlap.entry(pair).or_insert(Overlap {
                tick: 0,
                reversal: 0,
            });
            if let Some((old_a, old_b)) = changed
                && self.tick > 1
                && history.tick == self.tick - 1
            {
                self.reversals[self.tick - 1] += 1;
                if history.reversal > 0 && self.tick - history.reversal <= 30 {
                    self.returns += 1;
                    self.returned_pairs.insert(pair);
                }
                history.reversal = self.tick;
                if band == 0.0 {
                    self.jitter
                        .push((sort_distance - (old_a.sort - old_b.sort)).abs());
                }
            }
            history.tick = self.tick;
            let error = (-winning_lead).max(0.0);
            if error > 0.0 {
                self.errors.push(error);
            }
            if self.worst.is_null() || error > self.worst_error {
                self.worst_error = error;
                self.worst = json!({"tick": self.tick, "identities": pair.map(|id| id.to_string()),
                    "centre_a": a.centre, "centre_b": b.centre, "error": error,
                    "band_multiple": (self.positive_band > 0.0).then(|| error / self.positive_band)});
            }
        }
        Ok(())
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "pair and jitter counts are bounded by the trace and fit within a double"
    )]
    fn finish(mut self) -> Value {
        self.jitter.sort_unstable_by(f64::total_cmp);
        let percentile = |numerator| {
            self.jitter
                .get(self.jitter.len().saturating_sub(1) * numerator / 100)
                .copied()
        };
        let proposed = percentile(90).map(|p90| (100.0 * p90).ceil() / 100.0);
        let band = proposed.unwrap_or(self.positive_band);
        let above = |threshold| {
            self.errors
                .iter()
                .filter(|&&value| value > threshold)
                .count()
        };
        json!({
            "overlapping_pair_ticks": self.pairs,
            "reversals_per_tick": self.reversals,
            "total_reversals": self.reversals.iter().sum::<u64>(),
            "returns_within_30_ticks": self.returns, "distinct_returning_pairs": self.returned_pairs.len(),
            "share_over_one_band": above(band) as f64 / self.pairs as f64,
            "share_over_two_bands": above(2.0 * band) as f64 / self.pairs as f64,
            "worst": self.worst, "near_pair_rank_changes_checked": self.near_changes,
            "reversal_jitter_world_units": {"samples": self.jitter.len(), "p50": percentile(50),
                "p90": percentile(90), "p99": percentile(99), "max": self.jitter.last()},
            "proposed_band": proposed,
        })
    }
}

fn take(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    capabilities: &DeviceCapabilities,
    band: f64,
) -> Result<Value, super::Failure> {
    let game = CrowdView::new(COUNT, false)?.with_band(band).exact_steps();
    let mut runtime = Offscreen::new(game, device, capabilities, queue, [320, 200])?;
    runtime.set_timing_requested(false);
    let mut bodies = Vec::with_capacity(CAPACITY);
    let mut record = Record::new(band);
    for _ in 0..TICKS {
        let report = runtime.step()?;
        if !report.submitted {
            return Err("band evidence missed a submission".into());
        }
        rectangles(runtime.host_mut().world_mut(), &mut bodies)?;
        bodies.sort_unstable_by(|a, b| a.centre[0].total_cmp(&b.centre[0]));
        let widest = bodies.iter().map(|body| body.half).fold(0.0, f64::max);
        record.order(
            runtime
                .host()
                .sprite_world_order()
                .ok_or("expected a 2D scene")?,
            &bodies,
        )?;
        for (index, &left) in bodies.iter().enumerate() {
            let horizon = std::f64::consts::SQRT_2 * (left.half + widest) + 2.0 * band;
            for &right in &bodies[index + 1..] {
                if right.centre[0] - left.centre[0] > horizon {
                    break;
                }
                record.pair(left, right, band)?;
            }
        }
        std::mem::swap(&mut record.current, &mut record.previous);
        if record.tick.is_multiple_of(30) {
            record
                .overlap
                .retain(|_, state| state.tick + 30 >= record.tick);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while runtime.completed()?.is_none() {
            if std::time::Instant::now() >= deadline {
                return Err("band frame did not retire".into());
            }
            std::thread::sleep(std::time::Duration::from_micros(200));
        }
    }
    Ok(record.finish())
}
