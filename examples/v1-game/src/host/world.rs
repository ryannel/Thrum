//! The game's single authoritative clock and rules.

use std::time::Duration;

pub(super) use thrum_core::sim::WorldAdvance;
use thrum_core::{
    ClockConfig,
    sim::{ClockedWorld, SnapshotId, WorldGame, WorldPublication, WorldStep},
};

use crate::{
    scene::{self, GameContent},
    simulation::{PublishedPage, SimError, Simulation},
};

#[cfg(not(target_arch = "wasm32"))]
use super::world_thread::WorldClient;

pub(crate) enum WorldPort {
    Inline(Box<WorldDriver>),
    #[cfg(not(target_arch = "wasm32"))]
    Threaded(Box<WorldClient>),
    #[cfg(not(target_arch = "wasm32"))]
    Starting,
}

impl WorldPort {
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::unused_self,
            reason = "the shared world-port method dispatches native threaded variants; Wasm currently retains only inline"
        )
    )]
    pub(super) fn is_threaded(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            matches!(self, Self::Threaded(_))
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn inline(&self) -> &WorldDriver {
        match self {
            Self::Inline(driver) => driver,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(_) | Self::Starting => {
                unreachable!("the threaded host does not expose inline world methods")
            }
        }
    }

    #[cfg(all(target_os = "macos", feature = "proving"))]
    pub(super) fn inline_authority_hash(&self) -> Option<u64> {
        match self {
            Self::Inline(driver) => Some(driver.world.game().sim.authority_hash()),
            Self::Threaded(_) | Self::Starting => None,
        }
    }

    pub(super) fn tick(&self) -> u64 {
        match self {
            Self::Inline(driver) => driver.tick(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.tick(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn epoch(&self) -> u128 {
        match self {
            Self::Inline(driver) => driver.epoch(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.epoch(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn timeline(&self) -> u64 {
        match self {
            Self::Inline(driver) => driver.timeline(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.timeline(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn has_current_page(&self) -> bool {
        match self {
            Self::Inline(driver) => driver.has_current_page(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.has_current_page(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn published_page(&mut self) -> Result<PublishedPage, SimError> {
        match self {
            Self::Inline(driver) => driver.published_page(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.published_page(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn advance(&mut self, now: Duration) -> Result<WorldAdvance, String> {
        match self {
            Self::Inline(driver) => driver.advance(now),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.poll(now),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        match self {
            Self::Inline(driver) => driver.presentation_alpha_at(lead),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.presentation_alpha_at(lead),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn display_phase_at(&self, lead: Duration) -> f32 {
        match self {
            Self::Inline(driver) => driver.display_phase_at(lead),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.display_phase_at(lead),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }
    pub(super) fn is_interpolating(&self) -> bool {
        match self {
            Self::Inline(driver) => driver.is_interpolating(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.is_interpolating(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }
    pub(super) fn tick_due_time(&self) -> Option<Duration> {
        match self {
            Self::Inline(driver) => driver.world.tick_due_time(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.tick_due_time(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }
    pub(super) fn next_due_in(&self) -> Option<Duration> {
        match self {
            Self::Inline(driver) => driver.next_due_in(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(_) => None,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::unnecessary_wraps,
            reason = "the shared lifecycle boundary must refuse native threaded variants, which Wasm excludes"
        )
    )]
    pub(super) fn inline_lifecycle(&mut self) -> Result<&mut WorldDriver, String> {
        match self {
            Self::Inline(driver) => Ok(driver),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(_) | Self::Starting => Err(
                "inline world lifecycle unavailable: use WorldControls native requests and replies"
                    .to_owned(),
            ),
        }
    }

    pub(super) fn pause(&mut self) -> Result<(), String> {
        self.inline_lifecycle()?.pause();
        Ok(())
    }

    pub(super) fn resume(&mut self, now: Duration) -> Result<(), String> {
        self.inline_lifecycle()?.resume(now)
    }

    pub(super) fn set_controls(&mut self, axes: [f32; 3]) -> Result<(), String> {
        match self {
            Self::Inline(driver) => driver.set_controls(axes),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.set_controls(axes),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => unreachable!("world owner is starting"),
        }
    }

    pub(super) fn controlled_position(&self) -> Option<[f64; 3]> {
        match self {
            Self::Inline(driver) => driver.controlled_position(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Threaded(client) => client.controlled_position(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Starting => None,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn exit_mover_position(&self) -> Option<[f64; 3]> {
        self.inline().exit_mover_position()
    }
}

/// The game's authoritative world: its clock and rules, stepped inline on the
/// render owner or running on the engine's world lane.
pub struct GameWorld(pub(crate) WorldPort);

impl thrum_render::runtime::FrameWorld for GameWorld {
    type Page = PublishedPage;

    fn source(&self) -> thrum_render::extract::SourceTick {
        thrum_render::extract::SourceTick {
            epoch: self.0.epoch(),
            tick: self.0.tick(),
            timeline: self.0.timeline(),
        }
    }

    fn has_current_page(&self) -> bool {
        self.0.has_current_page()
    }

    fn is_threaded(&self) -> bool {
        self.0.is_threaded()
    }

    fn advance(
        &mut self,
        now: Duration,
    ) -> Result<WorldAdvance, thrum_render::runtime::FrameError> {
        self.0
            .advance(now)
            .map_err(thrum_render::runtime::FrameError::World)
    }

    fn publication(
        &mut self,
    ) -> Result<WorldPublication<Self::Page>, thrum_render::runtime::FrameError> {
        let page = self.0.published_page().map_err(|error| {
            thrum_render::runtime::FrameError::World(format!("world page: {error}"))
        })?;
        Ok(WorldPublication {
            id: page.id(),
            timeline: page.timeline(),
            page,
        })
    }

    fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        self.0.presentation_alpha_at(lead)
    }
    fn display_phase_at(&self, lead: Duration) -> f32 {
        self.0.display_phase_at(lead)
    }
    fn is_interpolating(&self) -> bool {
        self.0.is_interpolating()
    }
    fn tick_due_time(&self) -> Option<Duration> {
        self.0.tick_due_time()
    }
}

#[cfg(all(feature = "proving", any(target_os = "macos", target_arch = "wasm32")))]
impl GameWorld {
    pub(crate) fn set_menu(&mut self, menu: crate::interface_menu::MenuView) {
        let driver = self
            .0
            .inline_lifecycle()
            .expect("interface is attached before the world lane starts");
        driver.world.game_mut().menu = Some(menu);
    }
}

/// The game body stays here; scheduling and the fixed clock belong to core.
pub(crate) struct WorldDriver {
    pub(super) world: ClockedWorld<GameRules>,
}

pub(super) struct GameRules {
    entity_count: usize,
    #[cfg(feature = "proving")]
    route_overlay: scene::RouteOverlay,
    pub(super) sim: Simulation,
    desired_axes: [f32; 3],
    admitted_axes: [f32; 3],
    input_sequence: u64,
    weather_edits: Option<super::WeatherEdits>,
    rain: Option<crate::rain::ValleyRain>,
    #[cfg(all(feature = "proving", any(target_os = "macos", target_arch = "wasm32")))]
    menu: Option<crate::interface_menu::MenuView>,
}

impl WorldDriver {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn controlled_id(&self) -> thrum_core::StableId {
        self.world.game().controlled_id()
    }

    #[cfg(any(feature = "proving", test))]
    pub(super) fn new(
        content: GameContent,
        route_overlay: scene::RouteOverlay,
    ) -> Result<Self, String> {
        Self::selected(content, route_overlay, None)
    }

    #[cfg(feature = "proving")]
    pub(super) fn sprite_field(count: u32, interactive: bool) -> Result<Self, String> {
        Self::with_simulation(
            scene::RouteOverlay::Off,
            crate::sprite_field::simulation(count, interactive)?,
            count as usize,
        )
    }

    pub(super) fn selected(
        content: GameContent,
        route_overlay: scene::RouteOverlay,
        clouds: Option<crate::simulation::Weather>,
    ) -> Result<Self, String> {
        let sim = if clouds.is_some() {
            scene::weather_simulation(content, route_overlay, clouds)?
        } else {
            scene::simulation(content, route_overlay)?
        };
        Self::with_simulation(route_overlay, sim, content.entity_count())
    }

    pub(super) fn for_setup(
        setup: &super::GameSetup,
        clouds: Option<crate::simulation::Weather>,
        weather_edits: Option<super::WeatherEdits>,
        rain: Option<crate::rain::ValleyRain>,
    ) -> Result<Self, String> {
        let mut driver = Self::create_for_setup(setup, clouds)?;
        let game = driver.world.game_mut();
        game.weather_edits = weather_edits;
        game.rain = rain;
        Ok(driver)
    }

    fn create_for_setup(
        setup: &super::GameSetup,
        clouds: Option<crate::simulation::Weather>,
    ) -> Result<Self, String> {
        #[cfg(feature = "proving")]
        if setup.sun_proof_geometry != crate::SunProofGeometry::Off {
            if setup.content != GameContent::Exit || setup.route_overlay != scene::RouteOverlay::Off
            {
                return Err("Sun shadow proof geometry needs the populated exit scene".to_owned());
            }
            let sim = scene::exit_simulation(clouds, setup.sun_proof_geometry)
                .map_err(|error| error.to_string())?;
            return Self::with_simulation(setup.route_overlay, sim, setup.content.entity_count());
        }
        Self::selected(setup.content, setup.route_overlay(), clouds)
    }

    fn with_simulation(
        route_overlay: scene::RouteOverlay,
        sim: Simulation,
        entity_count: usize,
    ) -> Result<Self, String> {
        #[cfg(not(feature = "proving"))]
        let _ = route_overlay;
        let game = GameRules {
            #[cfg(feature = "proving")]
            route_overlay,
            sim,
            entity_count,
            desired_axes: [0.0; 3],
            admitted_axes: [0.0; 3],
            input_sequence: 1,
            weather_edits: None,
            rain: None,
            #[cfg(all(feature = "proving", any(target_os = "macos", target_arch = "wasm32")))]
            menu: None,
        };
        let world = ClockedWorld::new(game, ClockConfig::default(), Duration::ZERO)
            .map_err(|error| error.to_string())?;
        // Give slow ticks one chance to finish, then preserve the rest of the display callback.
        #[cfg(any(target_arch = "wasm32", target_os = "ios"))]
        let world = world.with_tick_work_slice(Duration::from_millis(2));
        Ok(Self { world })
    }

    pub(super) fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        self.world.presentation_alpha_at(lead)
    }
    pub(super) fn display_phase_at(&self, lead: Duration) -> f32 {
        self.world.display_phase_at(lead)
    }
    pub(super) fn is_interpolating(&self) -> bool {
        self.world.presentation_sample().is_interpolating()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn held_controls(&self) -> [f32; 3] {
        self.world.game().desired_axes
    }

    pub(super) fn tick(&self) -> u64 {
        self.world.game().sim.tick()
    }

    pub(super) fn epoch(&self) -> u128 {
        self.world.game().sim.epoch().0
    }

    pub(super) fn timeline(&self) -> u64 {
        self.world.game().sim.timeline()
    }

    pub(super) fn has_current_page(&self) -> bool {
        self.world.game().sim.has_current_page()
    }

    pub(super) fn published_page(&mut self) -> Result<PublishedPage, SimError> {
        let game = self.world.game_mut();
        game.recover_page()?;
        if !game.sim.has_current_page() {
            return Err(SimError::PublicationBusy);
        }
        game.published_page()
    }

    pub(super) fn next_due_in(&self) -> Option<Duration> {
        self.world.next_due_in()
    }

    pub(super) fn pause(&mut self) {
        self.world.pause();
    }

    pub(super) fn resume(&mut self, now: Duration) -> Result<(), String> {
        self.world.resume(now).map_err(|error| error.to_string())
    }

    pub(super) fn set_controls(&mut self, axes: [f32; 3]) -> Result<(), String> {
        self.world
            .set_controls(axes)
            .map_err(|error| error.to_string())
    }

    pub(super) fn controlled_position(&self) -> Option<[f64; 3]> {
        let game = self.world.game();
        let sim = &game.sim;
        sim.handle(game.controlled_id())
            .and_then(|handle| sim.get(handle).ok())
            .map(|entity| entity.position.0)
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn exit_mover_position(&self) -> Option<[f64; 3]> {
        let game = self.world.game();
        let sim = &game.sim;
        let handle = sim.handle(thrum_core::StableId(0))?;
        sim.get(handle).ok().map(|entity| entity.position.0)
    }

    pub(super) fn advance(&mut self, now: Duration) -> Result<WorldAdvance, String> {
        self.world
            .advance_inline(now)
            .map_err(|error| error.to_string())
    }
}

impl GameRules {
    fn published_page(&mut self) -> Result<PublishedPage, SimError> {
        let mut page = self.sim.published_page()?;
        if let Some(rain) = &self.rain {
            page.1 = Some(rain.page());
        }
        Ok(page)
    }

    fn recover_page(&mut self) -> Result<Option<usize>, SimError> {
        if self.sim.has_current_page() {
            return Ok(None);
        }
        match self.sim.published_page() {
            Ok(_) => Ok(Some(0)),
            Err(SimError::PublicationBusy) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

#[derive(Debug)]
pub(super) enum GameWorldError {
    Controls(thrum_core::sim::InputFrameError),
    Admission(thrum_core::sim::InputFrameError),
    Simulation {
        operation: &'static str,
        source: SimError,
    },
    Exhausted(&'static str),
    #[cfg(feature = "proving")]
    Reconstruction(String),
}

impl std::fmt::Display for GameWorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Controls(error) => write!(f, "local controls: {error}"),
            Self::Admission(error) => write!(f, "local input admission: {error:?}"),
            Self::Simulation { operation, source } => write!(f, "{operation}: {source:?}"),
            Self::Exhausted(what) => write!(f, "{what} exhausted"),
            #[cfg(feature = "proving")]
            Self::Reconstruction(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for GameWorldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Controls(error) | Self::Admission(error) => Some(error),
            Self::Simulation { source, .. } => Some(source),
            Self::Exhausted(_) => None,
            #[cfg(feature = "proving")]
            Self::Reconstruction(_) => None,
        }
    }
}

impl GameRules {
    fn controlled_id(&self) -> thrum_core::StableId {
        thrum_core::StableId((self.entity_count - 1) as u128)
    }
}

impl WorldGame for GameRules {
    type Control = [f32; 3];
    type Page = PublishedPage;
    type Error = GameWorldError;

    fn validate_control(control: Self::Control) -> Result<(), Self::Error> {
        scene::control_frame(scene::LOCAL_PEER, control, 0, 0)
            .validate_axes()
            .map_err(GameWorldError::Controls)
    }

    fn same_control(left: Self::Control, right: Self::Control) -> bool {
        left.iter()
            .zip(right)
            .all(|(left, right)| left.to_bits() == right.to_bits())
    }

    fn apply_control(&mut self, control: Self::Control) -> Result<(), Self::Error> {
        self.desired_axes = control;
        Ok(())
    }

    fn endpoint(&self) -> (SnapshotId, u64) {
        (
            SnapshotId {
                epoch: self.sim.epoch(),
                tick: self.sim.tick(),
            },
            self.sim.timeline(),
        )
    }

    fn step(&mut self) -> Result<WorldStep, Self::Error> {
        #[cfg(all(feature = "proving", any(target_os = "macos", target_arch = "wasm32")))]
        if let Some(menu) = &self.menu {
            self.desired_axes = menu.axes_for_tick(self.sim.tick().saturating_add(1));
        }
        if !Self::same_control(self.desired_axes, self.admitted_axes) {
            let tick = self
                .sim
                .tick()
                .checked_add(1)
                .ok_or(GameWorldError::Exhausted("input tick"))?;
            let next_sequence = self
                .input_sequence
                .checked_add(1)
                .ok_or(GameWorldError::Exhausted("input sequence"))?;
            let frame = scene::control_frame(
                scene::LOCAL_PEER,
                self.desired_axes,
                tick,
                self.input_sequence,
            );
            self.sim
                .admit_frame(frame)
                .map_err(GameWorldError::Admission)?;
            self.admitted_axes = self.desired_axes;
            self.input_sequence = next_sequence;
        }
        let mut commands = Vec::new();
        #[cfg(feature = "proving")]
        if self.route_overlay == scene::RouteOverlay::Reconstruction && self.sim.tick() == 59 {
            commands =
                scene::reconstruction_spawn(&self.sim).map_err(GameWorldError::Reconstruction)?;
        }
        let weather = self
            .weather_edits
            .as_ref()
            .map_or_else(super::WeatherTargets::default, super::WeatherEdits::pending);
        if let Some(target) = weather.sky {
            commands.push(crate::simulation::Command::SetWeather(target));
        }
        if let Some(target) = weather.ground {
            commands.push(crate::simulation::Command::SetGroundWeather(target));
        }
        if let Some((phase, held)) = weather.solar {
            commands.push(crate::simulation::Command::SetWeatherSolarTime { phase, held });
        }
        self.sim
            .step(&commands)
            .map_err(|source| GameWorldError::Simulation {
                operation: "simulation step",
                source,
            })?;
        if let Some(rain) = &mut self.rain {
            rain.step(self.sim.tick());
        }
        if let Some(edits) = &self.weather_edits {
            edits.committed(weather);
        }
        Ok(WorldStep {
            projected_changes: 0,
            skipped_publication: !self.sim.has_current_page(),
        })
    }

    fn recover_publication(&mut self) -> Result<Option<usize>, Self::Error> {
        self.recover_page()
            .map_err(|source| GameWorldError::Simulation {
                operation: "world page",
                source,
            })
    }

    fn publication(&mut self) -> Result<Option<WorldPublication<Self::Page>>, Self::Error> {
        // Recovery belongs to the clock's recovery hook, which also captures
        // derived output. A lease arriving here must not create another page.
        if !self.sim.has_current_page() {
            return Ok(None);
        }
        match self.published_page() {
            Ok(page) => Ok(Some(WorldPublication {
                id: page.id(),
                timeline: page.timeline(),
                page,
            })),
            Err(SimError::PublicationBusy) => Ok(None),
            Err(source) => Err(GameWorldError::Simulation {
                operation: "world page",
                source,
            }),
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
