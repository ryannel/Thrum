use std::{collections::hash_map::DefaultHasher, hash::Hasher, num::NonZeroU32};

use crate::simulation::{
    ComponentEntity, Health, Position, SimConfig, Simulation, Velocity, Visual,
};
use thrum_core::{StableId, WorldEpoch};

#[test]
fn capture_metadata_distinguishes_absent_velocity_from_present_zero() {
    let config = SimConfig {
        tick_hz: NonZeroU32::new(60).expect("nonzero rate"),
        entity_capacity: 1,
        peer_capacity: 4,
        actions_per_frame: 8,
        movement_speed: 6.0,
    };
    let mut absent = Simulation::new(WorldEpoch(301), config).expect("absent world");
    let mut present = Simulation::new(WorldEpoch(301), config).expect("present world");
    let absent_handle = absent
        .seed(ComponentEntity {
            id: StableId(1),
            position: Position::default(),
            velocity: Velocity::default(),
            health: 100,
            ownership: None,
            visual: Some(2),
        })
        .expect("ordinary static seed omits Velocity");
    // Use the existing generic writer to admit the other legal column shape;
    // the V1 seed helper deliberately omits unowned zero-velocity components.
    let present_handle = present
        .runtime
        .seed(StableId(1), |row| {
            row.add(Position::default())?;
            row.add(Health(100))?;
            row.add(Velocity::default())?;
            row.add(Visual(2))?;
            Ok(())
        })
        .expect("explicit present-zero component");
    assert_eq!(
        absent.get(absent_handle).expect("defaulted row"),
        present.get(present_handle).expect("present row")
    );
    let mut absent_hash = DefaultHasher::new();
    absent.capture().hash_authority_metadata(&mut absent_hash);
    let mut present_hash = DefaultHasher::new();
    present.capture().hash_authority_metadata(&mut present_hash);
    assert_ne!(
        absent_hash.finish(),
        present_hash.finish(),
        "presence must survive the read model's zero default"
    );
}

#[test]
fn a_misrouted_permit_refuses_the_whole_tick() {
    use crate::simulation::{Command, SimError};
    use std::hash::Hash;

    let config = SimConfig {
        tick_hz: NonZeroU32::new(60).expect("nonzero rate"),
        entity_capacity: 4,
        peer_capacity: 4,
        actions_per_frame: 8,
        movement_speed: 6.0,
    };
    let mut sim = Simulation::new(WorldEpoch(302), config).expect("world");
    let handles = [1_u128, 2].map(|id| {
        sim.seed(ComponentEntity {
            id: StableId(id),
            position: Position::default(),
            velocity: Velocity([1.0, 0.0, 0.0]),
            health: 100,
            ownership: None,
            visual: Some(2),
        })
        .expect("mover")
    });
    sim.published_page().expect("baseline");
    sim.step(&[]).expect("ordinary tick");
    let state = |sim: &Simulation| {
        let mut hash = DefaultHasher::new();
        sim.capture().hash_authority_metadata(&mut hash);
        for handle in handles {
            let row = sim.get(handle).expect("row");
            row.position.0.map(f64::to_bits).hash(&mut hash);
            row.health.hash(&mut hash);
        }
        hash.finish()
    };
    let before = state(&sim);
    // Resolve appends its health edit with Input health's permit.
    std::mem::swap(
        &mut sim.rules.resolve.health_write,
        &mut sim.rules.input_health.health_write,
    );
    assert_eq!(
        sim.step(&[Command::SetHealth(handles[0], 50)]).err(),
        Some(SimError::InvalidEdit)
    );
    assert_eq!(sim.tick(), 1);
    assert_eq!(
        state(&sim),
        before,
        "a foreign permit leaves authority unchanged"
    );
    std::mem::swap(
        &mut sim.rules.resolve.health_write,
        &mut sim.rules.input_health.health_write,
    );
    sim.step(&[Command::SetHealth(handles[0], 50)])
        .expect("own permit commits");
    assert_eq!(sim.get(handles[0]).expect("row").health, 50);
}
