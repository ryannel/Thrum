//! Equal fixture operations for a private representation comparison.
use bevy_ecs::prelude::Component;

pub const COUNT: usize = 10_000;
pub const MOVING: usize = 1_500;
const CHURN_PER_SECOND: usize = 100;
const HZ: usize = 60;

#[derive(Clone, Copy, Default, Component)]
pub struct Position(pub f32);
#[derive(Clone, Copy, Default, Component)]
pub struct Velocity(pub f32);
#[derive(Clone, Copy, Default, Component)]
pub struct Health(pub u16);
#[derive(Clone, Copy, Default, Component)]
pub struct Stable(pub usize);
#[derive(Clone, Copy, Default, Component)]
pub struct Visual(pub u64);
#[derive(Clone, Copy, Default, Component)]
pub struct Ownership(pub u8);

#[derive(Clone, Copy, Default)]
pub struct Row {
    pub position: Position,
    pub velocity: Velocity,
    pub health: Health,
    pub visual: Visual,
    pub ownership: Option<Ownership>,
    pub generation: u32,
}

impl Row {
    fn initial(id: usize) -> Self {
        Self {
            position: Position(id as f32),
            velocity: Velocity(if id < MOVING { 0.25 } else { 0.0 }),
            health: Health(100),
            visual: Visual(2),
            ownership: (id % 8 == 0).then_some(Ownership(1)),
            generation: 0,
        }
    }
    fn respawn(id: usize, generation: u32) -> Self {
        let mut row = Self::initial(id);
        row.ownership = (generation % 2 == 0).then_some(Ownership(1));
        row.generation = generation;
        row
    }
}

pub trait Adapter {
    fn new() -> Self;
    fn tick(&mut self);
    fn copy_snapshot(&mut self, output: &mut [Row]);
    fn restore(&mut self, input: &[Row]);
    fn query_checksum(&mut self) -> f64;
    fn changed_count(&mut self) -> usize;
    fn sparse_change_probe(&mut self) -> usize;
    fn toggle_sparse_ownership(&mut self);
    fn reset_progress(&mut self, tick: u64);
}

fn progress(tick: u64) -> (usize, usize) {
    let accrued = tick as usize * CHURN_PER_SECOND;
    (accrued % HZ, (accrued / HZ) % (COUNT - MOVING))
}

pub struct Custom {
    positions: Vec<Position>,
    velocities: Vec<Velocity>,
    health: Vec<Health>,
    visuals: Vec<Visual>,
    ownership: Vec<Option<Ownership>>,
    generations: Vec<u32>,
    changed: Vec<u64>,
    tick: u64,
    churn_credit: usize,
    churn_cursor: usize,
}

