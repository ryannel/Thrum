//! Private physical baseline-frame storage oracle. This does not implement
//! the unified N1 sequence/ACK owner or authorize BaselineAck supersession.

use thrum_network::envelope::{self, Header, Kind};
use thrum_network::retirement::PeerKey;

const CHUNK_BYTES: usize = 32 * 1024;
const IMAGE_BYTES: usize = 256 * 1024;
const CHUNK_FRAME_BYTES: usize = envelope::HEADER_BYTES + 12 + CHUNK_BYTES;
const BEGIN_FRAME_BYTES: usize = envelope::HEADER_BYTES + 120;
const END_FRAME_BYTES: usize = envelope::HEADER_BYTES + 8;
const STORAGE_BYTES: usize = 2 * CHUNK_FRAME_BYTES + BEGIN_FRAME_BYTES + END_FRAME_BYTES;
const BEGIN_OFFSET: usize = 2 * CHUNK_FRAME_BYTES;
const END_OFFSET: usize = BEGIN_OFFSET + BEGIN_FRAME_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Invalid,
    Sequence,
    Credit,
    Premature,
    Stale,
    Poisoned,
}

#[derive(Clone, Copy)]
struct Frame {
    sequence: u64,
    len: usize,
    written: usize,
}

#[derive(Clone, Copy)]
struct Chunk {
    frame: Frame,
    slot: Option<usize>,
    staged: bool,
}

struct PendingFrame<'a> {
    sequence: u64,
    accepted: usize,
    bytes: &'a [u8],
}

struct PhysicalBaseline {
    key: PeerKey,
    baseline_id: u64,
    image_len: usize,
    chunk_count: usize,
    image_sha256: [u8; 32],
    bytes: Box<[u8]>,
    begin: Option<Frame>,
    chunks: [Option<Chunk>; 8],
    end: Option<Frame>,
    slot_owner: [Option<usize>; 2],
    next_chunk: usize,
    last_sequence: u64,
    retained_bytes: usize,
    poisoned: bool,
}

impl PhysicalBaseline {
    fn new(
        key: PeerKey,
        baseline_id: u64,
        previous_sequence: u64,
        image_len: usize,
        image_sha256: [u8; 32],
    ) -> Result<Self, Refusal> {
        if key.match_id == [0; 16]
            || key.connection_epoch == 0
            || baseline_id == 0
            || image_len == 0
            || image_len > IMAGE_BYTES
        {
            return Err(Refusal::Invalid);
        }
        Ok(Self {
            key,
            baseline_id,
            image_len,
            chunk_count: image_len.div_ceil(CHUNK_BYTES),
            image_sha256,
            bytes: vec![0; STORAGE_BYTES].into_boxed_slice(),
            begin: None,
            chunks: [None; 8],
            end: None,
            slot_owner: [None; 2],
            next_chunk: 0,
            last_sequence: previous_sequence,
            retained_bytes: 0,
            poisoned: false,
        })
    }

    fn header(&self, kind: Kind, sequence: u64, tick: u64) -> Header {
        Header {
            kind,
            match_id: self.key.match_id,
            connection_epoch: self.key.connection_epoch,
            message_sequence: sequence,
            server_tick: tick,
            acknowledged_sequence: 0,
        }
    }

    fn require_next(&self, sequence: u64) -> Result<(), Refusal> {
        if self.poisoned {
            return Err(Refusal::Poisoned);
        }
        if self.last_sequence.checked_add(1) != Some(sequence) {
            return Err(Refusal::Sequence);
        }
        Ok(())
    }

