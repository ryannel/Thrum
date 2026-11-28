pub(crate) mod snapshot;
mod voice;

use std::sync::{Arc, atomic::Ordering};

use rtrb::Consumer;

use crate::{
    Config, Fate, Listener, Outcome, Play, PlayNumber, Record, SoundId, VoiceSettings, Volumes,
    record::Transport,
    samples::{Bank, Lease},
    spatial,
};

use voice::{Envelope, Voice, range_ramp_frames};

pub(crate) enum Command {
    Play(Play, Outcome, Lease),
    Stop(PlayNumber),
    State(u64, StateChange),
}

pub(crate) enum StateChange {
    Emitter(usize, Option<EmitterVoice>, Option<u64>),
    Listener(Listener),
    Volumes(Volumes),
}

pub(crate) struct EmitterVoice {
    pub(crate) identity: u64,
    pub(crate) sound: SoundId,
    pub(crate) settings: VoiceSettings,
    pub(crate) held: bool,
    pub(crate) samples: Lease,
}

struct VoiceSlots {
    slots: Box<[Option<Voice>]>,
    active: Vec<usize>,
}

impl VoiceSlots {
    fn new(capacity: usize) -> Self {
        Self {
            slots: (0..capacity).map(|_| None).collect(),
            active: Vec::with_capacity(capacity),
        }
    }

    fn take(&mut self, index: usize) -> Option<Voice> {
        let voice = self.slots[index].take()?;
        self.active.remove(
            self.active
                .binary_search(&index)
                .expect("occupied slot is active"),
        );
        Some(voice)
    }

    fn pop(&mut self) -> Option<Voice> {
        let index = self.active.pop()?;
        self.slots[index].take()
    }

    fn insert(&mut self, index: usize, voice: Voice) {
        debug_assert!(self.slots[index].is_none());
        self.slots[index] = Some(voice);
        // Preserve slot order so adding or freeing voices cannot change summation order.
        let offset = self.active.partition_point(|&active| active < index);
        self.active.insert(offset, index);
    }

    fn iter(&self) -> impl Iterator<Item = &Voice> {
        self.active
            .iter()
            .map(|&index| self.slots[index].as_ref().expect("active slot is occupied"))
    }

    fn render(&mut self, config: Config, frame: &mut [f32; 2]) -> usize {
        let mut sounding = 0;
        self.active.retain(|&index| {
            let slot = &mut self.slots[index];
            let voice = slot.as_mut().expect("active slot is occupied");
            sounding += usize::from(voice.sounding());
            let live = voice.render(config, frame);
            if !live {
                *slot = None;
            }
            live
        });
        sounding
    }
}

/// The pulling half of an audio connection. It may be handed to an independent thread.
/// Dropping it closes the connection and expires plays that had yet to start.
pub struct Mix {
    config: Config,
    shots: VoiceSlots,
    emitters: VoiceSlots,
    tails: Vec<Voice>,
    started: Vec<Outcome>,
    state_reader: Option<snapshot::Subscriber>,
    state_floor: Option<u64>,
    listener: Listener,
    current: Volumes,
    volumes: Volumes,
    resume: Envelope,
    record: Record,
    last_applied_play: u64,
    retired: bool,
    commands: Consumer<Command>,
    pub(crate) transport: Arc<Transport>,
    // Keeps sample storage alive when a command or voice releases its lease during a pull,
    // including when the game has already dropped its handle.
    _sample_bank: Arc<Bank>,
}

impl Mix {
    pub(crate) fn new(
        config: Config,
        commands: Consumer<Command>,
        transport: Arc<Transport>,
        bank: Arc<Bank>,
        state_reader: snapshot::Subscriber,
    ) -> Self {
        Self {
            config,
            shots: VoiceSlots::new(config.one_shots),
            emitters: VoiceSlots::new(config.emitters),
            tails: Vec::with_capacity(tail_capacity(config).expect("configuration was validated")),
            started: Vec::with_capacity(config.one_shots),
            state_reader: Some(state_reader),
            state_floor: None,
            listener: Listener::default(),
            current: Volumes::default(),
            volumes: Volumes::default(),
            resume: Envelope::default(),
            record: Record {
                epoch: transport.state().0,
                ..Record::default()
            },
            last_applied_play: 0,
            retired: false,
            commands,
            transport,
            _sample_bank: bank,
        }
    }

