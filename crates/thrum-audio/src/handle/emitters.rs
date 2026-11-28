use std::cmp::Ordering;

use crate::{EmitterId, Listener, SoundId, VoiceSettings, spatial};

pub(super) struct Emitter {
    pub(super) identity: EmitterId,
    pub(super) sound: SoundId,
    pub(super) settings: VoiceSettings,
    pub(super) held: bool,
    pub(super) revision: u64,
    slot: Option<usize>,
}

#[derive(Clone, Copy)]
struct Ranked {
    index: usize,
    loudness: f32,
    serial: u64,
}

fn loudest_first(a: &Ranked, b: &Ranked) -> Ordering {
    b.loudness
        .total_cmp(&a.loudness)
        .then(a.serial.cmp(&b.serial))
}

pub(super) struct Emitters {
    entries: Vec<Option<Emitter>>,
    free: Vec<usize>,
    slots: Vec<Option<usize>>,
    candidates: Vec<Ranked>,
    holders: Vec<Ranked>,
    vacancies: Vec<usize>,
    serial: u64,
    pub(super) count: usize,
    pub(super) eligible: usize,
}

impl Emitters {
    pub(super) fn new(slots: usize) -> Self {
        Self {
            entries: Vec::new(),
            free: Vec::new(),
            slots: vec![None; slots],
            candidates: Vec::new(),
            holders: Vec::with_capacity(slots),
            vacancies: Vec::with_capacity(slots),
            serial: 0,
            count: 0,
            eligible: 0,
        }
    }

    pub(super) fn start(&mut self, sound: SoundId, settings: VoiceSettings) -> EmitterId {
        let index = self.free.pop().unwrap_or(self.entries.len());
        if index == self.entries.len() {
            self.entries.push(None);
            self.free.reserve(self.entries.len() - self.free.len());
            self.candidates
                .reserve(self.entries.len() - self.candidates.len());
        }
        self.serial += 1;
        let identity = EmitterId(index, self.serial);
        self.entries[index] = Some(Emitter {
            identity,
            sound,
            settings,
            held: false,
            revision: 1,
            slot: None,
        });
        self.count += 1;
        identity
    }

    pub(super) fn get_mut(&mut self, identity: EmitterId) -> Option<&mut Emitter> {
        self.entries
            .get_mut(identity.0)?
            .as_mut()
            .filter(|e| e.identity == identity)
    }

    pub(super) fn end(&mut self, identity: EmitterId) {
        if self.get_mut(identity).is_some() {
            self.remove(identity.0);
        }
    }

    pub(super) fn remove_sound(&mut self, sound: SoundId) {
        for index in 0..self.entries.len() {
            if self.entries[index]
                .as_ref()
                .is_some_and(|entry| entry.sound == sound)
            {
                self.remove(index);
            }
        }
    }

    fn remove(&mut self, index: usize) {
        let entry = self.entries[index].take().expect("removed a live emitter");
        if let Some(slot) = entry.slot {
            self.slots[slot] = None;
        }
        self.free.push(index);
        self.count -= 1;
    }

    pub(super) fn get_slot(&self, slot: usize) -> Option<&Emitter> {
        self.entries[self.slots[slot]?].as_ref()
    }

    pub(super) fn voiced(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    pub(super) fn update(&mut self, listener: Listener, margin: f32) {
        self.candidates.clear();
        self.holders.clear();
        // Step 1: release ineligible holders while collecting the ranks of eligible emitters.
        for (index, entry) in self.entries.iter_mut().enumerate() {
            let Some(entry) = entry else { continue };
            if let Some(loudness) = spatial::loudness(entry.settings, listener) {
                let rank = Ranked {
                    index,
                    loudness,
                    serial: entry.identity.1,
                };
                if entry.slot.is_some() {
                    self.holders.push(rank);
                } else {
                    self.candidates.push(rank);
                }
            } else if let Some(slot) = entry.slot.take() {
                self.slots[slot] = None;
            }
        }
        self.eligible = self.candidates.len() + self.holders.len();
        let count = self.candidates.len().min(self.slots.len());
        if count == 0 {
            return;
        }
        if count < self.candidates.len() {
            self.candidates.select_nth_unstable_by(count, loudest_first);
        }
        self.candidates[..count].sort_unstable_by(loudest_first);
        self.vacancies.clear();
        self.vacancies.extend(
            self.slots
                .iter()
                .enumerate()
                .filter_map(|(i, slot)| slot.is_none().then_some(i)),
        );

        // Step 2: the remaining candidates rank below every newly admitted holder.
        let fill = count.min(self.vacancies.len());
        for i in 0..fill {
            self.assign(self.vacancies[i], self.candidates[i].index);
        }

        // Step 3: only original holders can lose a slot. Every evicted holder was quieter
        // than all remaining holders, so cannot be the next successful challenger.
        self.holders.sort_unstable_by(|a, b| loudest_first(b, a));
        for i in 0..(count - fill).min(self.holders.len()) {
            let candidate = self.candidates[fill + i];
            let holder = self.holders[i];
            if candidate.loudness <= holder.loudness * (1.0 + margin) {
                break;
            }
            let slot = self.entries[holder.index]
                .as_mut()
                .expect("ranked a live holder")
                .slot
                .take()
                .expect("ranked a voiced emitter");
            self.assign(slot, candidate.index);
        }
    }

    fn assign(&mut self, slot: usize, index: usize) {
        self.slots[slot] = Some(index);
        self.entries[index]
            .as_mut()
            .expect("ranked a live emitter")
            .slot = Some(slot);
    }
}
