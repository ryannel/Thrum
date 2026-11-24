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