impl Adapter for Custom {
    fn new() -> Self {
        let rows: Vec<_> = (0..COUNT).map(Row::initial).collect();
        Self {
            positions: rows.iter().map(|r| r.position).collect(),
            velocities: rows.iter().map(|r| r.velocity).collect(),
            health: rows.iter().map(|r| r.health).collect(),
            visuals: rows.iter().map(|r| r.visual).collect(),
            ownership: rows.iter().map(|r| r.ownership).collect(),
            generations: vec![0; COUNT],
            changed: vec![0; COUNT],
            tick: 0,
            churn_credit: 0,
            churn_cursor: 0,
        }
    }
    fn tick(&mut self) {
        self.tick += 1;
        for id in 0..MOVING {
            self.positions[id].0 += self.velocities[id].0;
            self.changed[id] = self.tick;
        }
        self.churn_credit += CHURN_PER_SECOND;
        while self.churn_credit >= HZ {
            self.churn_credit -= HZ;
            let id = MOVING + self.churn_cursor;
            let row = Row::respawn(id, self.generations[id] + 1);
            self.positions[id] = row.position;
            self.velocities[id] = row.velocity;
            self.health[id] = row.health;
            self.visuals[id] = row.visual;
            self.ownership[id] = row.ownership;
            self.generations[id] = row.generation;
            self.changed[id] = self.tick;
            self.churn_cursor = (self.churn_cursor + 1) % (COUNT - MOVING);
        }
    }
    fn copy_snapshot(&mut self, output: &mut [Row]) {
        for (id, row) in output.iter_mut().enumerate() {
            *row = Row {
                position: self.positions[id],
                velocity: self.velocities[id],
                health: self.health[id],
                visual: self.visuals[id],
                ownership: self.ownership[id],
                generation: self.generations[id],
            };
        }
    }
    fn restore(&mut self, input: &[Row]) {
        for (id, row) in input.iter().enumerate() {
            self.positions[id] = row.position;
            self.velocities[id] = row.velocity;
            self.health[id] = row.health;
            self.visuals[id] = row.visual;
            self.ownership[id] = row.ownership;
            self.generations[id] = row.generation;
        }
    }
    fn query_checksum(&mut self) -> f64 {
        self.positions
            .iter()
            .zip(&self.velocities)
            .map(|(p, v)| (p.0 * v.0) as f64)
            .sum()
    }
    fn changed_count(&mut self) -> usize {
        self.changed.iter().filter(|&&t| t == self.tick).count()
    }
    fn sparse_change_probe(&mut self) -> usize {
        self.tick += 1;
        for id in 0..100 {
            self.positions[id].0 += 0.25;
            self.changed[id] = self.tick;
        }
        self.changed_count()
    }
    fn toggle_sparse_ownership(&mut self) {
        for id in 0..100 {
            self.ownership[id] = self.ownership[id].is_none().then_some(Ownership(1));
        }
    }
    fn reset_progress(&mut self, tick: u64) {
        self.tick = tick;
        (self.churn_credit, self.churn_cursor) = progress(tick);
        self.changed.fill(0);
    }
}

pub struct Hecs {
    world: hecs::World,
    move_query: hecs::PreparedQuery<&'static mut Position>,
    entities: Vec<hecs::Entity>,
    generations: Vec<u32>,
    changed: Vec<u64>,
    tick: u64,
    churn_credit: usize,
    churn_cursor: usize,
}

impl Hecs {
    fn spawn(&mut self, id: usize, row: Row) -> hecs::Entity {
        if let Some(owner) = row.ownership {
            self.world.spawn((
                Stable(id),
                row.position,
                row.velocity,
                row.health,
                row.visual,
                owner,
            ))
        } else {
            self.world.spawn((
                Stable(id),
                row.position,
                row.velocity,
                row.health,
                row.visual,
            ))
        }
    }
}

