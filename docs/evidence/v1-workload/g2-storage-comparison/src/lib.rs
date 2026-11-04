//! G2 representation comparison. This lives outside the shipped workspace.
//! Both candidates run the same 60 Hz motion, static edits and stable-ID churn.

const HZ: usize = 60;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position(pub [f64; 3]);
#[derive(Clone, Copy, Debug)]
pub struct Velocity(pub [f64; 3]);
#[derive(Clone, Copy, Debug)]
struct Stable(usize);
#[derive(Clone, Copy, Debug)]
struct Health(u16);
#[derive(Clone, Copy, Debug)]
struct Visual(u64);
#[derive(Clone, Copy, Debug)]
struct Moving;
#[derive(Clone, Copy, Debug)]
struct Static;

pub trait Storage {
    fn new(count: usize) -> Self;
    fn tick(&mut self);
    fn checksum(&mut self) -> (f64, usize);
    fn changed_count(&self) -> usize;
    fn count(&self) -> usize;
    fn position_of(&self, id: usize) -> Position;
}

fn moving_count(count: usize) -> usize {
    count * 15 / 100
}

fn initial_position(id: usize) -> Position {
    Position([id as f64, (id % 97) as f64, (id % 31) as f64])
}

fn churn_rate(count: usize) -> usize {
    count / 100 // 100/s at 10k, 1k/s at 100k.
}

/// Two dense archetypes. Slot-to-row mapping is repaired on swap-remove.
pub struct Soa {
    mover_ids: Vec<usize>,
    mover_positions: Vec<Position>,
    mover_velocities: Vec<Velocity>,
    mover_health: Vec<Health>,
    mover_visuals: Vec<Visual>,
    static_ids: Vec<usize>,
    static_positions: Vec<Position>,
    static_health: Vec<Health>,
    static_visuals: Vec<Visual>,
    static_rows: Vec<usize>,
    generations: Vec<u32>,
    changed: Vec<u64>,
    tick: u64,
    churn_credit: usize,
    churn_cursor: usize,
}

impl Storage for Soa {
    fn new(count: usize) -> Self {
        let moving = moving_count(count);
        let mut static_rows = vec![usize::MAX; count];
        let static_ids: Vec<_> = (moving..count).collect();
        for (row, &id) in static_ids.iter().enumerate() {
            static_rows[id] = row;
        }
        Self {
            mover_ids: (0..moving).collect(),
            mover_positions: (0..moving).map(initial_position).collect(),
            mover_velocities: vec![Velocity([0.25, 0.5, -0.125]); moving],
            mover_health: vec![Health(100); moving],
            mover_visuals: vec![Visual(2); moving],
            static_positions: (moving..count).map(initial_position).collect(),
            static_health: vec![Health(100); count - moving],
            static_visuals: vec![Visual(2); count - moving],
            static_ids,
            static_rows,
            generations: vec![0; count],
            changed: vec![0; count],
            tick: 0,
            churn_credit: 0,
            churn_cursor: 0,
        }
    }

    fn tick(&mut self) {
        self.tick += 1;
        for ((position, velocity), &id) in self
            .mover_positions
            .iter_mut()
            .zip(&self.mover_velocities)
            .zip(&self.mover_ids)
        {
            for axis in 0..3 {
                position.0[axis] += velocity.0[axis];
            }
            self.changed[id] = self.tick;
        }
        // A 1% static change is a separate, sparse system in the same tick.
        for id in moving_count(self.count())..moving_count(self.count()) + self.count() / 100 {
            let row = self.static_rows[id];
            self.static_positions[row].0[1] += 0.125;
            self.changed[id] = self.tick;
        }
        self.churn_credit += churn_rate(self.count());
        while self.churn_credit >= HZ {
            self.churn_credit -= HZ;
            let id = moving_count(self.count()) + self.churn_cursor;
            let row = self.static_rows[id];
            self.static_positions.swap_remove(row);
            self.static_health.swap_remove(row);
            self.static_visuals.swap_remove(row);
            self.static_ids.swap_remove(row);
            if row < self.static_ids.len() {
                self.static_rows[self.static_ids[row]] = row;
            }
            self.generations[id] += 1;
            self.static_rows[id] = self.static_ids.len();
            self.static_ids.push(id);
            self.static_positions.push(initial_position(id));
            self.static_health.push(Health(100));
            self.static_visuals.push(Visual(2));
            self.changed[id] = self.tick;
            self.churn_cursor =
                (self.churn_cursor + 1) % (self.count() - moving_count(self.count()));
        }
    }

