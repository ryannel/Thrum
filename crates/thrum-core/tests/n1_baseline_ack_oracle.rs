//! Test-only N1 sent-frame model for a rejected group followed by a full baseline.
//! This is not a wire codec, queue implementation, or multi-group replica.

use std::collections::{BTreeMap, BTreeSet};

const KIB: usize = 1024;
const FRAME_CAP: usize = 256 * KIB;
const ORDINARY_CAP: usize = 160 * KIB;
const BASELINE_RESERVE: usize = 96 * KIB;
const ENTRY_CAP: usize = 128;
const ORDINARY_ENTRIES: usize = 112;
const BASELINE_ENTRIES: usize = 16;
const CHUNK_DATA: usize = 32 * KIB;
const CHUNK_FRAME: usize = 64 + 12 + CHUNK_DATA;
const BEGIN_FRAME: usize = 64 + 120;
const END_FRAME: usize = 64 + 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Group { rejected: bool },
    Begin,
    Chunk { index: usize, staged: bool },
    End,
}

struct Entry {
    kind: Kind,
    retained_bytes: usize,
    results: BTreeSet<u64>,
    events: BTreeSet<u64>,
}

struct Baseline {
    id: u64,
    supersedes_through: u64,
    chunk_count: usize,
    next_chunk: usize,
    included_results: BTreeSet<u64>,
    included_events: BTreeSet<u64>,
    ended: bool,
}

#[derive(Default)]
struct Ledger {
    next_sequence: u64,
    frames: BTreeMap<u64, Entry>,
    results: BTreeSet<u64>,
    events: BTreeSet<u64>,
    baseline: Option<Baseline>,
    superseded: Vec<u64>,
    admitted: Vec<u64>,
    closed: bool,
}

impl Ledger {
    fn account(&self) -> (usize, usize, usize, usize) {
        let (mut ordinary_bytes, mut ordinary_entries, mut baseline_bytes, mut baseline_entries) =
            (0, 0, 0, 0);
        for entry in self.frames.values() {
            if matches!(entry.kind, Kind::Group { .. }) {
                ordinary_bytes += entry.retained_bytes;
                ordinary_entries += 1;
            } else {
                baseline_bytes += entry.retained_bytes;
                baseline_entries += 1;
            }
        }
        (
            ordinary_bytes,
            ordinary_entries,
            baseline_bytes,
            baseline_entries,
        )
    }

    fn submit(&mut self, entry: Entry) -> Option<u64> {
        if self.closed {
            return None;
        }
        let (ordinary_bytes, ordinary_entries, baseline_bytes, baseline_entries) = self.account();
        let ordinary = matches!(entry.kind, Kind::Group { .. });
        let (new_bytes, new_entries, cap_bytes, cap_entries) = if ordinary {
            (
                ordinary_bytes + entry.retained_bytes,
                ordinary_entries + 1,
                ORDINARY_CAP,
                ORDINARY_ENTRIES,
            )
        } else {
            (
                baseline_bytes + entry.retained_bytes,
                baseline_entries + 1,
                BASELINE_RESERVE,
                BASELINE_ENTRIES,
            )
        };
        if new_bytes > cap_bytes
            || new_entries > cap_entries
            || ordinary_bytes + baseline_bytes + entry.retained_bytes > FRAME_CAP
            || ordinary_entries + baseline_entries + 1 > ENTRY_CAP
        {
            return None;
        }
        let sequence = self.next_sequence + 1;
        self.next_sequence = sequence;
        self.frames.insert(sequence, entry);
        Some(sequence)
    }

    fn group(&mut self, bytes: usize, results: &[u64], events: &[u64]) -> Option<u64> {
        if self.baseline.is_some() {
            return None;
        }
        let entry = Entry {
            kind: Kind::Group { rejected: false },
            retained_bytes: bytes,
            results: results.iter().copied().collect(),
            events: events.iter().copied().collect(),
        };
        let sequence = self.submit(entry)?;
        self.results.extend(results);
        self.events.extend(events);
        Some(sequence)
    }

    fn reject(&mut self, sequence: u64) -> bool {
        match self.frames.get_mut(&sequence) {
            Some(Entry {
                kind: Kind::Group { rejected },
                ..
            }) if !*rejected => {
                *rejected = true;
                true
            }
            _ => false,
        }
    }

    fn header_ack(&mut self, through: u64) -> bool {
        // A cumulative admission ACK cannot jump an outstanding rejected group.
        if through > self.next_sequence {
            return false;
        }
        if self.frames.iter().any(|(&sequence, entry)| {
            sequence <= through && !matches!(entry.kind, Kind::Group { rejected: false })
        }) {
            return false;
        }
        let admitted: Vec<u64> = self.frames.range(..=through).map(|(&seq, _)| seq).collect();
        for sequence in admitted {
            let entry = self
                .frames
                .remove(&sequence)
                .expect("test setup or assertion requires a value");
            for id in entry.results {
                self.results.remove(&id);
            }
            for id in entry.events {
                self.events.remove(&id);
            }
            self.admitted.push(sequence);
        }
        true
    }