impl Adapter for Hecs {
    fn new() -> Self {
        let mut result = Self {
            world: hecs::World::new(),
            move_query: hecs::PreparedQuery::new(),
            entities: Vec::with_capacity(COUNT),
            generations: vec![0; COUNT],
            changed: vec![0; COUNT],
            tick: 0,
            churn_credit: 0,
            churn_cursor: 0,
        };
        for id in 0..COUNT {
            let entity = result.spawn(id, Row::initial(id));
            result.entities.push(entity);
        }
        result
    }
    fn tick(&mut self) {
        self.tick += 1;
        {
            let mut view = self.move_query.view_mut(&mut self.world);
            for id in 0..MOVING {
                view.get_mut(self.entities[id]).unwrap().0 += 0.25;
                self.changed[id] = self.tick;
            }
        }
        self.churn_credit += CHURN_PER_SECOND;
        while self.churn_credit >= HZ {
            self.churn_credit -= HZ;
            let id = MOVING + self.churn_cursor;
            self.world.despawn(self.entities[id]).unwrap();
            self.generations[id] += 1;
            self.entities[id] = self.spawn(id, Row::respawn(id, self.generations[id]));
            self.changed[id] = self.tick;
            self.churn_cursor = (self.churn_cursor + 1) % (COUNT - MOVING);
        }
    }
    fn copy_snapshot(&mut self, output: &mut [Row]) {
        for (stable, pos, vel, health, visual, ownership) in self
            .world
            .query::<(
                &Stable,
                &Position,
                &Velocity,
                &Health,
                &Visual,
                Option<&Ownership>,
            )>()
            .iter()
        {
            let id = stable.0;
            output[id] = Row {
                position: *pos,
                velocity: *vel,
                health: *health,
                visual: *visual,
                ownership: ownership.copied(),
                generation: self.generations[id],
            };
        }
    }
    fn restore(&mut self, input: &[Row]) {
        self.world.clear();
        for (id, &row) in input.iter().enumerate() {
            self.entities[id] = self.spawn(id, row);
            self.generations[id] = row.generation;
        }
    }
    fn query_checksum(&mut self) -> f64 {
        self.world
            .query::<(&Position, &Velocity)>()
            .iter()
            .map(|(p, v)| (p.0 * v.0) as f64)
            .sum()
    }
    fn changed_count(&mut self) -> usize {
        self.changed.iter().filter(|&&t| t == self.tick).count()
    }
    fn sparse_change_probe(&mut self) -> usize {
        self.tick += 1;
        {
            let mut view = self.move_query.view_mut(&mut self.world);
            for id in 0..100 {
                view.get_mut(self.entities[id]).unwrap().0 += 0.25;
                self.changed[id] = self.tick;
            }
        }
        self.changed_count()
    }
    fn toggle_sparse_ownership(&mut self) {
        for id in 0..100 {
            let entity = self.entities[id];
            if self.world.get::<&Ownership>(entity).is_ok() {
                self.world.remove_one::<Ownership>(entity).unwrap();
            } else {
                self.world.insert_one(entity, Ownership(1)).unwrap();
            }
        }
    }
    fn reset_progress(&mut self, tick: u64) {
        self.tick = tick;
        (self.churn_credit, self.churn_cursor) = progress(tick);
        self.changed.fill(0);
    }
}

pub struct Bevy {
    world: bevy_ecs::world::World,
    snapshot_query: Option<
        bevy_ecs::query::QueryState<(
            &'static Stable,
            &'static Position,
            &'static Velocity,
            &'static Health,
            &'static Visual,
            Option<&'static Ownership>,
        )>,
    >,
    sum_query: Option<bevy_ecs::query::QueryState<(&'static Position, &'static Velocity)>>,
    move_query: Option<bevy_ecs::query::QueryState<&'static mut Position>>,
    changed_query:
        Option<bevy_ecs::query::QueryState<&'static Position, bevy_ecs::query::Changed<Position>>>,
    entities: Vec<bevy_ecs::entity::Entity>,
    generations: Vec<u32>,
    changed: Vec<u64>,
    tick: u64,
    churn_credit: usize,
    churn_cursor: usize,
}

impl Bevy {
    fn spawn(&mut self, id: usize, row: Row) -> bevy_ecs::entity::Entity {
        if let Some(owner) = row.ownership {
            self.world
                .spawn((
                    Stable(id),
                    row.position,
                    row.velocity,
                    row.health,
                    row.visual,
                    owner,
                ))
                .id()
        } else {
            self.world
                .spawn((
                    Stable(id),
                    row.position,
                    row.velocity,
                    row.health,
                    row.visual,
                ))
                .id()
        }
    }
}