    fn checksum(&mut self) -> (f64, usize) {
        let sum = self
            .mover_positions
            .iter()
            .chain(&self.static_positions)
            .map(|position| position.0.iter().sum::<f64>())
            .sum();
        let data_sum: usize = self
            .mover_health
            .iter()
            .chain(&self.static_health)
            .map(|health| usize::from(health.0))
            .sum::<usize>()
            + self
                .mover_visuals
                .iter()
                .chain(&self.static_visuals)
                .map(|visual| visual.0 as usize)
                .sum::<usize>();
        (
            sum,
            data_sum
                + self
                    .generations
                    .iter()
                    .map(|&generation| generation as usize)
                    .sum::<usize>(),
        )
    }

    fn changed_count(&self) -> usize {
        self.changed
            .iter()
            .filter(|&&tick| tick == self.tick)
            .count()
    }

    fn count(&self) -> usize {
        self.generations.len()
    }

    fn position_of(&self, id: usize) -> Position {
        if id < self.mover_ids.len() {
            self.mover_positions[id]
        } else {
            self.static_positions[self.static_rows[id]]
        }
    }
}

/// Hecs with storage-order prepared iteration for the mover system. Its private
/// entity index exists only for stable-ID churn and sparse static edits.
pub struct Hecs {
    world: hecs::World,
    motion: hecs::PreparedQuery<(&'static mut Position, &'static Velocity, &'static Stable)>,
    entities: Vec<hecs::Entity>,
    generations: Vec<u32>,
    changed: Vec<u64>,
    tick: u64,
    churn_credit: usize,
    churn_cursor: usize,
}

impl Storage for Hecs {
    fn new(count: usize) -> Self {
        let moving = moving_count(count);
        let mut world = hecs::World::new();
        let entities = (0..count)
            .map(|id| {
                if id < moving {
                    world.spawn((
                        Moving,
                        Stable(id),
                        initial_position(id),
                        Velocity([0.25, 0.5, -0.125]),
                        Health(100),
                        Visual(2),
                    ))
                } else {
                    world.spawn((
                        Static,
                        Stable(id),
                        initial_position(id),
                        Health(100),
                        Visual(2),
                    ))
                }
            })
            .collect();
        Self {
            world,
            motion: hecs::PreparedQuery::new(),
            entities,
            generations: vec![0; count],
            changed: vec![0; count],
            tick: 0,
            churn_credit: 0,
            churn_cursor: 0,
        }
    }

    fn tick(&mut self) {
        self.tick += 1;
        for (position, velocity, stable) in self.motion.query_mut(&mut self.world) {
            for axis in 0..3 {
                position.0[axis] += velocity.0[axis];
            }
            self.changed[stable.0] = self.tick;
        }
        for id in moving_count(self.count())..moving_count(self.count()) + self.count() / 100 {
            let mut position = self
                .world
                .get::<&mut Position>(self.entities[id])
                .expect("live static");
            position.0[1] += 0.125;
            self.changed[id] = self.tick;
        }
        self.churn_credit += churn_rate(self.count());
        while self.churn_credit >= HZ {
            self.churn_credit -= HZ;
            let id = moving_count(self.count()) + self.churn_cursor;
            self.world.despawn(self.entities[id]).expect("live static");
            self.generations[id] += 1;
            self.entities[id] = self.world.spawn((
                Static,
                Stable(id),
                initial_position(id),
                Health(100),
                Visual(2),
            ));
            self.changed[id] = self.tick;
            self.churn_cursor =
                (self.churn_cursor + 1) % (self.count() - moving_count(self.count()));
        }
    }

    fn checksum(&mut self) -> (f64, usize) {
        let sum = self
            .world
            .query::<&Position>()
            .iter()
            .map(|position| position.0.iter().sum::<f64>())
            .sum();
        let data_sum: usize = self
            .world
            .query::<(&Health, &Visual)>()
            .iter()
            .map(|(health, visual)| usize::from(health.0) + visual.0 as usize)
            .sum();
        (
            sum,
            data_sum
                + self
                    .generations
                    .iter()
                    .map(|&generation| generation as usize)
                    .sum::<usize>(),
        )
    }

    fn changed_count(&self) -> usize {
        self.changed
            .iter()
            .filter(|&&tick| tick == self.tick)
            .count()
    }

    fn count(&self) -> usize {
        self.generations.len()
    }

    fn position_of(&self, id: usize) -> Position {
        *self
            .world
            .get::<&Position>(self.entities[id])
            .expect("live entity")
    }
}

#[cfg(test)]
mod tests {
    use super::{Hecs, Soa, Storage};

    #[test]
    fn equal_result_with_two_archetypes_sparse_edits_and_churn() {
        for count in [10_000, 100_000] {
            let mut hecs = Hecs::new(count);
            let mut soa = Soa::new(count);
            for _ in 0..600 {
                hecs.tick();
                soa.tick();
                assert_eq!(hecs.changed_count(), soa.changed_count());
            }
            assert_eq!(hecs.checksum(), soa.checksum());
            for id in 0..count {
                assert_eq!(hecs.position_of(id), soa.position_of(id), "id {id}");
            }
        }
    }
}
