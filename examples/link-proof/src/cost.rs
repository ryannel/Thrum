//! Whole-message round trips and chunked transfers, through the public message link.
use serde_json::{Value, json};
use std::collections::BTreeMap;
use thrum_transport::{Lane, Link, LinkError, RELIABLE_LIMIT};

pub(super) struct Cost {
    host: bool,
    created: Option<u64>,
    opened: Option<u64>,
    next_datagram: u64,
    next_reliable: u64,
    sequence: u64,
    pending: BTreeMap<u64, (u64, usize, usize)>,
    samples: [[Vec<u64>; 2]; 4],
    stage: usize,
    next_transfer: u64,
    transfer_started: u64,
    transfer_times: Vec<u64>,
    outgoing: Option<(u8, usize, usize)>,
    incoming: [usize; 3],
    echo: Option<Vec<u8>>,
    datagrams_sent: u64,
    datagrams_received: u64,
    next_snapshot: u64,
    history: Vec<Value>,
}
impl Cost {
    pub(super) fn new(host: bool) -> Self {
        Self {
            host,
            created: None,
            opened: None,
            next_datagram: 0,
            next_reliable: 0,
            sequence: 0,
            pending: BTreeMap::new(),
            samples: std::array::from_fn(|_| std::array::from_fn(|_| Vec::new())),
            stage: 0,
            next_transfer: 5_000,
            transfer_started: 0,
            transfer_times: Vec::new(),
            outgoing: None,
            incoming: [0; 3],
            echo: None,
            datagrams_sent: 0,
            datagrams_received: 0,
            next_snapshot: 5_000,
            history: Vec::new(),
        }
    }
    pub(super) fn tick(&mut self, link: &Link, now: u64) -> Result<(), String> {
        self.created.get_or_insert(now);
        if !link.is_open() {
            return Ok(());
        }
        let opened = *self.opened.get_or_insert(now);
        let elapsed = now - opened;
        if elapsed >= self.next_snapshot {
            let counts = link.counters(now);
            let application_bytes: u64 = counts.lanes.iter().map(|c| c.sent_bytes).sum();
            self.history.push(json!({"elapsed_ms":elapsed,"wire_bytes":counts.wire_bytes,"application_bytes":application_bytes,"wire_per_application_byte":counts.wire_bytes.map(|bytes|bytes as f64/application_bytes.max(1) as f64)}));
            self.next_snapshot += 5_000;
        }
        if let Some(bytes) = self.echo.take() {
            match link.send(Lane::Reliable, &bytes, 0) {
                Ok(()) => (),
                Err(LinkError::Full) => {
                    self.echo = Some(bytes);
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        for lane in [Lane::Reliable, Lane::Datagram] {
            if lane == Lane::Reliable && self.echo.is_some() {
                continue;
            }
            while let Some(mut bytes) = link.receive(lane) {
                if bytes.len() < 9 {
                    return Err("short cost message".into());
                }
                match bytes[0] {
                    0 if self.host => {
                        bytes[0] = 1;
                        match link.send(lane, &bytes, 0) {
                            Ok(()) => (),
                            Err(LinkError::Full) => {
                                self.echo = Some(bytes);
                                break;
                            }
                            Err(e) => return Err(e.to_string()),
                        }
                    }
                    1 if !self.host => {
                        let sequence =
                            u64::from_le_bytes(bytes[1..9].try_into().map_err(|_| "sequence")?);
                        let (sent, phase, index) = self
                            .pending
                            .remove(&sequence)
                            .ok_or("duplicate or unknown echo")?;
                        if bytes[9..]
                            .iter()
                            .any(|byte| *byte != sequence.to_le_bytes()[0])
                        {
                            return Err("corrupt echo".into());
                        }
                        self.samples[phase][index].push(now - sent);
                        if lane == Lane::Datagram {
                            self.datagrams_received += 1;
                        }
                    }
                    2 if self.host => {
                        let id = bytes[1];
                        if id > 1 || self.outgoing.is_some() {
                            return Err("invalid transfer request".into());
                        }
                        self.outgoing = Some((id, 0, if id == 0 { 4 } else { 16 }));
                    }
                    3 | 4 => {
                        let id = usize::from(bytes[1]);
                        if id >= 3
                            || bytes.len() != RELIABLE_LIMIT
                            || bytes[9..].iter().any(|v| *v != bytes[2])
                        {
                            return Err("invalid transfer chunk".into());
                        }
                        if usize::from(bytes[2]) != self.incoming[id] {
                            return Err("transfer chunk order".into());
                        }
                        self.incoming[id] += 1;
                        let count = if id == 1 { 16 } else { 4 };
                        if self.incoming[id] == count {
                            if self.host {
                                let mut ack = vec![0; 9];
                                ack[0] = 5;
                                self.echo = Some(ack);
                            } else {
                                self.complete(now, elapsed);
                            }
                        }
                    }
                    5 if !self.host => self.complete(now, elapsed),
                    _ => return Err("unexpected cost message".into()),
                }
            }
        }
        if !self.host && elapsed < 80_000 {
            let phase = if self.transfer_started == 0 {
                0
            } else {
                self.stage + 1
            };
            for (index, lane) in [Lane::Reliable, Lane::Datagram].into_iter().enumerate() {
                let due = if index == 0 {
                    self.next_reliable
                } else {
                    self.next_datagram
                };
                if elapsed >= due {
                    let mut bytes =
                        vec![self.sequence.to_le_bytes()[0]; if index == 0 { 400 } else { 189 }];
                    bytes[0] = 0;
                    bytes[1..9].copy_from_slice(&self.sequence.to_le_bytes());
                    match link.send(lane, &bytes, 0) {
                        Ok(()) => {
                            self.pending
                                .insert(self.sequence, (now, phase.min(3), index));
                            self.sequence += 1;
                            if index == 0 {
                                self.next_reliable += 100;
                            } else {
                                self.datagrams_sent += 1;
                                self.next_datagram = self.datagrams_sent * 1_000 / 60;
                            }
                        }
                        Err(LinkError::Full) => (),
                        Err(e) => return Err(e.to_string()),
                    }
                }
            }
            if self.stage < 3 && self.transfer_started == 0 && elapsed >= self.next_transfer {
                self.transfer_started = now;
                if self.stage == 2 {
                    self.outgoing = Some((2, 0, 4));
                } else {
                    let mut bytes = [0; 9];
                    bytes[0] = 2;
                    bytes[1] = u8::try_from(self.stage).map_err(|_| "stage")?;
                    link.send(Lane::Reliable, &bytes, 0)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        if let Some((id, mut part, count)) = self.outgoing.take() {
            while part < count {
                let part_byte = u8::try_from(part).map_err(|_| "part")?;
                let mut bytes = vec![part_byte; RELIABLE_LIMIT];
                bytes[0] = if self.host { 3 } else { 4 };
                bytes[1] = id;
                bytes[2] = part_byte;
                match link.send(Lane::Reliable, &bytes, 0) {
                    Ok(()) => part += 1,
                    Err(LinkError::Full) => {
                        self.outgoing = Some((id, part, count));
                        break;
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
        }
        Ok(())
    }
    fn complete(&mut self, now: u64, elapsed: u64) {
        self.transfer_times.push(now - self.transfer_started);
        self.transfer_started = 0;
        self.stage += 1;
        self.next_transfer = elapsed + 3_000;
    }
    pub(super) fn record(&self) -> Value {
        let phases: Vec<_> = ["steady", "download_256_kib", "download_1_mib", "upload_256_kib"].into_iter().enumerate().map(|(phase, name)| {
            json!({"phase": name, "reliable_ms": quantiles(&self.samples[phase][0]), "datagram_ms": quantiles(&self.samples[phase][1])})
        }).collect();
        json!({"connect_to_open_ms":self.opened.zip(self.created).map(|(o,c)|o-c),"phases":phases,"transfer_ms":self.transfer_times,"datagrams_sent":self.datagrams_sent,"datagrams_received":self.datagrams_received,"complete":self.stage==3,"wire_history":self.history})
    }
}
fn quantiles(samples: &[u64]) -> Value {
    if samples.is_empty() {
        return Value::Null;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    json!({"count":sorted.len(),"median":sorted[sorted.len()/2],"p95":sorted[(sorted.len()-1)*95/100],"maximum":sorted[sorted.len()-1]})
}
