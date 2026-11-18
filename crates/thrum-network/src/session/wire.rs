use super::{MATCH_EPOCH, SLOTS, Settings, frame};
use crate::{
    envelope::{self, Header, Kind},
    input_batch::{ActionView, ActionWire, MAX_ACTION_BYTES, MAX_ACTIONS, dequantize, quantize},
};

use thrum_core::sim::InputFrame;
use thrum_transport::{Lane, Link, LinkError, RELIABLE_LIMIT};

pub(super) const SET_PREFIX: usize = 10;
pub(super) const INPUT_PREFIX: usize = 16;
pub(super) const PROTOCOL_CLOSE: u32 = 1;
pub(super) const RESPONSE_CLOSE: u32 = 2;
pub(super) const TIMEOUT_CLOSE: u32 = 3;
pub(super) const MAX_SET: usize =
    1 + 8 + SLOTS * (1 + 3 * 2 + 10 + 10 + 1 + MAX_ACTIONS * (6 + MAX_ACTION_BYTES));

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Connection {
    pub(super) id: [u8; 16],
    pub(super) epoch: u64,
}

impl Connection {
    pub(super) fn header(self, kind: Kind, tick: u64, sequence: u64) -> Header {
        Header {
            kind,
            match_id: self.id,
            connection_epoch: self.epoch,
            message_sequence: sequence,
            server_tick: tick,
            acknowledged_sequence: 0,
        }
    }
    pub(super) fn matches(self, header: Header) -> bool {
        header.match_id == self.id
            && header.connection_epoch == self.epoch
            && header.acknowledged_sequence == 0
    }
}

pub(super) fn send(
    link: &Link,
    lane: Lane,
    header: Header,
    payload: &[u8],
    now: u64,
    scratch: &mut [u8],
) -> Result<(), LinkError> {
    let len =
        envelope::encode(header, payload, scratch).expect("bounded and valid session envelope");
    link.send(lane, &scratch[..len], now)
}

pub(super) struct Read<'a>(&'a [u8]);

impl<'a> Read<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }
    pub(super) fn fixed<const N: usize>(&mut self) -> Option<[u8; N]> {
        Some(*self.take(N)?.first_chunk()?)
    }
    pub(super) fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let (data, rest) = self.0.split_at_checked(count)?;
        self.0 = rest;
        Some(data)
    }
    pub(super) fn rest(self) -> &'a [u8] {
        self.0
    }
    pub(super) fn done(self) -> Option<()> {
        self.0.is_empty().then_some(())
    }
    pub(super) fn byte(&mut self) -> Option<u8> {
        Some(self.fixed::<1>()?[0])
    }
    pub(super) fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.fixed()?))
    }
    pub(super) fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.fixed()?))
    }
    fn uint(&mut self) -> Option<u64> {
        let mut result = 0;
        for shift in (0..=63).step_by(7) {
            let byte = self.byte()?;
            if shift == 63 && byte > 1 {
                return None;
            }
            result |= u64::from(byte & 127) << shift;
            if byte < 128 {
                return Some(result);
            }
        }
        None
    }
}

fn uint(out: &mut Vec<u8>, mut value: u64) {
    let mut bytes = value.to_le_bytes();
    while value >= 128 {
        out.push(bytes[0] | 128);
        value >>= 7;
        bytes = value.to_le_bytes();
    }
    out.push(bytes[0]);
}

pub(super) fn write_settings(out: &mut Vec<u8>, settings: Settings) {
    out.extend(settings.tick_us.to_le_bytes());
    out.extend(settings.digest_interval.to_le_bytes());
    out.extend(settings.staging.to_le_bytes());
    out.extend(settings.backlog.to_le_bytes());
    out.extend(settings.behind.to_le_bytes());
    out.extend([settings.catch_up, settings.repeats]);
    out.extend(settings.image_limit.to_le_bytes());
    out.extend(settings.response_ms.to_le_bytes());
    out.extend(settings.join_ms.to_le_bytes());
}

pub(super) fn read_settings(input: &mut Read<'_>) -> Option<Settings> {
    let settings = Settings {
        tick_us: u32::from_le_bytes(input.fixed()?),
        digest_interval: input.u16()?,
        staging: input.u16()?,
        backlog: input.u16()?,
        behind: input.u16()?,
        catch_up: input.byte()?,
        repeats: input.byte()?,
        image_limit: u32::from_le_bytes(input.fixed()?),
        response_ms: u32::from_le_bytes(input.fixed()?),
        join_ms: u32::from_le_bytes(input.fixed()?),
    };
    settings.valid().then_some(settings)
}

