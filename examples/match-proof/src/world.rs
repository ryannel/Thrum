use std::cell::{Cell, RefCell};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use crowd_2d::{Candidate, Crowd, Simulation};
use thrum_core::{
    profile::StateBytes,
    sim::{GameRuntime, HistoryGame, InputFrame, PublicationMode, RuleSetIdentity, RuntimeUndo},
};
use thrum_network::{
    input_batch::{ActionView, ActionWire, InputCodecError},
    session::{RosterEvents, SLOTS, World},
};

#[cfg(not(target_arch = "wasm32"))]
pub(super) struct NoActions;

#[cfg(not(target_arch = "wasm32"))]
impl ActionWire<()> for NoActions {
    fn encode(&self, (): &(), _: &mut [u8]) -> Result<(u32, usize), InputCodecError> {
        Err(InputCodecError::InvalidAction)
    }
    fn decode(&self, _: ActionView<'_>) -> Result<(), InputCodecError> {
        Err(InputCodecError::InvalidAction)
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct Time {
    pub(super) count: u64,
    pub(super) total: u64,
    pub(super) max: u64,
    pub(super) last: u64,
}
impl Time {
    pub(super) fn add(&mut self, value: u64) {
        self.count += 1;
        self.total += value;
        self.max = self.max.max(value);
        self.last = value;
    }
    #[cfg(not(target_arch = "wasm32"))]
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
    headless: Option<Box<[RuntimeUndo; 2]>>,
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
            headless: None,
        }
    }

    pub(super) fn without_publication(mut self) -> Self {
        self.configure_headless();
        self
    }

    fn configure_headless(&mut self) {
        // On the Mac several proof processes share the machine. Keep each
        // step on its own thread so a paused worker cannot hold up the next
        // socket poll.
        #[cfg(not(any(target_arch = "wasm32", target_os = "macos")))]
        if self.count == 10_000 {
            self.runtime
                .set_execution_mode(thrum_core::sim::ExecutionMode::Rayon);
        }
        self.headless = Some(Box::new(std::array::from_fn(|_| self.runtime.new_undo())));
    }