    fn begin(&mut self, sequence: u64, capture_tick: u64) -> Result<(), Refusal> {
        self.require_next(sequence)?;
        if self.begin.is_some() {
            return Err(Refusal::Invalid);
        }
        let mut payload = [0u8; 120];
        payload[..8].copy_from_slice(&self.baseline_id.to_le_bytes());
        payload[8..16].copy_from_slice(&capture_tick.to_le_bytes());
        payload[16..20].copy_from_slice(
            &(u32::try_from(self.image_len).expect("bounded test value")).to_le_bytes(),
        );
        payload[20..22].copy_from_slice(
            &(u16::try_from(self.chunk_count).expect("bounded test value")).to_le_bytes(),
        );
        payload[24..56].fill(1); // Fixed test protocol hash.
        payload[56..88].fill(2); // Fixed test semantic hash.
        payload[88..120].copy_from_slice(&self.image_sha256);
        let len = envelope::encode(
            self.header(Kind::BaselineBegin, sequence, capture_tick),
            &payload,
            &mut self.bytes[BEGIN_OFFSET..BEGIN_OFFSET + BEGIN_FRAME_BYTES],
        )
        .expect("fixed Begin frame fits");
        self.begin = Some(Frame {
            sequence,
            len,
            written: 0,
        });
        self.retained_bytes += len;
        self.last_sequence = sequence;
        Ok(())
    }

    fn submit_chunk(&mut self, sequence: u64, index: usize, data: &[u8]) -> Result<(), Refusal> {
        self.require_next(sequence)?;
        if self.begin.is_none() || index != self.next_chunk || index >= self.chunk_count {
            return Err(Refusal::Invalid);
        }
        let expected = (self.image_len - index * CHUNK_BYTES).min(CHUNK_BYTES);
        if data.len() != expected {
            return Err(Refusal::Invalid);
        }
        let slot = self
            .slot_owner
            .iter()
            .position(Option::is_none)
            .ok_or(Refusal::Credit)?;
        let mut payload = vec![0u8; 12 + CHUNK_BYTES];
        payload[..8].copy_from_slice(&self.baseline_id.to_le_bytes());
        payload[8..12]
            .copy_from_slice(&(u32::try_from(index).expect("bounded test value")).to_le_bytes());
        payload[12..12 + data.len()].copy_from_slice(data);
        let start = slot * CHUNK_FRAME_BYTES;
        let len = envelope::encode(
            self.header(Kind::BaselineChunk, sequence, 0),
            &payload[..12 + data.len()],
            &mut self.bytes[start..start + CHUNK_FRAME_BYTES],
        )
        .expect("bounded chunk frame fits");
        self.chunks[index] = Some(Chunk {
            frame: Frame {
                sequence,
                len,
                written: 0,
            },
            slot: Some(slot),
            staged: false,
        });
        self.slot_owner[slot] = Some(index);
        self.next_chunk += 1;
        self.retained_bytes += len;
        self.last_sequence = sequence;
        Ok(())
    }

    fn end(&mut self, sequence: u64) -> Result<(), Refusal> {
        self.require_next(sequence)?;
        if self.end.is_some()
            || self.next_chunk != self.chunk_count
            || self.chunks[..self.chunk_count]
                .iter()
                .any(|chunk| !chunk.is_some_and(|entry| entry.staged))
        {
            return Err(Refusal::Premature);
        }
        let len = envelope::encode(
            self.header(Kind::BaselineEnd, sequence, 0),
            &self.baseline_id.to_le_bytes(),
            &mut self.bytes[END_OFFSET..END_OFFSET + END_FRAME_BYTES],
        )
        .expect("fixed End frame fits");
        self.end = Some(Frame {
            sequence,
            len,
            written: 0,
        });
        self.retained_bytes += len;
        self.last_sequence = sequence;
        Ok(())
    }

