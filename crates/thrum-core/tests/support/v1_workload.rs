//! Deterministic V1 workload scaffold. It measures fixture mechanics, not engine tick cost.

pub const ENTITY_COUNT: usize = 10_000;
pub const MOVING_COUNT: usize = 1_500;
pub const PEER_COUNT: usize = 4;
const TICKS_PER_SECOND: u32 = 60;
const CHURN_PER_SECOND: u32 = 100;
const PACKET_CAPACITY: usize = 64;

#[derive(Clone, Copy, Default)]
struct Packet {
    peer: usize,
    sent_tick: u64,
    arrival_tick: u64,
}

pub struct LinkConditioner {
    pending: [Packet; PACKET_CAPACITY],
    pending_len: usize,
    random_state: u64,
    pub delivered: [u64; PEER_COUNT],
    pub dropped: u64,
    pub min_one_way_ticks: u64,
    pub max_one_way_ticks: u64,
}

impl LinkConditioner {
    pub fn new(seed: u64) -> Self {
        Self {
            pending: [Packet::default(); PACKET_CAPACITY],
            pending_len: 0,
            random_state: seed.max(1),
            delivered: [0; PEER_COUNT],
            dropped: 0,
            min_one_way_ticks: u64::MAX,
            max_one_way_ticks: 0,
        }
    }

    fn random(&mut self) -> u64 {
        let mut x = self.random_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.random_state = x;
        x
    }

    pub fn step(&mut self, tick: u64) {
        for peer in 0..PEER_COUNT {
            let sample = self.random();
            if sample % 100 < 2 {
                self.dropped += 1;
                continue;
            }
            // One-way 50 ms plus or minus one 60 Hz tick; roughly 100 ms RTT
            // and 20 ms jitter. A1/A5 will replace packet mechanics with real I/O.
            let delay_ticks = 2 + (sample >> 8) % 3;
            assert!(self.pending_len < PACKET_CAPACITY, "conditioner capacity");
            self.pending[self.pending_len] = Packet {
                peer,
                sent_tick: tick,
                arrival_tick: tick + delay_ticks,
            };
            self.pending_len += 1;
        }
        let mut index = 0;
        while index < self.pending_len {
            if self.pending[index].arrival_tick <= tick {
                let packet = self.pending[index];
                self.delivered[packet.peer] += 1;
                let delay = tick - packet.sent_tick;
                self.min_one_way_ticks = self.min_one_way_ticks.min(delay);
                self.max_one_way_ticks = self.max_one_way_ticks.max(delay);
                self.pending_len -= 1;
                self.pending[index] = self.pending[self.pending_len];
            } else {
                index += 1;
            }
        }
    }

    pub fn in_flight(&self) -> usize {
        self.pending_len
    }
}

pub struct Workload {
    position_x: Vec<f32>,
    velocity_x: Vec<f32>,
    generations: Vec<u32>,
    churn_credit: u32,
    churn_cursor: usize,
    pub churned: u64,
    pub tick: u64,
    pub link: LinkConditioner,
}

impl Workload {
    pub fn new() -> Self {
        let mut position_x = Vec::with_capacity(ENTITY_COUNT);
        let mut velocity_x = Vec::with_capacity(ENTITY_COUNT);
        for id in 0..ENTITY_COUNT {
            position_x.push(id as f32);
            velocity_x.push(if id < MOVING_COUNT { 0.25 } else { 0.0 });
        }
        Self {
            position_x,
            velocity_x,
            generations: vec![0; ENTITY_COUNT],
            churn_credit: 0,
            churn_cursor: 0,
            churned: 0,
            tick: 0,
            link: LinkConditioner::new(0xA0A0_2026_1103),
        }
    }

    pub fn step(&mut self) {
        self.tick += 1;
        for id in 0..MOVING_COUNT {
            self.position_x[id] += self.velocity_x[id];
        }
        self.churn_credit += CHURN_PER_SECOND;
        while self.churn_credit >= TICKS_PER_SECOND {
            self.churn_credit -= TICKS_PER_SECOND;
            let id = MOVING_COUNT + self.churn_cursor;
            self.generations[id] = self.generations[id].wrapping_add(1);
            self.position_x[id] = id as f32;
            self.churn_cursor = (self.churn_cursor + 1) % (ENTITY_COUNT - MOVING_COUNT);
            self.churned += 1;
        }
        self.link.step(self.tick);
    }

    pub fn first_position(&self) -> f32 {
        self.position_x[0]
    }

    pub fn entity_count(&self) -> usize {
        self.position_x.len()
    }
}
