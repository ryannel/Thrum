//! Deterministic virtual-time carrier, including startup, cuts and bounded stack stalls.
use crate::{CloseReason, Lane, Link, LinkError, Profile};

/// One-way conditioner. A0 is 50 ± 10 ms and 2% loss, independently per direction.
#[derive(Clone, Copy, Debug)]
pub struct Conditioner {
    /// Base one-way delay in milliseconds.
    pub delay_ms: u64,
    /// Uniform jitter, plus or minus this many milliseconds.
    pub jitter_ms: u64,
    /// Loss probability in millionths.
    pub loss_ppm: u32,
    /// Repeatable random sequence.
    pub seed: u64,
    /// Named fault: duplicate datagrams, outside the link's contract.
    pub duplicate: bool,
}
impl Default for Conditioner {
    fn default() -> Self {
        Self {
            delay_ms: 50,
            jitter_ms: 10,
            loss_ppm: 20_000,
            seed: 0x7419,
            duplicate: false,
        }
    }
}
#[derive(Debug)]
struct Flight {
    due: u64,
    direction: usize,
    lane: Lane,
    bytes: Vec<u8>,
    duplicate: bool,
}

/// Observation of a virtual datagram at the carrier boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatagramEvent {
    /// The carrier discarded the datagram.
    Lost,
    /// The first copy reached the receiving link.
    Delivered,
    /// The additional copy reached the receiving link.
    Duplicate,
}
/// Host-side deterministic link factory and carrier. Time advances only by explicit calls.
#[derive(Debug)]
pub struct Memory {
    profile: Profile,
    conditioner: Conditioner,
    random: u64,
    now: u64,
    pairs: Vec<Pair>,
    cut: bool,
    stalled: bool,
}
#[derive(Debug)]
struct Pair {
    datagram_cap: usize,
    ends: [Link; 2],
    created: u64,
    heard: u64,
    reliable_due: [u64; 2],
    flight: Vec<Flight>,
    stack: [Vec<Vec<u8>>; 2],
}
impl Memory {
    /// Create a carrier with validated bounds. There is no wall clock or thread.
    /// # Errors
    /// Returns `InvalidProfile` for invalid limits or conditioner ranges.
    pub fn new(profile: Profile, conditioner: Conditioner) -> Result<Self, LinkError> {
        profile.validate()?;
        validate(conditioner)?;
        Ok(Self {
            profile,
            conditioner,
            random: conditioner.seed,
            now: 0,
            pairs: Vec::new(),
            cut: false,
            stalled: false,
        })
    }
    /// Change the treatment of newly handed-on packets, preserving existing flight and random state.
    ///
    /// # Errors
    /// Returns `InvalidProfile` for invalid conditioner ranges.
    pub fn recondition(&mut self, mut conditioner: Conditioner) -> Result<(), LinkError> {
        validate(conditioner)?;
        conditioner.seed = self.conditioner.seed;
        self.conditioner = conditioner;
        Ok(())
    }
    /// Start a client/host pair. Returns immediately in the connecting state.
    /// # Errors
    /// Returns `Full` at the host's cap, counting startups too.
    /// # Panics
    /// Panics if link state was poisoned by an earlier panic.
    pub fn connect(&mut self) -> Result<[Link; 2], LinkError> {
        self.pairs.retain(|p| {
            p.ends.iter().any(|e| {
                e.state
                    .lock()
                    .expect("link state poisoned")
                    .closed
                    .is_none()
            })
        });
        if self.pairs.len() >= self.profile.links {
            return Err(LinkError::Full);
        }
        let ends = [
            Link::new(self.profile.clone()),
            Link::new(self.profile.clone()),
        ];
        self.pairs.push(Pair {
            datagram_cap: self.profile.datagram_bytes,
            ends: ends.clone(),
            created: self.now,
            heard: self.now,
            reliable_due: [self.now; 2],
            flight: Vec::new(),
            stack: std::array::from_fn(|_| Vec::new()),
        });
        Ok(ends)
    }
    /// Synthetic identity for this virtual host. It is a test value, never a TLS identity.
    pub fn ticket(&self) -> crate::Ticket {
        crate::Ticket {
            address: "127.0.0.1:0".into(),
            hash: std::array::from_fn(|i| self.conditioner.seed.to_le_bytes()[i % 8]),
        }
    }
    /// Exercise the same exact-ticket trust rule without a socket or cryptography.
    /// # Errors
    /// Returns `InvalidTicket` on identity mismatch, otherwise the host's capacity error.
    /// # Panics
    /// Panics if link state was poisoned by an earlier panic.
    pub fn connect_ticket(&mut self, ticket: &crate::Ticket) -> Result<[Link; 2], LinkError> {
        if ticket != &self.ticket() {
            return Err(LinkError::InvalidTicket);
        }
        self.connect()
    }
    /// Cut all paths. Keep-alives cannot cross, including on links without game traffic.
    pub fn cut(&mut self, cut: bool) {
        self.cut = cut;
    }
    /// Stop stack progress but retain its allowance. Used to rehearse late datagrams.
    pub fn stall(&mut self, stalled: bool) {
        self.stalled = stalled;
    }
    /// Advance to caller monotonic time; repeated calls at the same time are deterministic.
    /// # Panics
    /// Panics if time goes backwards.
    pub fn advance(&mut self, now: u64) {
        self.advance_observed(now, |_, _, _| {});
    }

