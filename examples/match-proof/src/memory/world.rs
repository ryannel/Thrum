use std::{
    cell::{Cell, RefCell},
    time::Instant,
};

use crowd_2d::{Candidate, Crowd, Simulation};
use thrum_core::sim::{GameRuntime, HistoryGame, InputFrame, RuleSetIdentity};
use thrum_network::{
    input_batch::{ActionView, ActionWire, InputCodecError},
    session::{SLOTS, World},
};

pub(super) struct NoActions;

impl ActionWire<()> for NoActions {
    fn encode(&self, _: &(), _: &mut [u8]) -> Result<(u32, usize), InputCodecError> {
        Err(InputCodecError::InvalidAction)
    }
    fn decode(&self, _: ActionView<'_>) -> Result<(), InputCodecError> {
        Err(InputCodecError::InvalidAction)
    }
}

#[derive(Default)]
pub(super) struct Time {
    pub(super) count: u64,
    pub(super) total: u64,
    pub(super) max: u64,
}
impl Time {
    pub(super) fn add(&mut self, value: u64) {
        self.count += 1;
        self.total += value;
        self.max = self.max.max(value);
    }
    pub(super) fn json(&self) -> serde_json::Value {
        serde_json::json!({"samples": self.count, "total": self.total, "max": self.max})
    }
}

pub(super) struct CrowdWorld {
    runtime: GameRuntime<Crowd>,
    game: Crowd,
    candidate: Candidate,
    pub(super) digests: RefCell<Vec<(u64, u64)>>,
    pub(super) restored: Option<(u64, u64, usize)>,
    pub(super) engine: Time,
    pub(super) barrier_send: Time,
    pub(super) image_time: RefCell<Time>,
    pub(super) image_bytes: Cell<usize>,
    costs: bool,
    barrier: Option<(u64, Instant)>,
    count: u32,
    epoch: u128,
}

impl CrowdWorld {
    pub(super) fn new(count: u32, epoch: u128, authority: bool, costs: bool) -> Self {
        let (runtime, game, candidate) = if authority {
            let Simulation {
                runtime,
                game,
                candidate,
                ..
            } = Simulation::new(count, epoch);
            (runtime, game, candidate)
        } else {
            Simulation::image_parts(count, epoch, false)
        };
        Self {
            runtime,
            game,
            candidate,
            digests: RefCell::default(),
            restored: None,
            engine: Time::default(),
            barrier_send: Time::default(),
            image_time: RefCell::default(),
            image_bytes: Cell::default(),
            costs,
            barrier: None,
            count,
            epoch,
        }
    }

    pub(super) fn digest_value(&self) -> u64 {
        self.runtime
            .world_digest(&self.game.resources, |_, _| {})
            .expect("registered crowd schema")
    }

    pub(super) fn epoch(&self) -> u128 {
        self.epoch
    }
}

fn elapsed(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).expect("short measurement")
}

impl World for CrowdWorld {
    type Action = ();
    type Error = String;

    fn tick(&self) -> u64 {
        self.runtime.tick()
    }

    fn identity(&self) -> RuleSetIdentity {
        crate::rule_set(false)
    }

    fn step(&mut self, frames: &[InputFrame<()>; SLOTS], _: u8) -> Result<(), String> {
        let start = self.costs.then(Instant::now);
        self.game
            .prepare_history_tick(&self.runtime, self.tick() + 1, frames)
            .map_err(|error| error.to_string())?;
        self.runtime
            .step(&mut self.game, &mut self.candidate)
            .map_err(|error| error.to_string())?;
        if let Some(start) = start {
            self.engine.add(elapsed(start));
        }
        Ok(())
    }

    fn digest(&self) -> u64 {
        let digest = self.digest_value();
        self.digests.borrow_mut().push((self.tick(), digest));
        digest
    }

    fn image(&self, destination: &mut Vec<u8>) -> Result<(), String> {
        let start = self.costs.then(Instant::now);
        self.runtime
            .write_world_image(self.identity(), &self.game.resources, destination)
            .map_err(|error| error.to_string())?;
        if let Some(start) = start {
            self.image_time.borrow_mut().add(elapsed(start));
            self.image_bytes.set(destination.len());
        }
        Ok(())
    }

    fn restore(&mut self, tick: u64, image: &[u8]) -> Result<(), String> {
        let (mut runtime, mut game, mut candidate) =
            Simulation::image_parts(self.count, self.epoch, false);
        runtime
            .restore_world_image(self.identity(), &mut game.resources, image, 4 * 1024 * 1024)
            .map_err(|error| error.to_string())?;
        if runtime.tick() != tick {
            return Err("baseline tick differs from image".into());
        }
        runtime.publish_initial(&mut game, &mut candidate);
        self.runtime = runtime;
        self.game = game;
        self.candidate = candidate;
        self.restored = Some((tick, self.digest_value(), image.len()));
        Ok(())
    }

    fn barrier(&mut self, tick: u64) {
        if self.costs {
            self.barrier = Some((tick, Instant::now()));
        }
    }

    fn set_offered(&mut self, tick: u64) {
        if let Some((barrier, start)) = self.barrier.take()
            && tick == barrier
        {
            self.barrier_send.add(elapsed(start));
        }
    }
}