    fn next_write(&self) -> Result<Option<PendingFrame<'_>>, Refusal> {
        if self.poisoned {
            return Err(Refusal::Poisoned);
        }
        if let Some(frame) = self.begin.filter(|frame| frame.written < frame.len) {
            return Ok(Some(PendingFrame {
                sequence: frame.sequence,
                accepted: frame.written,
                bytes: &self.bytes[BEGIN_OFFSET..BEGIN_OFFSET + frame.len],
            }));
        }
        for chunk in self.chunks[..self.next_chunk].iter().flatten() {
            if chunk.frame.written < chunk.frame.len {
                let start =
                    chunk.slot.expect("unstaged written chunk owns slot") * CHUNK_FRAME_BYTES;
                return Ok(Some(PendingFrame {
                    sequence: chunk.frame.sequence,
                    accepted: chunk.frame.written,
                    bytes: &self.bytes[start..start + chunk.frame.len],
                }));
            }
        }
        if let Some(frame) = self.end.filter(|frame| frame.written < frame.len) {
            return Ok(Some(PendingFrame {
                sequence: frame.sequence,
                accepted: frame.written,
                bytes: &self.bytes[END_OFFSET..END_OFFSET + frame.len],
            }));
        }
        Ok(None)
    }

    fn accept_written(&mut self, sequence: u64, count: usize) -> Result<(), Refusal> {
        let Some(frame) = self.next_write()? else {
            return Err(Refusal::Sequence);
        };
        if frame.sequence != sequence || count > frame.bytes.len() - frame.accepted {
            return Err(Refusal::Sequence);
        }
        if self.begin.is_some_and(|frame| frame.sequence == sequence) {
            self.begin
                .as_mut()
                .expect("test operation should succeed")
                .written += count;
        } else if self.end.is_some_and(|frame| frame.sequence == sequence) {
            self.end
                .as_mut()
                .expect("test operation should succeed")
                .written += count;
        } else {
            self.chunks[..self.next_chunk]
                .iter_mut()
                .flatten()
                .find(|chunk| chunk.frame.sequence == sequence)
                .expect("known chunk sequence")
                .frame
                .written += count;
        }
        Ok(())
    }

    fn chunk_ack(&mut self, key: PeerKey, baseline_id: u64, index: usize) -> Result<(), Refusal> {
        if self.poisoned {
            return Err(Refusal::Poisoned);
        }
        if key != self.key || baseline_id != self.baseline_id || index >= self.next_chunk {
            return Err(Refusal::Stale);
        }
        let chunk = self.chunks[index].as_mut().expect("submitted chunk");
        if chunk.staged {
            return Err(Refusal::Stale);
        }
        if chunk.frame.written != chunk.frame.len {
            return Err(Refusal::Premature);
        }
        let slot = chunk
            .slot
            .take()
            .expect("unstaged chunk owns physical slot");
        assert_eq!(self.slot_owner[slot], Some(index));
        self.slot_owner[slot] = None;
        chunk.staged = true;
        self.retained_bytes -= chunk.frame.len;
        Ok(())
    }

    fn quarantine_uncertain_write(&mut self) {
        self.poisoned = true;
    }

    fn tombstones(&self) -> usize {
        usize::from(self.begin.is_some()) + self.next_chunk + usize::from(self.end.is_some())
    }
}

fn key() -> PeerKey {
    PeerKey {
        match_id: *b"0123456789abcdef",
        peer_id: 3,
        connection_epoch: 7,
    }
}

fn write_all(stager: &mut PhysicalBaseline) {
    while let Some(frame) = stager.next_write().expect("test operation should succeed") {
        let remaining = frame.bytes.len() - frame.accepted;
        stager
            .accept_written(frame.sequence, remaining)
            .expect("test operation should succeed");
    }
}