    /// Advance and observe datagram loss and delivery with original bytes.
    /// The direction is zero from the first link returned by `connect`.
    ///
    /// # Panics
    /// Panics if time goes backwards or an earlier panic poisoned a link.
    pub fn advance_observed(
        &mut self,
        now: u64,
        mut observe: impl FnMut(usize, DatagramEvent, &[u8]),
    ) {
        assert!(now >= self.now, "virtual time must be monotonic");
        self.now = now;
        for pair in &mut self.pairs {
            for direction in 0..2 {
                let closed = pair.ends[direction]
                    .state
                    .lock()
                    .expect("link state poisoned")
                    .closed
                    .clone();
                if let Some(reason) = closed {
                    let peer = if let CloseReason::Local(code) = reason {
                        CloseReason::Peer(code)
                    } else {
                        reason
                    };
                    pair.ends[1 - direction]
                        .state
                        .lock()
                        .expect("link state poisoned")
                        .close(peer);
                    pair.flight.clear();
                    for stack in &mut pair.stack {
                        stack.clear();
                    }
                }
            }
            let started = pair.ends[0].is_open();
            if !self.cut && !self.stalled {
                pair.heard = now;
                if now.saturating_sub(pair.created)
                    >= 2 * (self.conditioner.delay_ms + self.conditioner.jitter_ms)
                {
                    for end in &pair.ends {
                        end.state.lock().expect("link state poisoned").opened();
                    }
                }
            } else {
                let deadline = if started {
                    self.profile.idle_ms
                } else {
                    self.profile.startup_ms
                };
                if now.saturating_sub(pair.heard) >= deadline {
                    for end in &pair.ends {
                        end.state
                            .lock()
                            .expect("link state poisoned")
                            .close(CloseReason::TimedOut);
                    }
                    pair.flight.clear();
                }
            }
            for direction in 0..2 {
                let end = pair.ends[direction].clone();
                let mut state = end.state.lock().expect("link state poisoned");
                state.expire(now);
                if !state.open {
                    continue;
                }
                if !self.stalled && !self.cut {
                    for bytes in std::mem::take(&mut pair.stack[direction]) {
                        schedule(
                            &mut self.random,
                            self.conditioner,
                            pair,
                            direction,
                            Lane::Datagram,
                            bytes,
                            now,
                            &mut observe,
                        );
                    }
                    while pair
                        .flight
                        .iter()
                        .filter(|f| f.direction == direction && f.lane == Lane::Reliable)
                        .map(|f| f.bytes.len())
                        .sum::<usize>()
                        + state.outbound[0].front().map_or(0, |m| m.bytes.len())
                        <= self.profile.reliable_bytes
                    {
                        let Some(bytes) = state.pop(Lane::Reliable) else {
                            break;
                        };
                        schedule(
                            &mut self.random,
                            self.conditioner,
                            pair,
                            direction,
                            Lane::Reliable,
                            bytes,
                            now,
                            &mut observe,
                        );
                    }
                }
                loop {
                    let used: usize = pair.stack[direction].iter().map(Vec::len).sum();
                    let Some(front) = state.outbound[1].front() else {
                        break;
                    };
                    if (self.stalled || self.cut)
                        && used + front.bytes.len() > self.profile.stack_bytes
                    {
                        break;
                    }
                    let bytes = state.pop(Lane::Datagram).expect("front exists");
                    if self.stalled {
                        pair.stack[direction].push(bytes);
                    } else if !self.cut {
                        schedule(
                            &mut self.random,
                            self.conditioner,
                            pair,
                            direction,
                            Lane::Datagram,
                            bytes,
                            now,
                            &mut observe,
                        );
                    } else {
                        pair.stack[direction].push(bytes);
                    }
                }
                state.counts.stack_bytes = Some(pair.stack[direction].iter().map(Vec::len).sum());
            }
            if !self.cut && !self.stalled {
                pair.flight.sort_by_key(|f| f.due);
                let mut index = 0;
                let mut blocked = [false; 2];
                while index < pair.flight.len() {
                    if pair.flight[index].due > now {
                        break;
                    }
                    let flight = &pair.flight[index];
                    if flight.lane == Lane::Reliable && blocked[flight.direction] {
                        index += 1;
                        continue;
                    }
                    let accepted = pair.ends[1 - flight.direction]
                        .state
                        .lock()
                        .expect("link state poisoned")
                        .receive(flight.lane, flight.bytes.clone());
                    if accepted || flight.lane == Lane::Datagram {
                        if accepted && flight.lane == Lane::Datagram {
                            let event = if flight.duplicate {
                                DatagramEvent::Duplicate
                            } else {
                                DatagramEvent::Delivered
                            };
                            observe(flight.direction, event, &flight.bytes);
                        }
                        pair.flight.remove(index);
                    } else {
                        blocked[flight.direction] = true;
                        index += 1;
                    }
                }
            }
        }
    }
}
fn validate(conditioner: Conditioner) -> Result<(), LinkError> {
    if conditioner.loss_ppm > 1_000_000
        || conditioner.jitter_ms > conditioner.delay_ms
        || conditioner.seed == 0
        || conditioner.delay_ms > 120_000
    {
        return Err(LinkError::InvalidProfile);
    }
    Ok(())
}

