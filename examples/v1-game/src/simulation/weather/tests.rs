use crate::simulation::{
    Command, ComponentEntity, Position, SimConfig, SimError, Simulation, Velocity,
};
use std::num::NonZeroU32;
use thrum_core::{StableId, WorldEpoch};

#[test]
fn weather_is_committed_published_restored_and_protected_with_the_world() {
    let config = SimConfig {
        tick_hz: NonZeroU32::new(60).expect("tick rate"),
        entity_capacity: 3,
        peer_capacity: 0,
        actions_per_frame: 8,
        movement_speed: 6.0,
    };
    let mut sim = Simulation::new_clouded(WorldEpoch(99), config).expect("weather schema");
    for i in 0..2 {
        sim.seed(ComponentEntity {
            id: StableId(i),
            position: Position([0.0; 3]),
            velocity: Velocity([0.0; 3]),
            health: 100,
            ownership: None,
            visual: Some(0),
        })
        .expect("game entity");
    }
    sim.seed_weather(Some(super::Weather::new(
        crate::clouds::initial_state(),
        None,
    )))
    .expect("weather authority");
    let original = sim.published_page().expect("tick zero page");
    let zero = original.weather().expect("weather in page");
    assert_eq!(zero.tick(), 0);
    let capture = sim.capture();
    sim.step(&[]).expect("world tick including weather");
    let page = sim.published_page().expect("tick one page");
    let one = page.weather().expect("weather endpoint");
    assert_eq!(one.tick(), page.id().tick);
    assert!((one.time() - 1.0 / 60.0).abs() < 1e-12);
    assert!((one.travel()[0] - 10.0 / 60.0).abs() < 1e-12);
    assert_eq!(zero.tick(), 0, "an old page retains its own weather");
    let handle = sim.handle(StableId(u128::MAX)).expect("weather entity");
    assert_eq!(
        sim.step(&[Command::Despawn(handle)]),
        Err(SimError::InvalidOwnership)
    );
    assert_eq!(sim.tick(), 1, "a refused operation changes no authority");
    sim.restore(&capture).expect("restore whole authority");
    assert_eq!(
        sim.published_page()
            .expect("restored page")
            .weather()
            .expect("restored weather")
            .as_ref(),
        zero.as_ref()
    );
    sim.step(&[]).expect("replay");
    assert_eq!(
        sim.published_page()
            .expect("replayed page")
            .weather()
            .expect("replayed weather")
            .as_ref(),
        one.as_ref()
    );
    assert_eq!(page.weather().expect("old endpoint").tick(), 1);
}

#[test]
fn disabled_weather_adds_no_entity_or_publication_component() {
    let mut sim = crate::scene::selected_simulation(
        crate::GameContent::Grid,
        crate::RouteOverlay::Off,
        false,
    )
    .expect("legacy game");
    let page = sim.published_page().expect("legacy page");
    assert_eq!(page.len(), crate::scene::ENTITY_COUNT);
    assert!(page.weather().is_none());
}

#[test]
#[cfg(feature = "proving")]
fn clearing_deck_does_not_reintroduce_cumulus() {
    use crate::clouds::{WeatherPreset, WeatherSequence};

    let parameters = WeatherPreset::Overcast.parameters();
    let mut state = thrum_cloud_model::State::new(parameters, 5.0, 0).expect("initial deck");
    let step = 1.0 / 60.0;
    for tick in 1_u32..=65 * 60 {
        let seconds = 180.0 + f64::from(tick) * step;
        state
            .set_target(WeatherSequence::Presets.target(parameters, seconds))
            .expect("sequence target");
        state.step(u64::from(tick), step).expect("weather tick");
        let weather = state.parameters();
        if weather.coverage > 0.0 {
            assert!(weather.kind == 0.0, "keep the layered type as it clears");
        }
        if seconds >= 240.0 {
            assert!(weather.coverage == 0.0, "the sky is clear at four minutes");
        }
    }
    assert!(state.parameters().kind > 0.0);
}