#[test]
fn eight_chunks_use_two_physical_credits_and_keep_exact_tombstones() {
    // Every chunk has distinct bytes, so a wrong image offset is observable.
    let image: Vec<u8> = (0..IMAGE_BYTES)
        .map(|offset| {
            u8::try_from(((offset / CHUNK_BYTES) * 17 + offset % 251) % 256).expect("byte pattern")
        })
        .collect(); // Separate immutable image owner.
    let mut stager = PhysicalBaseline::new(key(), 41, 2, image.len(), [3; 32])
        .expect("test operation should succeed");
    assert_eq!(stager.bytes.len(), STORAGE_BYTES);
    assert!(size_of::<PhysicalBaseline>() + STORAGE_BYTES <= 96 * 1024);
    stager.begin(3, 100).expect("test operation should succeed");
    let begin = envelope::decode(&stager.bytes[BEGIN_OFFSET..BEGIN_OFFSET + BEGIN_FRAME_BYTES])
        .expect("test operation should succeed");
    assert_eq!(begin.header.kind, Kind::BaselineBegin);
    assert_eq!(begin.header.message_sequence, 3);
    assert_eq!(begin.header.match_id, key().match_id);
    assert_eq!(begin.header.connection_epoch, key().connection_epoch);
    assert_eq!(begin.header.server_tick, 100);
    assert_eq!(begin.header.acknowledged_sequence, 0);
    assert_eq!(begin.payload[..8], 41u64.to_le_bytes());
    assert_eq!(begin.payload[8..16], 100u64.to_le_bytes());
    assert_eq!(
        begin.payload[16..20],
        (u32::try_from(IMAGE_BYTES).expect("bounded test value")).to_le_bytes()
    );
    assert_eq!(begin.payload[20..22], 8u16.to_le_bytes());
    assert_eq!(begin.payload[22..24], [0; 2]);
    assert_eq!(begin.payload[24..56], [1; 32]);
    assert_eq!(begin.payload[56..88], [2; 32]);
    assert_eq!(begin.payload[88..120], [3; 32]);
    stager
        .submit_chunk(4, 0, &image[..CHUNK_BYTES])
        .expect("test operation should succeed");
    stager
        .submit_chunk(5, 1, &image[CHUNK_BYTES..2 * CHUNK_BYTES])
        .expect("test operation should succeed");
    assert_eq!(
        stager.retained_bytes,
        BEGIN_FRAME_BYTES + 2 * CHUNK_FRAME_BYTES
    );
    assert_eq!(stager.tombstones(), 3);
    assert_eq!(
        stager.submit_chunk(6, 2, &image[2 * CHUNK_BYTES..3 * CHUNK_BYTES]),
        Err(Refusal::Credit)
    );
    assert_eq!(stager.last_sequence, 5);
    assert_eq!(stager.chunk_ack(key(), 41, 1), Err(Refusal::Premature));
    let begin_frame = stager
        .next_write()
        .expect("test operation should succeed")
        .expect("test operation should succeed");
    assert_eq!(
        envelope::decode(begin_frame.bytes)
            .expect("test operation should succeed")
            .header
            .kind,
        Kind::BaselineBegin
    );
    stager
        .accept_written(3, 5)
        .expect("test operation should succeed");
    let frame = stager
        .next_write()
        .expect("test operation should succeed")
        .expect("test operation should succeed");
    assert_eq!((frame.sequence, frame.accepted), (3, 5));
    write_all(&mut stager);
    assert_eq!(stager.chunk_ack(key(), 42, 1), Err(Refusal::Stale));
    assert_eq!(stager.chunk_ack(key(), 41, 8), Err(Refusal::Stale));
    let wrong = PeerKey {
        connection_epoch: 8,
        ..key()
    };
    assert_eq!(stager.chunk_ack(wrong, 41, 1), Err(Refusal::Stale));
    stager
        .chunk_ack(key(), 41, 1)
        .expect("test operation should succeed"); // Out-of-order staging ACK.
    let chunk0_before = stager.bytes[..CHUNK_FRAME_BYTES].to_vec();
    stager
        .submit_chunk(6, 2, &image[2 * CHUNK_BYTES..3 * CHUNK_BYTES])
        .expect("test operation should succeed");
    assert_eq!(
        stager.chunks[2]
            .expect("test operation should succeed")
            .slot,
        Some(1)
    );
    assert_eq!(stager.chunk_ack(key(), 41, 1), Err(Refusal::Stale));
    assert_eq!(stager.slot_owner, [Some(0), Some(2)]);
    assert_eq!(&stager.bytes[..CHUNK_FRAME_BYTES], chunk0_before);
    let chunk2 = envelope::decode(&stager.bytes[CHUNK_FRAME_BYTES..2 * CHUNK_FRAME_BYTES])
        .expect("test operation should succeed");
    assert_eq!(chunk2.header.message_sequence, 6);
    assert_eq!(chunk2.payload[..8], 41u64.to_le_bytes());
    assert_eq!(chunk2.payload[8..12], 2u32.to_le_bytes());
    assert_eq!(
        &chunk2.payload[12..],
        &image[2 * CHUNK_BYTES..3 * CHUNK_BYTES]
    );
    write_all(&mut stager);
    stager
        .chunk_ack(key(), 41, 0)
        .expect("test operation should succeed");
    for index in 3..8 {
        let start = index * CHUNK_BYTES;
        stager
            .submit_chunk(index as u64 + 4, index, &image[start..start + CHUNK_BYTES])
            .expect("test operation should succeed");
        write_all(&mut stager);
        stager
            .chunk_ack(key(), 41, index - 1)
            .expect("test operation should succeed");
    }
    stager
        .chunk_ack(key(), 41, 7)
        .expect("test operation should succeed");
    assert_eq!(stager.retained_bytes, BEGIN_FRAME_BYTES);
    assert_eq!(stager.tombstones(), 9);
    stager.end(12).expect("test operation should succeed");
    write_all(&mut stager);
    assert_eq!(stager.retained_bytes, BEGIN_FRAME_BYTES + END_FRAME_BYTES);
    assert_eq!(stager.tombstones(), 10);
    let end = envelope::decode(&stager.bytes[END_OFFSET..END_OFFSET + END_FRAME_BYTES])
        .expect("test operation should succeed");
    assert_eq!(end.header.kind, Kind::BaselineEnd);
    assert_eq!(end.header.message_sequence, 12);
    assert_eq!(end.payload, 41u64.to_le_bytes());
    assert_eq!(stager.last_sequence, 12);
}