fn random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}
fn schedule(
    rng: &mut u64,
    condition: Conditioner,
    pair: &mut Pair,
    direction: usize,
    lane: Lane,
    bytes: Vec<u8>,
    now: u64,
    observe: &mut impl FnMut(usize, DatagramEvent, &[u8]),
) {
    // The virtual network is finite too. A datagram that cannot enter it is network loss,
    // separate from expiry or overflow in the sender's link queue.
    if lane == Lane::Datagram
        && pair
            .flight
            .iter()
            .filter(|f| f.lane == Lane::Datagram && f.direction == direction)
            .map(|f| f.bytes.len())
            .sum::<usize>()
            + bytes.len()
            > pair.datagram_cap
    {
        observe(direction, DatagramEvent::Lost, &bytes);
        return;
    }
    let lost = random(rng) % 1_000_000 < u64::from(condition.loss_ppm);
    if lost && lane == Lane::Datagram {
        observe(direction, DatagramEvent::Lost, &bytes);
        return;
    }
    let delay =
        condition.delay_ms - condition.jitter_ms + random(rng) % (2 * condition.jitter_ms + 1);
    let mut due = now + delay;
    if lane == Lane::Reliable {
        due += if lost { condition.delay_ms * 2 } else { 0 };
        due = due.max(pair.reliable_due[direction]);
        pair.reliable_due[direction] = due;
    }
    let duplicate = lane == Lane::Datagram && condition.duplicate;
    let duplicate_bytes = duplicate.then(|| bytes.clone());
    pair.flight.push(Flight {
        due,
        direction,
        lane,
        bytes,
        duplicate: false,
    });
    if let Some(bytes) = duplicate_bytes {
        pair.flight.push(Flight {
            due,
            direction,
            lane,
            bytes,
            duplicate: true,
        });
    }
}
