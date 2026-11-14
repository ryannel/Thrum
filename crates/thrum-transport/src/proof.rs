//! Shared link-script primitives. Both virtual and target runners drive this same state machine.
use crate::{Lane, Link, LinkError};
use std::collections::BTreeSet;

/// Content-checked exchange report, independent of adapter counters.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Reliable messages sent by this endpoint.
    pub reliable_sent: u64,
    /// Reliable messages received from its peer, in order.
    pub reliable_received: u64,
    /// Datagram messages sent by this endpoint.
    pub datagrams_sent: u64,
    /// Unique datagrams received.
    pub datagrams_received: u64,
    /// Datagrams received behind a newer one.
    pub reordered: u64,
}
/// Stateful, content-checking script endpoint. No adapter, clock or sleep is hidden here.
#[derive(Debug)]
pub struct Script {
    direction: u8,
    report: Report,
    datagrams: BTreeSet<u64>,
    newest: u64,
    reliable_position: u64,
    started: Option<u64>,
    next_datagram: u64,
    next_reliable: u64,
    bulk: usize,
    stop_ms: u64,
    identity: u64,
    peer_identity: Option<u64>,
}
impl Script {
    /// Direction is 0 for the client and 1 for the host; distinct patterns detect cross-link leaks.
    pub fn new(direction: u8) -> Self {
        Self {
            direction,
            report: Report::default(),
            datagrams: BTreeSet::new(),
            newest: 0,
            reliable_position: 0,
            started: None,
            next_datagram: 1_000,
            next_reliable: 1_000,
            bulk: 0,
            stop_ms: 20_000,
            identity: 0,
            peer_identity: None,
        }
    }
    /// Set when the harness will stop accepting new sends and begin draining.
    pub fn stop_at(&mut self, milliseconds: u64) {
        self.stop_ms = milliseconds;
    }
    /// Give this script endpoint a harness-unique identity, carried in patterned payloads.
    pub fn identify(&mut self, identity: u64) {
        self.identity = identity;
    }
    /// This endpoint's payload identity.
    pub const fn identity(&self) -> u64 {
        self.identity
    }
    /// Identity bound from this link's first full-size peer payload.
    pub const fn peer_identity(&self) -> Option<u64> {
        self.peer_identity
    }
    /// Exercise immediate refusals, a full reliable queue, and oldest-first datagram overflow.
    /// The harness drains afterward and compares independent tallies at both endpoints.
    /// # Errors
    /// Returns `Protocol` for a missing refusal or incorrect capacity accounting.
    pub fn limits(&mut self, link: &Link, now: u64) -> Result<(), crate::CloseReason> {
        for lane in [Lane::Reliable, Lane::Datagram] {
            if link.send(lane, &[], now) != Err(LinkError::Empty)
                || link.send(lane, &vec![0; lane.limit() + 1], now) != Err(LinkError::TooLarge)
            {
                return Err(crate::CloseReason::Protocol);
            }
        }
        let mut full = false;
        for _ in 0..1_000 {
            match self.send(link, Lane::Reliable, crate::RELIABLE_LIMIT, now) {
                Ok(()) => (),
                Err(LinkError::Full) => {
                    full = true;
                    break;
                }
                Err(_) => return Err(crate::CloseReason::Protocol),
            }
        }
        if !full {
            return Err(crate::CloseReason::Protocol);
        }
        for _ in 0..1_000 {
            self.send(link, Lane::Datagram, crate::DATAGRAM_LIMIT, now)
                .map_err(|_| crate::CloseReason::Protocol)?;
        }
        if link.counters(now).dropped_cap == 0 {
            return Err(crate::CloseReason::Protocol);
        }
        Ok(())
    }
    /// Saturate the datagram queue during a harness-controlled stall. Expiry is counted
    /// by the timer after the harness stops calling this, while the stall is still held.
    /// # Errors
    /// Returns an unexpected send refusal.
    pub fn expiry_burst(&mut self, link: &Link, now: u64) -> Result<(), LinkError> {
        for _ in 0..32 {
            self.send(link, Lane::Datagram, crate::DATAGRAM_LIMIT, now)?;
        }
        Ok(())
    }
    /// Drive the common lanes/load exchange: host speaks first, 60 Hz datagrams, 10 Hz
    /// reliable messages, both lane extremes and 256 KiB/1 MiB bursts. Stop sends at 20 s.
    /// Calls after 20 s drain only; the harness compares both endpoints after quiescence.
    /// # Errors
    /// Returns a framing, ordering or duplicate failure, or an unexpected send refusal.
    pub fn tick(&mut self, link: &Link, now: u64) -> Result<(), crate::CloseReason> {
        if !link.is_open() {
            return Ok(());
        }
        let start = *self.started.get_or_insert(now);
        let elapsed = now.saturating_sub(start);
        self.drain(link)?;
        if self.direction == 1 && self.report.reliable_sent == 0 {
            self.send(link, Lane::Reliable, 1, now)
                .map_err(|_| crate::CloseReason::Protocol)?;
        }
        if elapsed < self.stop_ms {
            if elapsed >= self.next_datagram {
                let size = if self.report.datagrams_sent == 0 {
                    1
                } else if self.report.datagrams_sent == 1 {
                    crate::DATAGRAM_LIMIT
                } else {
                    189
                };
                self.send(link, Lane::Datagram, size, now)
                    .map_err(|_| crate::CloseReason::Protocol)?;
                self.next_datagram = 1_000 + self.report.datagrams_sent * 1_000 / 60;
            }
            if elapsed >= self.next_reliable {
                match self.send(
                    link,
                    Lane::Reliable,
                    if self.direction == 0 && self.report.reliable_sent == 0 {
                        1
                    } else {
                        400
                    },
                    now,
                ) {
                    Ok(()) => self.next_reliable += 100,
                    Err(LinkError::Full) => (),
                    Err(_) => return Err(crate::CloseReason::Protocol),
                }
            }
            let wanted = if elapsed >= 10_000 {
                20
            } else if elapsed >= 3_000 {
                4
            } else {
                0
            };
            while self.bulk < wanted {
                match self.send(link, Lane::Reliable, crate::RELIABLE_LIMIT, now) {
                    Ok(()) => self.bulk += 1,
                    Err(LinkError::Full) => break,
                    Err(_) => return Err(crate::CloseReason::Protocol),
                }
            }
        }
        Ok(())
    }
    /// Send a patterned message at any lane size, preserving position on `Full`.
    /// # Errors
    /// Returns the link's immediate refusal.
    pub fn send(
        &mut self,
        link: &Link,
        lane: Lane,
        size: usize,
        now: u64,
    ) -> Result<(), LinkError> {
        let position = if lane == Lane::Reliable {
            self.report.reliable_sent
        } else {
            self.report.datagrams_sent
        };
        let bytes = pattern(self.direction, position, size, self.identity);
        link.send(lane, &bytes, now)?;
        if lane == Lane::Reliable {
            self.report.reliable_sent += 1;
        } else {
            self.report.datagrams_sent += 1;
        }
        Ok(())
    }
    /// Check every arrived message's size, contents and sequence. Duplicate faults fail here.
    /// # Errors
    /// Returns `Protocol` if contents, reliable order or datagram uniqueness differ.
    /// # Panics
    /// Panics if an internal I/O panic poisoned the link state.
    pub fn drain(&mut self, link: &Link) -> Result<(), crate::CloseReason> {
        for lane in [Lane::Reliable, Lane::Datagram] {
            while let Some(bytes) = link.receive(lane) {
                let position = if bytes.len() < 9 {
                    if lane == Lane::Datagram {
                        u64::MAX
                    } else {
                        self.reliable_position
                    }
                } else {
                    u64::from_le_bytes(bytes[1..9].try_into().expect("eight bytes"))
                };
                let identity = if bytes.len() >= 17 {
                    u64::from_le_bytes(bytes[9..17].try_into().expect("eight bytes"))
                } else {
                    self.peer_identity.unwrap_or(0)
                };
                if bytes.len() >= 17 {
                    if self
                        .peer_identity
                        .is_some_and(|previous| previous != identity)
                    {
                        return Err(crate::CloseReason::Protocol);
                    }
                    self.peer_identity = Some(identity);
                }
                if bytes != pattern(1 - self.direction, position, bytes.len(), identity) {
                    return Err(crate::CloseReason::Protocol);
                }
                if lane == Lane::Reliable {
                    if position != self.reliable_position {
                        return Err(crate::CloseReason::Protocol);
                    }
                    self.reliable_position += 1;
                    self.report.reliable_received += 1;
                } else {
                    if !self.datagrams.insert(position) {
                        return Err(crate::CloseReason::Protocol);
                    }
                    if bytes.len() >= 9 {
                        if position < self.newest {
                            self.report.reordered += 1;
                        }
                        self.newest = self.newest.max(position);
                    }
                    self.report.datagrams_received += 1;
                }
            }
        }
        Ok(())
    }
    /// Independent script tally, to compare with the public counters.
    pub const fn report(&self) -> &Report {
        &self.report
    }
    /// Pattern direction, used by lifecycle drivers to select which end closes.
    pub const fn direction(&self) -> u8 {
        self.direction
    }
}
fn pattern(direction: u8, position: u64, size: usize, identity: u64) -> Vec<u8> {
    let mut bytes = vec![direction; size];
    if size >= 9 {
        bytes[1..9].copy_from_slice(&position.to_le_bytes());
    }
    for (index, byte) in bytes.iter_mut().enumerate().skip(9) {
        *byte = index.to_le_bytes()[0]
            .wrapping_add(position.to_le_bytes()[0])
            .wrapping_add(direction);
    }
    if size >= 17 {
        bytes[9..17].copy_from_slice(&identity.to_le_bytes());
    }
    bytes
}