impl Adapter for Bevy {
    fn new() -> Self {
        let mut result = Self {
            world: bevy_ecs::world::World::new(),
            snapshot_query: None,
            sum_query: None,
            move_query: None,
            changed_query: None,
            entities: Vec::with_capacity(COUNT),
            generations: vec![0; COUNT],
            changed: vec![0; COUNT],
            tick: 0,
            churn_credit: 0,
            churn_cursor: 0,
        };
        for id in 0..COUNT {
            let entity = result.spawn(id, Row::initial(id));
            result.entities.push(entity);
        }
        result.snapshot_query = Some(result.world.query::<(
            &Stable,
            &Position,
            &Velocity,
            &Health,
            &Visual,
            Option<&Ownership>,
        )>());
        result.sum_query = Some(result.world.query::<(&Position, &Velocity)>());
        result.move_query = Some(result.world.query::<&mut Position>());
        result.changed_query = Some(
            result
                .world
                .query_filtered::<&Position, bevy_ecs::query::Changed<Position>>(),
        );
        result
    }
    fn tick(&mut self) {
        // Standalone Bevy worlds need the same per-frame tracker maintenance
        // that the Bevy app runner normally performs.
        self.world.clear_trackers();
        self.tick += 1;
        self.world.increment_change_tick();
        {
            let mut query = self.move_query.as_mut().unwrap().query_mut(&mut self.world);
            for id in 0..MOVING {
                query.get_mut(self.entities[id]).unwrap().0 += 0.25;
                self.changed[id] = self.tick;
            }
        }
        self.churn_credit += CHURN_PER_SECOND;
        while self.churn_credit >= HZ {
            self.churn_credit -= HZ;
            let id = MOVING + self.churn_cursor;
            assert!(self.world.despawn(self.entities[id]));
            self.generations[id] += 1;
            self.entities[id] = self.spawn(id, Row::respawn(id, self.generations[id]));
            self.changed[id] = self.tick;
            self.churn_cursor = (self.churn_cursor + 1) % (COUNT - MOVING);
        }
    }
    fn copy_snapshot(&mut self, output: &mut [Row]) {
        let query = self.snapshot_query.as_mut().unwrap();
        for (stable, pos, vel, health, visual, ownership) in query.iter(&self.world) {
            let id = stable.0;
            output[id] = Row {
                position: *pos,
                velocity: *vel,
                health: *health,
                visual: *visual,
                ownership: ownership.copied(),
                generation: self.generations[id],
            };
        }
    }
    fn restore(&mut self, input: &[Row]) {
        // Reconstruct the private world; rollback must not preserve private handles.
        self.world = bevy_ecs::world::World::new();
        for (id, &row) in input.iter().enumerate() {
            self.entities[id] = self.spawn(id, row);
            self.generations[id] = row.generation;
        }
        self.snapshot_query = Some(self.world.query::<(
            &Stable,
            &Position,
            &Velocity,
            &Health,
            &Visual,
            Option<&Ownership>,
        )>());
        self.sum_query = Some(self.world.query::<(&Position, &Velocity)>());
        self.move_query = Some(self.world.query::<&mut Position>());
        self.changed_query = Some(
            self.world
                .query_filtered::<&Position, bevy_ecs::query::Changed<Position>>(),
        );
    }
    fn query_checksum(&mut self) -> f64 {
        self.sum_query
            .as_mut()
            .unwrap()
            .iter(&self.world)
            .map(|(p, v)| (p.0 * v.0) as f64)
            .sum()
    }
    fn changed_count(&mut self) -> usize {
        self.changed.iter().filter(|&&t| t == self.tick).count()
    }
    fn sparse_change_probe(&mut self) -> usize {
        self.world.clear_trackers();
        self.tick += 1;
        self.world.increment_change_tick();
        {
            let mut query = self.move_query.as_mut().unwrap().query_mut(&mut self.world);
            for id in 0..100 {
                query.get_mut(self.entities[id]).unwrap().0 += 0.25;
                self.changed[id] = self.tick;
            }
        }
        self.changed_query
            .as_mut()
            .unwrap()
            .iter(&self.world)
            .count()
    }
    fn toggle_sparse_ownership(&mut self) {
        self.world.clear_trackers();
        for id in 0..100 {
            let entity = self.entities[id];
            if self.world.get::<Ownership>(entity).is_some() {
                self.world.entity_mut(entity).remove::<Ownership>();
            } else {
                self.world.entity_mut(entity).insert(Ownership(1));
            }
        }
    }
    fn reset_progress(&mut self, tick: u64) {
        self.tick = tick;
        (self.churn_credit, self.churn_cursor) = progress(tick);
        self.changed.fill(0);
    }
}