    fn begin_replacement(
        &mut self,
        id: u64,
        chunk_count: usize,
        included_results: &[u64],
        included_events: &[u64],
    ) -> Option<u64> {
        if self.baseline.is_some()
            || !self
                .frames
                .values()
                .any(|entry| matches!(entry.kind, Kind::Group { rejected: true }))
            || chunk_count == 0
            || chunk_count * CHUNK_DATA > 256 * KIB
        {
            return None;
        }
        let supersedes_through = self.next_sequence;
        let sequence = self.submit(Entry {
            kind: Kind::Begin,
            retained_bytes: BEGIN_FRAME,
            results: BTreeSet::new(),
            events: BTreeSet::new(),
        })?;
        self.baseline = Some(Baseline {
            id,
            supersedes_through,
            chunk_count,
            next_chunk: 0,
            included_results: included_results.iter().copied().collect(),
            included_events: included_events.iter().copied().collect(),
            ended: false,
        });
        Some(sequence)
    }

    fn submit_chunk(&mut self) -> Option<(u64, usize)> {
        let baseline = self.baseline.as_ref()?;
        let index = baseline.next_chunk;
        if index == baseline.chunk_count
            || self
                .frames
                .values()
                .filter(|entry| {
                    matches!(entry.kind, Kind::Chunk { .. }) && entry.retained_bytes != 0
                })
                .count()
                >= 2
        {
            return None;
        }
        let sequence = self.submit(Entry {
            kind: Kind::Chunk {
                index,
                staged: false,
            },
            retained_bytes: CHUNK_FRAME,
            results: BTreeSet::new(),
            events: BTreeSet::new(),
        })?;
        self.baseline
            .as_mut()
            .expect("test setup or assertion requires a value")
            .next_chunk += 1;
        Some((sequence, index))
    }

    fn chunk_ack(&mut self, baseline_id: u64, index: usize) -> bool {
        if self
            .baseline
            .as_ref()
            .is_none_or(|baseline| baseline.id != baseline_id)
        {
            return false;
        }
        for entry in self.frames.values_mut() {
            if let Kind::Chunk { index: i, staged } = &mut entry.kind
                && *i == index
                && !*staged
            {
                *staged = true;
                entry.retained_bytes = 0; // Keep the application-sequence tombstone.
                return true;
            }
        }
        false
    }

    fn end_replacement(&mut self) -> Option<u64> {
        let baseline = self.baseline.as_ref()?;
        if baseline.next_chunk != baseline.chunk_count
            || self
                .frames
                .values()
                .filter(|entry| matches!(entry.kind, Kind::Chunk { staged: true, .. }))
                .count()
                != baseline.chunk_count
        {
            return None;
        }
        let sequence = self.submit(Entry {
            kind: Kind::End,
            retained_bytes: END_FRAME,
            results: BTreeSet::new(),
            events: BTreeSet::new(),
        })?;
        self.baseline
            .as_mut()
            .expect("test setup or assertion requires a value")
            .ended = true;
        Some(sequence)
    }

    fn baseline_ack(&mut self, id: u64) -> bool {
        let Some(baseline) = self.baseline.as_ref() else {
            return false;
        };
        if baseline.id != id || !baseline.ended {
            return false;
        }
        // No mandatory identity belonging to a superseded group may disappear.
        let covered = self
            .frames
            .iter()
            .filter(|(sequence, entry)| {
                **sequence <= baseline.supersedes_through
                    && matches!(entry.kind, Kind::Group { .. })
            })
            .all(|(_, entry)| {
                entry.results.is_subset(&baseline.included_results)
                    && entry.events.is_subset(&baseline.included_events)
            });
        if !covered {
            return false;
        }
        let baseline = self
            .baseline
            .take()
            .expect("test setup or assertion requires a value");
        for id in &baseline.included_results {
            self.results.remove(id);
        }
        for id in &baseline.included_events {
            self.events.remove(id);
        }
        let superseded: Vec<u64> = self
            .frames
            .iter()
            .filter_map(|(&sequence, entry)| {
                (sequence <= baseline.supersedes_through
                    && matches!(entry.kind, Kind::Group { .. }))
                .then_some(sequence)
            })
            .collect();
        for sequence in superseded {
            self.frames.remove(&sequence);
            self.superseded.push(sequence);
        }
        self.frames.clear(); // Valid BaselineAck also disposes its Begin/Chunk/End frames.
        true
    }

    fn fail_replacement_and_close(&mut self) {
        self.closed = true;
        self.baseline = None;
        self.frames.clear();
        // Mandatory journals remain explicit uncertain outcomes after close.
    }
}