#[test]
fn live_target_is_a_tick_edit_that_can_be_replayed() {
    let config = SimConfig {
        tick_hz: NonZeroU32::new(60).expect("tick rate"),
        entity_capacity: 1,
        peer_capacity: 0,
        actions_per_frame: 8,
        movement_speed: 6.0,
    };
    let mut sim = Simulation::new_clouded(WorldEpoch(99), config).expect("weather schema");
    sim.seed_weather(Some(super::Weather::new(
        crate::clouds::initial_state(),
        None,
    )))
    .expect("weather authority");
    let old = sim
        .published_page()
        .expect("valid weather edit test")
        .weather()
        .expect("valid weather edit test");
    let snapshot = sim.capture();
    let target = thrum_cloud_model::Parameters {
        coverage: 1.0,
        ..old.parameters()
    };
    let bad = Command::SetWeather(thrum_cloud_model::Parameters {
        coverage: f64::NAN,
        ..target
    });
    assert_eq!(sim.step(&[bad]).err(), Some(SimError::InvalidConfig));
    assert_eq!(sim.tick(), 0);
    let edit = Command::SetWeather(target);
    sim.step(&[edit]).expect("valid weather edit test");
    let next = sim
        .published_page()
        .expect("valid weather edit test")
        .weather()
        .expect("valid weather edit test");
    assert!(next.parameters().coverage > old.parameters().coverage);
    assert!(next.parameters().coverage < target.coverage);
    assert_eq!(old.tick(), 0);
    sim.restore(&snapshot).expect("valid weather edit test");
    sim.step(&[edit]).expect("valid weather edit test");
    assert_eq!(
        sim.published_page()
            .expect("valid weather edit test")
            .weather()
            .expect("valid weather edit test"),
        next
    );
}

#[test]
fn cooling_uses_the_committed_held_or_running_sun() {
    use crate::clouds::{CloudSelection, GroundWeather, ground};
    use thrum_cloud_model::{Density, Field, State};

    let preset = GroundWeather::DawnMist;
    let selection = CloudSelection::Ground(preset);
    let ground = ground::valley(preset).expect("valley limits");
    let profile = selection.profile(None);
    let field = Field::new(0, profile.model, profile.types).expect("cloud field");
    let [x, z] = [0.0, -1200.0];
    let [height, ..] = ground.surface.sample([x, z]);
    let point = [x, height + 1.0, z];
    let query = |page: &crate::simulation::PublishedPage| {
        ground.fog.density(
            &ground.surface,
            &field,
            point,
            &page.weather().expect("weather"),
            Density::Coarse,
        )
    };
    for (phase, held) in [(0.5, true), (16.5 / 24.0, false)] {
        let config = SimConfig {
            tick_hz: NonZeroU32::new(60).expect("tick rate"),
            entity_capacity: 1,
            peer_capacity: 0,
            actions_per_frame: 8,
            movement_speed: 6.0,
        };
        let mut sim = Simulation::new_clouded(WorldEpoch(99), config).expect("weather schema");
        let mut parameters = selection.default_weather();
        parameters.time_scale = 60.0;
        let mut state = State::new(parameters, 5.0, 0).expect("dawn mist state");
        state.set_ground(preset.targets().state().expect("ground targets"));
        sim.seed_weather(Some(
            super::Weather::new(state, None).with_ground(std::sync::Arc::clone(&ground.fog)),
        ))
        .expect("weather authority");
        sim.step(&[Command::SetWeatherSolarTime { phase, held }])
            .expect("route solar start");
        let first = sim.published_page().expect("origin publication");
        assert_eq!(
            first.weather_solar().unwrap().sample(1.0).to_bits(),
            phase.to_bits()
        );
        let mut dawn_seen = false;
        let mut morning_clear = false;
        for _ in 0..620 {
            sim.step(&[]).expect("weather tick");
            let page = sim.published_page().expect("weather publication");
            let solar = page.weather_solar().expect("committed Sun");
            let hour = solar.sample(1.0) * 24.0;
            let amount = page.weather().unwrap().ground().amount(0, true);
            if held {
                assert_eq!(hour.to_bits(), 12.0_f32.to_bits());
                assert_eq!(amount, 0.0);
                assert_eq!(query(&page), 0.0);
            } else if (4.0..6.0).contains(&hour) {
                dawn_seen |= amount > 0.5 && query(&page) > 0.0;
            } else if (9.0..10.0).contains(&hour) {
                morning_clear |= amount == 0.0 && query(&page) == 0.0;
            }
        }
        if !held {
            assert!(dawn_seen, "the headless witness must condense by dawn");
            assert!(morning_clear, "the headless witness must clear after dawn");
        }
    }
}