    pub(super) fn digest_value(&self) -> u64 {
        self.runtime
            .world_digest(&self.game.resources, |_, _| {})
            .expect("registered crowd schema")
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn epoch(&self) -> u128 {
        self.epoch
    }
}

impl CrowdWorld {
    fn step_runtime(&mut self) -> Result<(), String> {
        if let Some(undo) = &mut self.headless {
            // A deferred endpoint keeps the last preimages leased. The other
            // record can be reused without allocation on this tick.
            let current = usize::from(!self.runtime.tick().is_multiple_of(2));
            self.runtime
                .step_recording(
                    &mut self.game,
                    &mut self.candidate,
                    &mut undo[current],
                    PublicationMode::Defer,
                )
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        self.runtime
            .step(&mut self.game, &mut self.candidate)
            .map_err(|error| error.to_string())?;
        Ok(())
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

    fn step(&mut self, frames: &[InputFrame<()>; SLOTS], _: RosterEvents) -> Result<(), String> {
        let start = self.costs.then(Instant::now);
        self.game
            .prepare_history_tick(&self.runtime, self.tick() + 1, frames)
            .map_err(|error| error.to_string())?;
        self.step_runtime()?;
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

    fn image(&self, destination: &mut StateBytes) -> Result<(), String> {
        let start = self.costs.then(Instant::now);
        self.runtime
            .write_bounded_world_image(self.identity(), &self.game.resources, destination)
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
        if self.headless.is_some() {
            self.configure_headless();
        }
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

pub(super) mod fault_world {
    use super::Instant;
    use super::{
        ActionView, ActionWire, CrowdWorld, InputCodecError, InputFrame, RefCell, RosterEvents,
        RuleSetIdentity, SLOTS, World,
    };
    use crowd_2d::{Body, PlayerController, PlayerMovement};
    use std::{cell::Cell, rc::Rc};
    #[cfg(not(target_arch = "wasm32"))]
    use thrum_core::profile::Vector;
    use thrum_core::{
        profile::{Digest, Number, StateBytes, StateEncode},
        sim::SystemId,
    };
    use thrum_network::session::{ControllerInput, DigestPart, PredictedController};

    pub(crate) struct PlayerRule {
        pub(crate) movement: PlayerMovement,
        pub(crate) timing: Option<Rc<Cell<super::Time>>>,
    }

    pub(crate) struct CrowdController;

    pub(crate) fn controller_bits(state: PlayerController) -> [u32; 4] {
        [
            state.position.x,
            state.position.y,
            state.velocity.x,
            state.velocity.y,
        ]
        .map(Number::to_bits)
    }

    impl PredictedController<Shared> for CrowdController {
        type State = PlayerController;
        type Rules = PlayerRule;

        fn read(world: &Shared, slot: usize) -> Option<PlayerController> {
            world.0.borrow().controller(slot)
        }

        fn step(
            state: PlayerController,
            frame: ControllerInput<'_, u8>,
            rules: &PlayerRule,
        ) -> PlayerController {
            let start = rules.timing.as_ref().map(|_| Instant::now());
            let next = rules.movement.step(
                state,
                [
                    Number::from_axis(frame.axes[0]),
                    Number::from_axis(frame.axes[1]),
                ],
            );
            if let Some((start, time)) = start.zip(rules.timing.as_ref()) {
                let mut samples = time.get();
                samples.add(super::elapsed(start));
                time.set(samples);
            }
            next
        }

        fn same_bits(first: PlayerController, second: PlayerController) -> bool {
            controller_bits(first) == controller_bits(second)
        }
    }

    pub(crate) struct FaultWire;

    impl ActionWire<u8> for FaultWire {
        fn encode(&self, value: &u8, bytes: &mut [u8]) -> Result<(u32, usize), InputCodecError> {
            bytes[0] = *value;
            Ok((1, 1))
        }

        fn decode(&self, view: ActionView<'_>) -> Result<u8, InputCodecError> {
            match (view.schema_id, view.payload) {
                (1, [value]) => Ok(*value),
                _ => Err(InputCodecError::InvalidAction),
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) const COMPONENT: u128 = 1;
    pub(crate) const FREE_LIST: u128 = 2;
    pub(crate) const RESOURCE: u128 = 3;

    #[derive(Clone, Copy, Debug)]
    pub(crate) enum Fault {
        #[cfg(not(target_arch = "wasm32"))]
        Component,
        #[cfg(not(target_arch = "wasm32"))]
        FreeList,
        Resource,
        #[cfg(not(target_arch = "wasm32"))]
        StepRefused,
    }

    impl Fault {
        #[cfg(not(target_arch = "wasm32"))]
        pub(crate) fn part(self) -> Option<u128> {
            match self {
                Self::Component => Some(COMPONENT),
                Self::FreeList => Some(FREE_LIST),
                Self::Resource => Some(RESOURCE),
                Self::StepRefused => None,
            }
        }
    }

    pub(crate) struct Step {
        pub(crate) tick: u64,
        pub(crate) roster: RosterEvents,
        pub(crate) axes: [[f32; 3]; SLOTS],
    }

    pub(crate) struct Witness {
        pub(crate) crowd: CrowdWorld,
        pub(crate) steps: Vec<Step>,
        pub(crate) restores: Vec<(u64, u64, usize)>,
        #[cfg(feature = "sockets")]
        pub(crate) images: Option<RefCell<Vec<(u64, u64, usize)>>>,
    }

    // Session and harness share a game-owned runtime. Only the harness uses
    // the mutable handle to commit a different value to the replica.
    pub(crate) struct Shared(pub(crate) Rc<RefCell<Witness>>);

    impl Shared {
        pub(crate) fn new(crowd: CrowdWorld) -> Self {
            Self(Rc::new(RefCell::new(Witness {
                crowd,
                steps: Vec::new(),
                restores: Vec::new(),
                #[cfg(feature = "sockets")]
                images: None,
            })))
        }
    }

    impl Witness {
        #[cfg(not(target_arch = "wasm32"))]
        pub(crate) fn position(&self, slot: usize) -> Vector {
            self.controller(slot)
                .expect("valid registered crowd witness")
                .position
        }

        pub(crate) fn controller(&self, slot: usize) -> Option<PlayerController> {
            let read = self.crowd.runtime.read();
            let permit = self
                .crowd
                .runtime
                .read_permit::<Body>(SystemId(1))
                .expect("valid registered crowd witness");
            let column = read
                .column(&permit)
                .expect("valid registered crowd witness");
            column
                .at_slot(u32::try_from(slot).ok()?)
                .map(Body::controller)
        }

        #[cfg(not(target_arch = "wasm32"))]
        pub(crate) fn page(&self) -> Option<(u64, u64)> {
            let page = self.crowd.runtime.latest()?;
            let mut fingerprint = Digest::default();
            page.timeline().encode(&mut fingerprint);
            page.previous_id()
                .map_or(u64::MAX, |id| id.tick)
                .encode(&mut fingerprint);
            u32::from(page.is_baseline()).encode(&mut fingerprint);
            for &slot in page.dirty().structural() {
                slot.encode(&mut fingerprint);
            }
            page.visit(|handle, id| {
                handle.slot.encode(&mut fingerprint);
                handle.generation.encode(&mut fingerprint);
                id.0.encode(&mut fingerprint);
                u32::from(page.pair_valid(handle.slot)).encode(&mut fingerprint);
                if let Some(body) = page.component::<Body>(handle) {
                    body.encode(&mut fingerprint);
                }
                if let Some(previous) = page.previous_component::<Body>(handle) {
                    previous.encode(&mut fingerprint);
                }
            });
            Some((page.id().tick, fingerprint.finish()))
        }

        pub(crate) fn change(&mut self, fault: Fault) -> Result<(), String> {
            let before = self.crowd.digest_value();
            match fault {
                Fault::Resource => self.crowd.game.resources.hits += 1,
                #[cfg(not(target_arch = "wasm32"))]
                Fault::StepRefused => self.crowd.game.resources.sequences[0] = u64::MAX,
                #[cfg(not(target_arch = "wasm32"))]
                Fault::FreeList | Fault::Component => {
                    self.change_image(fault)?;
                }
            }
            if before == self.crowd.digest_value() {
                return Err("perturbation did not change the replica's world".into());
            }
            Ok(())
        }

        #[cfg(not(target_arch = "wasm32"))]
        fn change_image(&mut self, fault: Fault) -> Result<(), String> {
            let mut image = Vec::new();
            self.crowd
                .runtime
                .write_world_image(
                    self.crowd.identity(),
                    &self.crowd.game.resources,
                    &mut image,
                )
                .map_err(|error| error.to_string())?;
            // World image v1: header and tick, capacity and slot metadata,
            // free list, identity lookups, directory marker, then Body column.
            assert_eq!(&image[..8], b"THRWIMG1");
            let capacity = self.crowd.runtime.read().capacity();
            let free_count_at = 48 + 8 + 8 + 33 * capacity;
            let free_at = free_count_at + 8;
            let free_count = usize::try_from(u64::from_le_bytes(
                image[free_count_at..free_at]
                    .try_into()
                    .expect("valid registered crowd witness"),
            ))
            .expect("valid registered crowd witness");
            if matches!(fault, Fault::FreeList) {
                assert!(free_count > 2);
                // Change the bottom of the allocation stack. The rule's next
                // few spawns can consume the top without overwriting this order.
                let (first, rest) = image[free_at..free_at + 8].split_at_mut(4);
                first.swap_with_slice(rest);
            } else {
                let directory = free_at + 4 * free_count + 5 * capacity;
                assert_eq!(image[directory], 0, "crowd uses the physical column");
                let schema_at = directory + 1;
                assert_eq!(&image[schema_at..schema_at + 16], &COMPONENT.to_le_bytes());
                let first_body = schema_at + 16 + 8 * capacity;
                assert_eq!(image[first_body], 1);
                // The first row is a hero. The rule carries its health
                // forward while updating its position and controls.
                let health = first_body + 1 + 7 * 4;
                let old = i32::from_le_bytes(
                    image[health..health + 4]
                        .try_into()
                        .expect("valid registered crowd witness"),
                );
                image[health..health + 4].copy_from_slice(&(old + 1).to_le_bytes());
            }
            self.crowd.restore(self.crowd.tick(), &image)
        }
    }

    impl World for Shared {
        type Action = u8;
        type Error = String;
        fn tick(&self) -> u64 {
            self.0.borrow().crowd.tick()
        }
        fn identity(&self) -> RuleSetIdentity {
            self.0.borrow().crowd.identity()
        }
        fn step(
            &mut self,
            frames: &[InputFrame<u8>; SLOTS],
            roster: RosterEvents,
        ) -> Result<(), String> {
            let mut world = self.0.borrow_mut();
            let controls = std::array::from_fn(|slot| {
                let frame = &frames[slot];
                InputFrame::new(frame.origin, frame.actor, frame.tick, frame.axes)
            });
            world.crowd.step(&controls, roster)?;
            let tick = world.crowd.tick();
            world.steps.push(Step {
                tick,
                roster,
                axes: std::array::from_fn(|slot| frames[slot].axes),
            });
            Ok(())
        }
        fn digest(&self) -> u64 {
            self.0.borrow().crowd.digest()
        }
        fn digest_with_parts(&self, partial: &mut dyn FnMut(DigestPart)) -> u64 {
            let world = self.0.borrow();
            let crowd = &world.crowd;
            partial((FREE_LIST, crowd.runtime.free_list_digest()));
            let mut resources = Digest::default();
            crowd.game.resources.encode(&mut resources);
            partial((RESOURCE, resources.finish()));
            let digest = crowd
                .runtime
                .world_digest(&crowd.game.resources, |schema, value| {
                    partial((schema.0, value));
                })
                .expect("valid registered crowd witness");
            crowd.digests.borrow_mut().push((crowd.tick(), digest));
            digest
        }
        fn image(&self, destination: &mut StateBytes) -> Result<(), String> {
            let world = self.0.borrow();
            world.crowd.image(destination)?;
            #[cfg(feature = "sockets")]
            if let Some(images) = &world.images {
                images.borrow_mut().push((
                    self.tick(),
                    world.crowd.digest_value(),
                    destination.len(),
                ));
            }
            Ok(())
        }
        fn restore(&mut self, tick: u64, image: &[u8]) -> Result<(), String> {
            let mut world = self.0.borrow_mut();
            world.crowd.restore(tick, image)?;
            let restored = world
                .crowd
                .restored
                .expect("valid registered crowd witness");
            world.restores.push(restored);
            Ok(())
        }
    }
}