pub(super) fn write_axes(out: &mut Vec<u8>, axes: [f32; 3]) {
    for axis in axes {
        out.extend(
            quantize(axis)
                .expect("canonical session axis")
                .to_le_bytes(),
        );
    }
}

pub(super) fn read_axes(input: &mut Read<'_>) -> Option<[f32; 3]> {
    let mut axes = [0.0; 3];
    for axis in &mut axes {
        *axis = dequantize(i16::from_le_bytes(input.fixed()?)).ok()?;
    }
    Some(axes)
}

pub(super) fn action<A>(out: &mut Vec<u8>, value: &A, wire: &impl ActionWire<A>) -> Option<()> {
    let mut bytes = [0; MAX_ACTION_BYTES];
    let (schema, size) = wire.encode(value, &mut bytes).ok()?;
    if size > bytes.len() {
        return None;
    }
    out.extend(schema.to_le_bytes());
    out.extend(u16::try_from(size).ok()?.to_le_bytes());
    out.extend(&bytes[..size]);
    Some(())
}

pub(super) fn read_action<A>(input: &mut Read<'_>, wire: &impl ActionWire<A>) -> Option<A> {
    let schema_id = u32::from_le_bytes(input.fixed()?);
    let size = usize::from(input.u16()?);
    if size > MAX_ACTION_BYTES {
        return None;
    }
    wire.decode(ActionView {
        schema_id,
        payload: input.take(size)?,
    })
    .ok()
}

#[derive(Clone, Copy, Default)]
pub(super) struct SlotResult {
    pub(super) frame: u64,
    pub(super) last_command: u64,
    pub(super) commands: u8,
}

pub(super) struct Set<A> {
    pub(super) frames: [InputFrame<A>; SLOTS],
    pub(super) results: [SlotResult; SLOTS],
    pub(super) joined: u8,
    pub(super) digest: Option<u64>,
}

pub(super) fn write_set<A>(
    output: &mut Vec<u8>,
    set: &Set<A>,
    prior: [[f32; 3]; SLOTS],
    wire: &impl ActionWire<A>,
) -> Option<()> {
    output.clear();
    output.push(set.joined | (u8::from(set.digest.is_some()) << 4));
    if let Some(digest) = set.digest {
        output.extend(digest.to_le_bytes());
    }
    for ((frame, result), old) in set.frames.iter().zip(set.results).zip(prior) {
        let changed = frame.axes.map(f32::to_bits) != old.map(f32::to_bits);
        let count = u8::try_from(frame.actions.len()).ok()?;
        output.push(
            count
                | (u8::from(changed) << 4)
                | (u8::from(result.frame != 0) << 5)
                | (u8::from(result.commands != 0) << 6),
        );
        if changed {
            write_axes(output, frame.axes);
        }
        if result.frame != 0 {
            uint(output, result.frame);
        }
        if result.commands != 0 {
            uint(output, result.last_command);
            output.push(result.commands);
        }
        for value in &frame.actions {
            action(output, value, wire)?;
        }
    }
    Some(())
}

pub(super) fn read_set<A>(
    bytes: &[u8],
    tick: u64,
    prior: [[f32; 3]; SLOTS],
    wire: &impl ActionWire<A>,
) -> Option<Set<A>> {
    let mut input = Read::new(bytes);
    let flags = input.byte()?;
    if flags > 31 {
        return None;
    }
    let digest = if flags & 16 == 0 {
        None
    } else {
        Some(input.u64()?)
    };
    let mut set = Set {
        frames: std::array::from_fn(|slot| frame(slot, MATCH_EPOCH, tick, tick, prior[slot])),
        results: [SlotResult::default(); SLOTS],
        joined: flags & 15,
        digest,
    };
    for (frame, result) in set.frames.iter_mut().zip(&mut set.results) {
        let flags = input.byte()?;
        let count = flags & 15;
        if flags >= 128 || usize::from(count) > MAX_ACTIONS {
            return None;
        }
        if flags & 16 != 0 {
            frame.axes = read_axes(&mut input)?;
        }
        if flags & 32 != 0 {
            result.frame = input.uint()?;
            if result.frame == 0 {
                return None;
            }
        }
        if flags & 64 != 0 {
            result.last_command = input.uint()?;
            result.commands = input.byte()?;
            if result.commands == 0
                || result.commands > count
                || result.last_command < u64::from(result.commands)
            {
                return None;
            }
        }
        for _ in 0..count {
            frame.actions.push(read_action(&mut input, wire)?);
        }
    }
    input.done()?;
    Some(set)
}

pub(super) fn scratch() -> Box<[u8]> {
    vec![0; RELIABLE_LIMIT].into_boxed_slice()
}

#[cfg(test)]
mod tests;
