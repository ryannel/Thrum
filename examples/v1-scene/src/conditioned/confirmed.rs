//! Ordered, delayed in-memory controller groups for the first confirmed client
//! consumer. This is not the V1 state-delta wire or result/ACK retention owner.

use thrum_core::{Origin, sim::ControllerState};

use super::PEER_COUNT;

const RETURN_CAPACITY: usize = 32;
const RETURN_SEED: u64 = 0xC0F1_2026_1104;

#[derive(Clone, Copy)]
struct Group {
    tick: u64,
    state: ControllerState,
    committed: Option<Origin>,
}

#[derive(Clone, Copy)]
struct Pending {
    peer_index: usize,
    arrival: u64,
    group: Group,
}

#[derive(Clone, Copy)]
pub(super) struct ConfirmedPeer {
    tick: u64,
    state: ControllerState,
    committed_count: u64,
    last_sequence: Option<u64>,
}

impl ConfirmedPeer {
    fn admit(&mut self, group: Group) -> Result<(), String> {
        if self.tick.checked_add(1) != Some(group.tick) || self.state.binding != group.state.binding
        {
            return Err("confirmed controller group tick or binding mismatch".to_owned());
        }
        if group
            .state
            .position
            .0
            .iter()
            .chain(group.state.velocity.0.iter())
            .any(|value| !value.is_finite())
            || group
                .state
                .held_axes
                .iter()
                .any(|axis| !axis.is_finite() || axis.abs() > 1.0)
        {
            return Err("confirmed controller group has invalid numeric values".to_owned());
        }
        if let Some(origin) = group.committed {
            if origin.peer != self.state.binding.peer
                || origin.connection_epoch != self.state.binding.connection_epoch
                || self
                    .last_sequence
                    .is_some_and(|sequence| origin.sequence <= sequence)
            {
                return Err("confirmed input origin is stale or misbound".to_owned());
            }
            self.last_sequence = Some(origin.sequence);
            self.committed_count += 1;
        }
        self.tick = group.tick;
        self.state = group.state;
        Ok(())
    }

    pub(super) fn tick(&self) -> u64 {
        self.tick
    }

    pub(super) fn position(&self) -> [f64; 3] {
        self.state.position.0
    }

    pub(super) fn committed_count(&self) -> u64 {
        self.committed_count
    }
}

pub(super) struct ConfirmedLoop {
    clients: [ConfirmedPeer; PEER_COUNT],
    pending: Vec<Pending>,
    last_arrival: [u64; PEER_COUNT],
    random_state: u64,
    sent: u64,
    delivered: u64,
    max_age: u64,
}

impl ConfirmedLoop {
    pub(super) fn new(states: &[ControllerState; PEER_COUNT]) -> Self {
        Self {
            clients: std::array::from_fn(|index| ConfirmedPeer {
                tick: 0,
                state: states[index],
                committed_count: 0,
                last_sequence: None,
            }),
            pending: Vec::with_capacity(RETURN_CAPACITY),
            last_arrival: [0; PEER_COUNT],
            random_state: RETURN_SEED,
            sent: 0,
            delivered: 0,
            max_age: 0,
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

    pub(super) fn send(
        &mut self,
        peer_index: usize,
        tick: u64,
        state: ControllerState,
        committed: Option<Origin>,
    ) -> Result<(), String> {
        if peer_index >= PEER_COUNT || self.pending.len() == RETURN_CAPACITY {
            return Err("confirmed return lane capacity or peer refusal".to_owned());
        }
        let delay = 2 + self.random() % 3;
        let arrival = tick
            .checked_add(delay)
            .ok_or("return arrival tick overflow")?
            .max(
                self.last_arrival[peer_index]
                    .checked_add(1)
                    .ok_or("return ordering tick overflow")?,
            );
        self.last_arrival[peer_index] = arrival;
        self.pending.push(Pending {
            peer_index,
            arrival,
            group: Group {
                tick,
                state,
                committed,
            },
        });
        self.sent += 1;
        Ok(())
    }

    pub(super) fn deliver(&mut self, now: u64) -> Result<(), String> {
        let mut index = 0;
        while index < self.pending.len() {
            if self.pending[index].arrival <= now {
                let pending = self.pending.swap_remove(index);
                self.clients[pending.peer_index].admit(pending.group)?;
                self.delivered += 1;
                self.max_age = self.max_age.max(now - pending.group.tick);
            } else {
                index += 1;
            }
        }
        Ok(())
    }

    pub(super) fn clients(&self) -> &[ConfirmedPeer; PEER_COUNT] {
        &self.clients
    }

    pub(super) fn counts(&self) -> (u64, u64, usize, u64) {
        (self.sent, self.delivered, self.pending.len(), self.max_age)
    }

    pub(super) fn in_flight_committed(&self) -> u64 {
        u64::try_from(
            self.pending
                .iter()
                .filter(|pending| pending.group.committed.is_some())
                .count(),
        )
        .expect("bounded return capacity fits u64")
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfirmedLoop, Group, PEER_COUNT};
    use crate::scene;
    use thrum_core::{Origin, PeerId, sim::ControllerState};

    fn states() -> [ControllerState; PEER_COUNT] {
        let sim = scene::simulation().expect("V1 world");
        std::array::from_fn(|index| {
            sim.controller_state(PeerId(u64::try_from(index + 1).expect("peer index")))
                .expect("initial controller")
        })
    }

    #[test]
    fn refused_groups_leave_the_confirmed_client_unchanged() {
        let states = states();
        let mut loopback = ConfirmedLoop::new(&states);
        let client = &mut loopback.clients[0];
        let initial = *client;
        let mut group = Group {
            tick: 1,
            state: states[0],
            committed: None,
        };
        group.tick = 2;
        assert!(client.admit(group).is_err());
        assert_eq!(client.tick(), initial.tick());
        group.tick = 1;
        group.state.position.0[0] = f64::NAN;
        assert!(client.admit(group).is_err());
        assert_eq!(client.tick(), initial.tick());
        group.state = states[0];
        group.state.held_axes[0] = 2.0;
        assert!(client.admit(group).is_err());
        assert_eq!(client.tick(), initial.tick());
        group.state = states[1];
        assert!(client.admit(group).is_err());
        assert_eq!(client.tick(), initial.tick());
        group.state = states[0];
        group.committed = Some(Origin {
            peer: states[1].binding.peer,
            connection_epoch: states[0].binding.connection_epoch,
            sequence: 1,
        });
        assert!(client.admit(group).is_err());
        assert_eq!(client.tick(), initial.tick());
        assert_eq!(client.committed_count(), initial.committed_count());
    }

    #[test]
    fn committed_origin_cannot_repeat_or_go_backwards() {
        let states = states();
        let mut loopback = ConfirmedLoop::new(&states);
        let client = &mut loopback.clients[0];
        let origin = Origin {
            peer: states[0].binding.peer,
            connection_epoch: states[0].binding.connection_epoch,
            sequence: 7,
        };
        client
            .admit(Group {
                tick: 1,
                state: states[0],
                committed: Some(origin),
            })
            .expect("first result");
        assert!(
            client
                .admit(Group {
                    tick: 2,
                    state: states[0],
                    committed: Some(origin),
                })
                .is_err()
        );
        assert_eq!(client.tick(), 1);
        assert_eq!(client.committed_count(), 1);
    }
}