#[test]
fn partial_final_chunk_and_uncertain_write_keep_exact_sequence_and_bytes() {
    assert!(PhysicalBaseline::new(key(), 1, 0, 0, [0; 32]).is_err());
    assert!(PhysicalBaseline::new(key(), 1, 0, IMAGE_BYTES + 1, [0; 32]).is_err());
    let image = vec![0x7b; CHUNK_BYTES + 1];
    let mut stager = PhysicalBaseline::new(key(), 5, 0, image.len(), [9; 32])
        .expect("test operation should succeed");
    stager.begin(1, 1).expect("test operation should succeed");
    assert_eq!(
        stager.submit_chunk(3, 0, &image[..CHUNK_BYTES]),
        Err(Refusal::Sequence)
    );
    assert_eq!(
        stager.submit_chunk(2, 0, &image[..CHUNK_BYTES - 1]),
        Err(Refusal::Invalid)
    );
    assert_eq!(stager.last_sequence, 1);
    stager
        .submit_chunk(2, 0, &image[..CHUNK_BYTES])
        .expect("test operation should succeed");
    assert_eq!(stager.end(3), Err(Refusal::Premature));
    stager
        .submit_chunk(3, 1, &image[CHUNK_BYTES..])
        .expect("test operation should succeed");
    assert_eq!(
        stager.chunks[1]
            .expect("test operation should succeed")
            .frame
            .len,
        envelope::HEADER_BYTES + 12 + 1
    );
    let snapshot = stager.bytes[..CHUNK_FRAME_BYTES].to_vec();
    stager.quarantine_uncertain_write();
    assert_eq!(stager.next_write().err(), Some(Refusal::Poisoned));
    assert_eq!(stager.chunk_ack(key(), 5, 0), Err(Refusal::Poisoned));
    assert_eq!(
        stager.submit_chunk(4, 2, &image[CHUNK_BYTES..]),
        Err(Refusal::Poisoned)
    );
    assert_eq!(stager.last_sequence, 3);
    assert_eq!(&stager.bytes[..CHUNK_FRAME_BYTES], snapshot);
}
