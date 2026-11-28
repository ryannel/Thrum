use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::SeqCst};

use crate::{Fate, Outcome, PlayNumber, Record};

pub(crate) struct Transport {
    // Low bit is running. The upper bits are the epoch.
    state: AtomicU64,
    closed: AtomicBool,
    last_drained_play: AtomicU64,
    pub(crate) serial: AtomicU64,
    pub(crate) outcomes: Outcomes,
    pub(crate) stats: Stats,
}

impl Transport {
    pub(crate) fn new(outcomes: usize, epoch: u64) -> Self {
        Self {
            state: AtomicU64::new(epoch << 1),
            closed: AtomicBool::new(false),
            last_drained_play: AtomicU64::new(0),
            serial: AtomicU64::new(0),
            outcomes: Outcomes::new(outcomes),
            stats: Stats::default(),
        }
    }

    pub(crate) fn state(&self) -> (u64, bool) {
        let state = self.state.load(SeqCst);
        (state >> 1, state & 1 != 0)
    }

    pub(crate) fn set_state(&self, epoch: u64, running: bool) {
        self.state.store((epoch << 1) | u64::from(running), SeqCst);
    }

    pub(crate) fn close(&self, last_drained_play: u64) {
        self.last_drained_play.store(last_drained_play, SeqCst);
        self.closed.store(true, SeqCst);
    }

    pub(crate) fn last_drained_play(&self) -> Option<u64> {
        self.closed
            .load(SeqCst)
            .then(|| self.last_drained_play.load(SeqCst))
    }
}

pub(crate) struct Outcomes {
    total: AtomicU64,
    entries: Box<[Entry]>,
}

#[derive(Default)]
struct Entry {
    revision: AtomicU64,
    serial: AtomicU64,
    number: AtomicU64,
    event: AtomicU64,
    tick: AtomicU64,
    epoch: AtomicU64,
    fate: AtomicU64,
}

impl Outcomes {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            total: AtomicU64::default(),
            entries: (0..capacity).map(|_| Entry::default()).collect(),
        }
    }

    fn entry(&self, number: u64) -> &Entry {
        let slot = (number - 1) % self.entries.len() as u64;
        &self.entries[usize::try_from(slot).expect("index is within the outcome ring")]
    }

    pub(crate) fn push(&self, serial: u64, outcome: Outcome) {
        let number = self.total.load(SeqCst) + 1;
        let entry = self.entry(number);
        entry.revision.store(0, SeqCst);
        entry.serial.store(serial, SeqCst);
        entry.number.store(outcome.number.0, SeqCst);
        entry.event.store(outcome.event, SeqCst);
        entry.tick.store(outcome.tick.unwrap_or_default(), SeqCst);
        entry.epoch.store(outcome.epoch, SeqCst);
        entry.fate.store(
            ((outcome.fate as u64) << 1) | u64::from(outcome.tick.is_some()),
            SeqCst,
        );
        entry.revision.store(number, SeqCst);
        self.total.store(number, SeqCst);
    }

    pub(crate) fn total(&self) -> u64 {
        self.total.load(SeqCst)
    }

    pub(crate) fn read(&self, number: u64) -> Option<(u64, Outcome)> {
        let entry = self.entry(number);
        if entry.revision.load(SeqCst) != number {
            return None;
        }
        let serial = entry.serial.load(SeqCst);
        let fate = entry.fate.load(SeqCst);
        let outcome = Outcome {
            number: PlayNumber(entry.number.load(SeqCst)),
            event: entry.event.load(SeqCst),
            tick: (fate & 1 != 0).then(|| entry.tick.load(SeqCst)),
            epoch: entry.epoch.load(SeqCst),
            fate: Fate::ALL.get(usize::try_from(fate >> 1).ok()?).copied()?,
        };
        (entry.revision.load(SeqCst) == number).then_some((serial, outcome))
    }
}

const METRICS: usize = 6 + Fate::ALL.len();

#[derive(Default)]
pub(crate) struct Stats {
    revision: AtomicU64,
    values: [AtomicU64; METRICS],
}

impl Stats {
    pub(crate) fn publish(&self, record: &Record) {
        self.revision.fetch_add(1, SeqCst);
        let metrics = [
            record.voices as u64,
            record.peak_voices as u64,
            record.displaced,
            record.interrupted_voices,
            record.clipped_frames,
            record.frames,
        ];
        for (cell, value) in self
            .values
            .iter()
            .zip(metrics.into_iter().chain(record.fates))
        {
            cell.store(value, SeqCst);
        }
        self.revision.fetch_add(1, SeqCst);
    }

    pub(crate) fn read(&self) -> Option<Record> {
        let revision = self.revision.load(SeqCst);
        if !revision.is_multiple_of(2) {
            return None;
        }
        let values: [u64; METRICS] = std::array::from_fn(|i| self.values[i].load(SeqCst));
        if self.revision.load(SeqCst) != revision {
            return None;
        }
        Some(Record {
            voices: usize::try_from(values[0]).expect("published from usize on this platform"),
            peak_voices: usize::try_from(values[1]).expect("published from usize on this platform"),
            displaced: values[2],
            interrupted_voices: values[3],
            clipped_frames: values[4],
            frames: values[5],
            fates: std::array::from_fn(|i| values[6 + i]),
            ..Record::default()
        })
    }
}