pub fn verify<A: Adapter>() {
    let mut candidate = A::new();
    let mut reference = Custom::new();
    let mut snapshot = vec![Row::default(); COUNT];
    for tick in 0..600 {
        candidate.tick();
        reference.tick();
        assert_eq!(
            candidate.changed_count(),
            reference.changed_count(),
            "changed count at tick {tick}"
        );
        if tick % 60 == 0 {
            candidate.copy_snapshot(&mut snapshot);
            let mut expected = vec![Row::default(); COUNT];
            reference.copy_snapshot(&mut expected);
            for id in 0..COUNT {
                assert_eq!(
                    snapshot[id].position.0, expected[id].position.0,
                    "position at tick {tick}, ID {id}"
                );
                assert_eq!(
                    snapshot[id].generation, expected[id].generation,
                    "generation at tick {tick}, ID {id}"
                );
                assert_eq!(
                    snapshot[id].ownership.map(|o| o.0),
                    expected[id].ownership.map(|o| o.0),
                    "ownership at tick {tick}, ID {id}"
                );
            }
            let sum = candidate.query_checksum();
            assert_eq!(sum, reference.query_checksum(), "query at tick {tick}");
            candidate.restore(&snapshot);
            assert_eq!(candidate.query_checksum(), sum, "restore at tick {tick}");
            candidate.reset_progress(tick as u64 + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom() {
        verify::<Custom>();
    }
    #[test]
    fn hecs() {
        verify::<Hecs>();
    }
    #[test]
    fn bevy() {
        verify::<Bevy>();
    }
    fn rollback<A: Adapter>() {
        let mut a = A::new();
        let mut ring = vec![vec![Row::default(); COUNT]; 16];
        for tick in 0..16 {
            a.tick();
            a.copy_snapshot(&mut ring[tick]);
        }
        let expected = ring[15].clone();
        a.restore(&ring[7]);
        a.reset_progress(8);
        for _ in 8..16 {
            a.tick();
        }
        let mut replay = vec![Row::default(); COUNT];
        a.copy_snapshot(&mut replay);
        for id in 0..COUNT {
            assert_eq!(
                replay[id].position.0, expected[id].position.0,
                "replay position {id}"
            );
            assert_eq!(
                replay[id].generation, expected[id].generation,
                "replay generation {id}"
            );
        }
    }
    #[test]
    fn sixteen_tick_rollback() {
        rollback::<Custom>();
        rollback::<Hecs>();
        rollback::<Bevy>();
    }
    #[test]
    fn one_percent_change_detection() {
        assert_eq!(Custom::new().sparse_change_probe(), 100);
        assert_eq!(Hecs::new().sparse_change_probe(), 100);
        assert_eq!(Bevy::new().sparse_change_probe(), 100);
    }
    #[test]
    fn sparse_component_transitions() {
        fn check<A: Adapter>() {
            let mut a = A::new();
            let mut oracle = Custom::new();
            let mut actual = vec![Row::default(); COUNT];
            let mut expected = vec![Row::default(); COUNT];
            for _ in 0..10 {
                a.toggle_sparse_ownership();
                oracle.toggle_sparse_ownership();
                a.copy_snapshot(&mut actual);
                oracle.copy_snapshot(&mut expected);
                for id in 0..100 {
                    assert_eq!(
                        actual[id].ownership.map(|v| v.0),
                        expected[id].ownership.map(|v| v.0)
                    );
                }
            }
        }
        check::<Custom>();
        check::<Hecs>();
        check::<Bevy>();
    }

    #[test]
    fn hecs_disjoint_world_queries_run_in_parallel() {
        let world = hecs::World::new();
        let mut a = Hecs::new();
        let world = std::mem::replace(&mut a.world, world);
        std::thread::scope(|scope| {
            let shared = &world;
            scope.spawn(move || {
                for position in shared.query::<&mut Position>().iter() {
                    position.0 += 1.0;
                }
            });
            scope.spawn(move || {
                for health in shared.query::<&mut Health>().iter() {
                    health.0 += 1;
                }
            });
        });
        let mut seen = 0;
        for (stable, position, health) in world.query::<(&Stable, &Position, &Health)>().iter() {
            assert_eq!(position.0, stable.0 as f32 + 1.0);
            assert_eq!(health.0, 101);
            seen += 1;
        }
        assert_eq!(seen, COUNT);
    }
}