    /// Permits pulls. After an interruption or rebuild, output ramps up from silence.
    pub fn start_pulling(&mut self) {
        if self.record.pulling {
            return;
        }
        self.restore_state();
        self.record.pulling = true;
        self.transport.set_state(self.record.epoch, true);
    }

    /// Stops rendering and advances the epoch if pulling had started.
    pub fn stop_pulling(&mut self) {
        if !self.record.pulling {
            return;
        }
        self.record.pulling = false;
        self.record.epoch += 1;
        self.transport.set_state(self.record.epoch, false);
        while let Some(voice) = self.shots.pop() {
            if let Some(outcome) = voice.pending {
                self.finish(outcome, Fate::Interrupted);
            } else {
                self.record.interrupted_voices += 1;
            }
        }
        self.record.interrupted_voices += self
            .tails
            .iter()
            .filter(|voice| voice.emitter.is_none())
            .count() as u64;
        self.tails.clear();
        self.resume.restart(self.config.ramp_frames);
        self.apply_waiting();
        self.publish();
    }

    /// Writes stereo frames, left then right. A trailing unpaired sample is set to zero.
    /// When stopped, writes silence without advancing the mix.
    pub fn pull(&mut self, samples: &mut [f32]) {
        if !self.record.pulling || samples.len() < 2 {
            samples.fill(0.0);
            return;
        }
        self.apply_waiting();
        self.render(samples);
        self.publish();
    }

    pub(crate) fn retire(&mut self) -> Record {
        if self.record.pulling {
            self.stop_pulling();
        } else {
            self.record.epoch += 1;
            self.transport.set_state(self.record.epoch, false);
            self.apply_waiting();
        }
        self.retired = true;
        self.record
    }

    pub(crate) fn rebuilt(&mut self, mut record: Record) {
        record.voices = 0;
        self.record = record;
        self.resume.restart(self.config.ramp_frames);
    }

    fn publish(&mut self) {
        self.record.voices = if self.record.pulling {
            self.voices().count()
        } else {
            0
        };
        self.transport.stats.publish(&self.record);
    }

    fn voices(&self) -> impl Iterator<Item = &Voice> {
        self.shots
            .iter()
            .chain(self.emitters.iter())
            .chain(&self.tails)
            .filter(|v| v.sounding())
    }

    fn apply_waiting(&mut self) {
        // Snapshot the available commands so a concurrent producer cannot prolong this pull.
        for _ in 0..self.commands.slots() {
            let command = self
                .commands
                .pop()
                .expect("only this consumer removes commands");
            self.apply(command);
        }
    }

    pub(crate) fn apply(&mut self, command: Command) {
        match command {
            Command::Play(play, outcome, samples) => {
                self.last_applied_play = outcome.number.0;
                self.play(play, outcome, samples);
            }
            Command::Stop(number) => {
                if let Some(&index) = self.shots.active.iter().find(|&&index| {
                    self.shots.slots[index]
                        .as_ref()
                        .is_some_and(|voice| voice.number == Some(number))
                }) {
                    let voice = self.shots.slots[index]
                        .as_mut()
                        .expect("located a sounding voice");
                    if voice.pending.is_some() {
                        voice.stop_after_start = true;
                    } else {
                        self.stop_shot(index);
                    }
                }
            }
            Command::State(revision, change) => {
                if self.record.pulling && self.state_floor.is_none_or(|floor| revision > floor) {
                    self.change_state(change);
                }
            }
        }
    }

    fn change_state(&mut self, change: StateChange) {
        match change {
            StateChange::Emitter(index, new, range_exit) => {
                self.emitter(index, new, range_exit, false);
            }
            StateChange::Listener(listener) => {
                self.listener = listener;
                for group in [&mut self.shots, &mut self.emitters] {
                    for &index in &group.active {
                        group.slots[index]
                            .as_mut()
                            .expect("active slot is occupied")
                            .retarget(listener);
                    }
                }
            }
            StateChange::Volumes(volumes) => self.volumes = volumes,
        }
    }

