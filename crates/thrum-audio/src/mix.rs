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

use voice::{Envelope, Voice};

pub(crate) enum Command {
    Play(Play, Outcome, Lease),
    Stop(PlayNumber),
    State(u64, StateChange),
}

pub(crate) enum StateChange {
    Emitter(usize, Option<EmitterVoice>),
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

/// The pulling half of an audio connection. It may be handed to an independent thread.
pub struct Mix {
    config: Config,
    shots: Box<[Option<Voice>]>,
    emitters: Box<[Option<Voice>]>,
    tails: Vec<Voice>,
    started: Vec<Outcome>,
    state_reader: Option<snapshot::Subscriber>,
    state_floor: Option<u64>,
    listener: Listener,
    current: Volumes,
    volumes: Volumes,
    resume: Envelope,
    record: Record,
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
            shots: (0..config.one_shots).map(|_| None).collect(),
            emitters: (0..config.emitters).map(|_| None).collect(),
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
        for index in 0..self.shots.len() {
            if let Some(voice) = self.shots[index].take() {
                if let Some(outcome) = voice.pending {
                    self.finish(outcome, Fate::Interrupted);
                } else {
                    self.record.interrupted_voices += 1;
                }
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
        self.record
    }

    pub(crate) fn rebuilt(&mut self, mut record: Record) {
        record.voices = 0;
        self.record = record;
        self.resume.restart(self.config.ramp_frames);
    }

    fn publish(&mut self) {
        self.record.voices = self.voices().count();
        self.record.peak_voices = self.record.peak_voices.max(self.record.voices);
        self.transport.stats.publish(&self.record);
    }

    fn voices(&self) -> impl Iterator<Item = &Voice> {
        self.shots.iter().chain(&self.emitters).flatten()
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
            Command::Play(play, outcome, samples) => self.play(play, outcome, samples),
            Command::Stop(number) => {
                if let Some(index) = self.shots.iter().position(|voice| {
                    voice
                        .as_ref()
                        .is_some_and(|voice| voice.number == Some(number))
                }) {
                    let voice = self.shots[index]
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
        self.record.peak_voices = self.record.peak_voices.max(self.voices().count());
    }

    fn change_state(&mut self, change: StateChange) {
        match change {
            StateChange::Emitter(index, new) => self.emitter(index, new, false),
            StateChange::Listener(listener) => {
                self.listener = listener;
                for voice in self.shots.iter_mut().chain(&mut self.emitters).flatten() {
                    voice.retarget(listener);
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
                    && let Some(previous) = self.emitters.iter().position(|slot| {
                        slot.as_ref()
                            .is_some_and(|voice| voice.emitter == Some(emitter.identity))
                    })
                {
                    self.emitters.swap(index, previous);
                }
            }
            for (index, emitter) in snapshot.emitters.iter_mut().enumerate() {
                self.emitter(index, emitter.take(), true);
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
        if let Some(voice) = self.shots[index].take() {
            self.tail(voice);
        }
    }

    fn tail(&mut self, mut voice: Voice) {
        if voice.audible() {
            voice.ramp_out(self.config.ramp_frames);
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
            self.shots
                .iter()
                .flatten()
                .chain(self.tails.iter().filter(|v| v.emitter.is_none()))
                .filter(|v| v.sound == play.sound)
                .count()
                >= limit.get()
        }) {
            self.finish(outcome, Fate::Limit);
            return;
        }

        let slot = self.shots.iter().position(Option::is_none).or_else(|| {
            self.shots
                .iter()
                .enumerate()
                .filter_map(|(i, voice)| Some((i, voice.as_ref()?.loudness(self.listener))))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .filter(|(_, quietest)| loudness > *quietest)
                .map(|(i, _)| i)
        });
        let Some(index) = slot else {
            self.finish(outcome, Fate::Pool);
            return;
        };

        if let Some(voice) = self.shots[index].take() {
            if let Some(pending) = voice.pending {
                self.finish(pending, Fate::Pool);
            } else {
                self.record.displaced += 1;
                self.tail(voice);
            }
        }
        self.shots[index] = Some(Voice::new(
            play.sound,
            samples,
            play.settings,
            self.listener,
            Some(outcome),
            None,
        ));
    }

    fn emitter(&mut self, index: usize, new: Option<EmitterVoice>, restoring: bool) {
        let mut old = self.emitters[index].take();
        if let (Some(voice), Some(next)) = (&mut old, &new)
            && voice.emitter == Some(next.identity)
        {
            voice.set(
                next.settings,
                self.listener,
                next.held,
                self.config.ramp_frames,
            );
            if restoring {
                voice.settle();
            }
            self.emitters[index] = old;
            return;
        }
        if let Some(voice) = old
            && !restoring
        {
            self.tail(voice);
        }
        self.emitters[index] = new.map(|next| {
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
            voice
        });
    }

    pub(crate) fn render(&mut self, samples: &mut [f32]) {
        let (frames, unpaired) = samples.as_chunks_mut::<2>();
        unpaired.fill(0.0);
        let Some((first, rest)) = frames.split_first_mut() else {
            return;
        };
        self.started.extend(
            self.shots
                .iter_mut()
                .flatten()
                .filter_map(|voice| voice.pending.take()),
        );
        self.render_frame(first);
        for index in 0..self.started.len() {
            self.finish(self.started[index], Fate::Started);
        }
        if !self.started.is_empty() {
            for index in 0..self.shots.len() {
                if self.shots[index]
                    .as_ref()
                    .is_some_and(|voice| voice.stop_after_start)
                {
                    self.stop_shot(index);
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
        for slot in self.shots.iter_mut().chain(&mut self.emitters) {
            if let Some(voice) = slot
                && !voice.render(self.config, &mut frame)
            {
                *slot = None;
            }
        }
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

pub(crate) fn tail_capacity(config: Config) -> Option<usize> {
    config
        .one_shots
        .checked_add(config.emitters)?
        .checked_mul(config.ramp_frames as usize)
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
