use crate::Simulation;
use thrum_core::{
    ActorId, Origin, PeerId,
    profile::{Digest, StateEncode},
    sim::InputFrame,
};

/// Compact corrected timeline and witnesses from one trace execution.
#[derive(Clone, Debug, Default)]
pub struct TraceResult {
    /// One endpoint digest per tick, replaced when replay supersedes it.
    pub timeline: Vec<u64>,
    /// Fold over the corrected timeline only.
    pub fold: u64,
    /// Digests every 120 ticks.
    pub checkpoints: Vec<(u64, u64)>,
    /// Structural slot reuse count.
    pub reused: u64,
    /// Projectile hits.
    pub hits: u64,
    /// Rewinds performed.
    pub rewinds: u64,
    /// Rewinds whose replay changed a digest.
    pub changed_rewinds: u64,
    /// Raw axes changed by the real wire route.
    pub changed_axes: u64,
    /// Accepted neighbour visits, accumulated over forward ticks.
    pub neighbours: u64,
}

/// Trace frames deliberately include values off the wire grid. A correction
/// replaces player zero's first axis eight ticks before each 120-tick boundary.
pub fn frames(tick: u64, corrected: bool) -> [InputFrame<()>; 4] {
    const AXES: [f32; 8] = [0.1, 0.5, -0.5, -0.0, 0.3, -0.7, 1.0, -1.0];
    std::array::from_fn(|index| {
        let phase = (tick / 60 + index as u64) % 8;
        let mut axes = [AXES[phase as usize], AXES[((phase + 2) % 8) as usize], 0.0];
        if corrected && tick % 120 == 113 && index == 0 {
            axes[0] = -0.9;
        }
        InputFrame::new(
            Origin {
                peer: PeerId(index as u64),
                connection_epoch: 1,
                sequence: tick,
            },
            ActorId(index as u64),
            tick,
            axes,
        )
    })
}

/// Step the recorded trace using the engine's runtime and sparse history.
/// `route` optionally round-trips complete input batches and returns how many
/// axes changed. A straight run starts with corrected frames and never rewinds.
///
/// # Panics
/// If the recorded trace cannot commit or the engine's history invariants fail.
pub fn run_trace(
    simulation: &mut Simulation,
    ticks: u64,
    straight: bool,
    route: impl Fn(&mut [InputFrame<()>; 4]) -> u64,
) -> TraceResult {
    let mut result = TraceResult::default();
    for tick in 1..=ticks {
        let mut input = frames(tick, straight);
        result.changed_axes += route(&mut input);
        simulation
            .history
            .step(
                &mut simulation.runtime,
                &mut simulation.game,
                &mut simulation.candidate,
                &input,
            )
            .expect("trace tick");
        result.neighbours += simulation.candidate.neighbours;
        result.timeline.push(simulation.digest());
        if !straight && tick % 120 == 0 {
            let start = tick - 8;
            simulation
                .history
                .rewind(&mut simulation.runtime, &mut simulation.game, 8)
                .expect("eight-tick rewind");
            let mut corrected = frames(start + 1, true);
            result.changed_axes += route(&mut corrected);
            simulation
                .history
                .amend_future(
                    &simulation.runtime,
                    &simulation.game,
                    start + 1,
                    |_| Ok(corrected),
                    |record, correction| *record = correction,
                )
                .expect("correct input frame");
            let mut changed = false;
            while simulation
                .history
                .replay_next(
                    &mut simulation.runtime,
                    &mut simulation.game,
                    &mut simulation.candidate,
                )
                .expect("replay")
            {
                let position =
                    usize::try_from(simulation.runtime.tick() - 1).expect("timeline index");
                let digest = simulation.digest();
                changed |= result.timeline[position] != digest;
                result.timeline[position] = digest;
            }
            result.rewinds += 1;
            result.changed_rewinds += u64::from(changed);
        }
    }
    // A reused logical slot has generation > 0, observable through checked handles.
    for slot in 0..simulation.candidate.damage.len() {
        if let Some(live) = simulation
            .runtime
            .read()
            .live_at_slot(u32::try_from(slot).expect("slot fits"))
        {
            result.reused += u64::from(live.handle().generation > 1);
        }
    }
    result.hits = simulation.game.resources.hits;
    let mut fold = Digest::default();
    for (index, &digest) in result.timeline.iter().enumerate() {
        digest.encode(&mut fold);
        let tick = index as u64 + 1;
        if tick.is_multiple_of(120) {
            result.checkpoints.push((tick, digest));
        }
    }
    result.fold = fold.finish();
    result
}