    fn restore_state(&mut self) {
        let mut reader = self
            .state_reader
            .take()
            .expect("mix owns its snapshot subscriber");
        if let Some(snapshot) = reader.latest() {
            self.state_floor = Some(snapshot.revision);
            self.listener = snapshot.listener;
            self.volumes = snapshot.volumes;
            self.current = snapshot.volumes;
            // An emitter may have been reassigned while stopped. Move retained voices by
            // identity before applying the snapshot so their loop positions follow them.
            for (index, emitter) in snapshot.emitters.iter().enumerate() {
                if let Some(emitter) = emitter
                    && let Some(previous) = self.emitters.slots.iter().position(|slot| {
                        slot.as_ref()
                            .is_some_and(|voice| voice.emitter == Some(emitter.identity))
                    })
                {
                    self.emitters.slots.swap(index, previous);
                }
            }
            self.emitters.active.clear();
            for (index, emitter) in snapshot.emitters.iter_mut().enumerate() {
                if self.emitters.slots[index].is_some() {
                    self.emitters.active.push(index);
                }
                self.emitter(index, emitter.take(), None, true);
            }
        }
        if self.resume.value == 0.0 {
            // Every restored voice uses the one output ramp, including newly selected emitters.
            for &index in &self.emitters.active {
                self.emitters.slots[index]
                    .as_mut()
                    .expect("active slot is occupied")
                    .settle();
            }
        }
        self.state_reader = Some(reader);
    }

    fn finish(&mut self, mut outcome: Outcome, fate: Fate) {
        outcome.fate = fate;
        self.record.fates[fate as usize] += 1;
        let serial = self.transport.serial.fetch_add(1, Ordering::SeqCst);
        self.transport.outcomes.push(serial, outcome);
    }

    fn stop_shot(&mut self, index: usize) {
        if let Some(voice) = self.shots.take(index) {
            self.tail(voice, false);
        }
    }

    fn tail(&mut self, mut voice: Voice, leaving_radius: bool) {
        if voice.audible() {
            voice.ramp_out(self.config, leaving_radius);
            debug_assert!(self.tails.len() < self.tails.capacity());
            self.tails.push(voice);
        }
    }

    fn play(&mut self, play: Play, outcome: Outcome, samples: Lease) {
        if outcome.epoch != self.record.epoch || !self.record.pulling {
            self.finish(outcome, Fate::Interrupted);
            return;
        }
        let Some(loudness) = spatial::loudness(play.settings, self.listener) else {
            self.finish(outcome, Fate::OutOfRange);
            return;
        };
        if play.limit.is_some_and(|limit| {
            self.voices().filter(|v| v.sound == play.sound).count() >= limit.get()
        }) {
            self.finish(outcome, Fate::Limit);
            return;
        }

        let slot = self
            .shots
            .slots
            .iter()
            .position(Option::is_none)
            .or_else(|| {
                self.shots
                    .active
                    .iter()
                    .zip(self.shots.iter())
                    .map(|(&i, voice)| (i, voice.loudness(self.listener)))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .filter(|(_, quietest)| loudness > *quietest)
                    .map(|(i, _)| i)
            });
        let Some(index) = slot else {
            self.finish(outcome, Fate::Pool);
            return;
        };

        if let Some(voice) = self.shots.take(index) {
            if let Some(pending) = voice.pending {
                self.finish(pending, Fate::Pool);
            } else {
                self.record.displaced += 1;
                self.tail(voice, false);
            }
        }
        self.shots.insert(
            index,
            Voice::new(
                play.sound,
                samples,
                play.settings,
                self.listener,
                Some(outcome),
                None,
            ),
        );
    }