#[test]
fn rejected_sequence_does_not_block_eight_chunk_baseline_credits() {
    let mut ledger = Ledger::default();
    let admitted = ledger
        .group(8000, &[7], &[70])
        .expect("test setup or assertion requires a value");
    assert!(!ledger.header_ack(admitted + 1)); // No unsent publication may be admitted.
    assert_eq!(ledger.results, BTreeSet::from([7]));
    assert_eq!(ledger.account().1, 1);
    assert!(ledger.header_ack(admitted));
    let rejected = ledger
        .group(8000, &[9], &[90])
        .expect("test setup or assertion requires a value");
    assert!(ledger.reject(rejected));
    assert!(!ledger.header_ack(rejected));
    assert_eq!(ledger.results, BTreeSet::from([9]));
    assert_eq!(ledger.events, BTreeSet::from([90]));

    let begin = ledger
        .begin_replacement(41, 8, &[9], &[90])
        .expect("test setup or assertion requires a value");
    assert_eq!(begin, rejected + 1);
    assert!(ledger.group(100, &[], &[]).is_none());
    let (first, first_index) = ledger
        .submit_chunk()
        .expect("test setup or assertion requires a value");
    let (second, second_index) = ledger
        .submit_chunk()
        .expect("test setup or assertion requires a value");
    assert_eq!((first_index, second_index), (0, 1));
    assert_eq!((first, second), (begin + 1, begin + 2));
    assert!(ledger.submit_chunk().is_none()); // Both credits are occupied.
    assert_eq!(ledger.next_sequence, second);
    let mut last_sequence = second;
    let mut peak_baseline_bytes = ledger.account().2;
    assert_eq!(peak_baseline_bytes, BEGIN_FRAME + 2 * CHUNK_FRAME);
    for index in 0..8 {
        assert!(!ledger.chunk_ack(42, index));
        assert!(!ledger.chunk_ack(41, 8));
        assert!(ledger.chunk_ack(41, index));
        assert!(!ledger.chunk_ack(41, index));
        if index + 2 < 8 {
            let (sequence, actual_index) = ledger
                .submit_chunk()
                .expect("test setup or assertion requires a value");
            assert_eq!(actual_index, index + 2);
            assert_eq!(sequence, last_sequence + 1);
            last_sequence = sequence;
            assert!(ledger.submit_chunk().is_none());
        }
        let (ordinary_bytes, ordinary_entries, baseline_bytes, baseline_entries) = ledger.account();
        peak_baseline_bytes = peak_baseline_bytes.max(baseline_bytes);
        assert_eq!(ordinary_bytes, 8000); // Rejected S remains held.
        assert_eq!(ordinary_entries, 1);
        assert!(baseline_bytes <= BASELINE_RESERVE);
        assert!(baseline_entries <= BASELINE_ENTRIES);
        assert!(ordinary_bytes + baseline_bytes <= FRAME_CAP);
        assert!(ordinary_entries + baseline_entries <= ENTRY_CAP);
        assert!(!ledger.header_ack(last_sequence));
    }
    assert_eq!(peak_baseline_bytes, BEGIN_FRAME + 2 * CHUNK_FRAME);
    assert!(ledger.submit_chunk().is_none());
    let end = ledger
        .end_replacement()
        .expect("test setup or assertion requires a value");
    assert_eq!(end, last_sequence + 1);
    assert_eq!(ledger.account(), (8000, 1, BEGIN_FRAME + END_FRAME, 10));
    assert!(!ledger.baseline_ack(42));
    assert_eq!(ledger.results, BTreeSet::from([9]));
    assert!(ledger.baseline_ack(41));
    assert_eq!(ledger.superseded, vec![rejected]);
    assert_eq!(ledger.admitted, vec![admitted]);
    assert!(ledger.results.is_empty() && ledger.events.is_empty());
    assert_eq!(ledger.account(), (0, 0, 0, 0));
    assert_eq!(ledger.next_sequence, end);
    assert!(!ledger.baseline_ack(41));
}

#[test]
fn incomplete_or_failed_replacement_cannot_release_mandatory_records() {
    let mut ledger = Ledger::default();
    let rejected = ledger
        .group(100, &[11], &[110])
        .expect("test setup or assertion requires a value");
    assert!(ledger.reject(rejected));
    ledger
        .begin_replacement(5, 1, &[], &[110])
        .expect("test setup or assertion requires a value");
    let (_, index) = ledger
        .submit_chunk()
        .expect("test setup or assertion requires a value");
    assert!(!ledger.baseline_ack(5));
    assert!(ledger.chunk_ack(5, index));
    ledger
        .end_replacement()
        .expect("test setup or assertion requires a value");
    assert!(!ledger.baseline_ack(5)); // Result 11 was omitted.
    assert_eq!(ledger.results, BTreeSet::from([11]));
    assert_eq!(ledger.events, BTreeSet::from([110]));
    assert_eq!(ledger.superseded, []);
    ledger.fail_replacement_and_close();
    assert!(ledger.closed);
    assert_eq!(ledger.account(), (0, 0, 0, 0));
    assert_eq!(ledger.results, BTreeSet::from([11]));
    assert!(ledger.group(100, &[], &[]).is_none());
}