    fn emitter(
        &mut self,
        index: usize,
        new: Option<EmitterVoice>,
        range_exit: Option<u64>,
        restoring: bool,
    ) {
        if let (Some(voice), Some(next)) = (&mut self.emitters.slots[index], &new)
            && voice.emitter == Some(next.identity)
        {
            voice.set(
                next.settings,
                self.listener,
                next.held,
                self.config.ramp_frames,
            );
            return;
        }
        if let Some(voice) = self.emitters.take(index)
            && !restoring
        {
            let leaving_radius = range_exit.is_some() && voice.emitter == range_exit;
            self.tail(voice, leaving_radius);
        }
        if let Some(next) = new {
            let mut voice = Voice::new(
                next.sound,
                next.samples,
                next.settings,
                self.listener,
                None,
                Some(next.identity),
            );
            voice.set(
                next.settings,
                self.listener,
                next.held,
                self.config.ramp_frames,
            );
            self.emitters.insert(index, voice);
        }
    }

    pub(crate) fn render(&mut self, samples: &mut [f32]) {
        let (frames, unpaired) = samples.as_chunks_mut::<2>();
        unpaired.fill(0.0);
        let Some((first, rest)) = frames.split_first_mut() else {
            return;
        };
        for &index in &self.shots.active {
            if let Some(outcome) = self.shots.slots[index]
                .as_mut()
                .expect("active slot is occupied")
                .pending
                .take()
            {
                self.started.push(outcome);
            }
        }
        self.render_frame(first);
        for index in 0..self.started.len() {
            self.finish(self.started[index], Fate::Started);
        }
        if !self.started.is_empty() {
            let mut active = 0;
            while let Some(&slot) = self.shots.active.get(active) {
                if self.shots.slots[slot]
                    .as_ref()
                    .is_some_and(|voice| voice.stop_after_start)
                {
                    self.stop_shot(slot);
                } else {
                    active += 1;
                }
            }
            self.started.clear();
        }
        for output in &mut *rest {
            self.render_frame(output);
        }
        self.record.frames += 1 + rest.len() as u64;
    }

    fn render_frame(&mut self, output: &mut [f32; 2]) {
        let mut frame = [0.0; 2];
        let sounding = self.shots.render(self.config, &mut frame)
            + self.emitters.render(self.config, &mut frame)
            + self.tails.len();
        self.record.peak_voices = self.record.peak_voices.max(sounding);
        self.tails
            .retain_mut(|voice| voice.render(self.config, &mut frame) && !voice.tail_done());
        let scale = (self.current.effects * self.current.master) * self.resume.value;
        let scaled = frame.map(|sample| sample * scale);
        self.record.clipped_frames += u64::from(scaled.iter().any(|s| s.abs() >= self.config.knee));
        *output = scaled.map(|sample| clip(sample, self.config.knee));
        self.glide_volumes();
        self.resume.advance();
    }

    fn glide_volumes(&mut self) {
        let step = self.config.glide_step;
        self.current.master = spatial::glide(self.current.master, self.volumes.master, step);
        self.current.music = spatial::glide(self.current.music, self.volumes.music, step);
        self.current.effects = spatial::glide(self.current.effects, self.volumes.effects, step);
    }
}

impl Drop for Mix {
    fn drop(&mut self) {
        if !self.retired {
            self.stop_pulling();
            // There may be a command crossing the stopped-state publication. The handle
            // resolves any play still in flight after this final bounded drain.
            self.apply_waiting();
            self.publish();
            self.transport.close(self.last_applied_play);
        }
    }
}

pub(crate) fn tail_capacity(config: Config) -> Option<usize> {
    let shots = config.one_shots.checked_mul(config.ramp_frames as usize)?;
    let emitters = if config.emitters == 0 {
        0
    } else {
        // Equal-power pan can exceed unit gain. Reserve enough frames to leave the
        // radius at the glide step even from the highest channel gain.
        let frames = range_ramp_frames(config, std::f32::consts::SQRT_2)?;
        config.emitters.checked_mul(frames as usize)?
    };
    shots.checked_add(emitters)
}

fn clip(sample: f32, knee: f32) -> f32 {
    let magnitude = sample.abs();
    if magnitude <= knee {
        return sample;
    }
    let width = 1.0 - knee;
    let excess = magnitude - knee;
    (knee + width * (excess / (width + excess)))
        .min(f32::from_bits(1.0f32.to_bits() - 1))
        .copysign(sample)
}
