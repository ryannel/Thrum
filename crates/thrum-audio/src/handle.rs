mod emitters;

use std::{
    cell::Cell,
    sync::{Arc, atomic::Ordering},
};

use rtrb::{Producer, RingBuffer};

use crate::{
    Config, EmitterId, Error, Fate, Listener, Mix, Outcome, Play, PlayNumber, Record, SoundId,
    VoiceSettings, Volumes,
    mix::{Command, EmitterVoice, StateChange, snapshot, tail_capacity},
    record::{Outcomes, Transport},
    samples::{Clip, Samples},
    spatial,
};

use emitters::Emitters;

/// Game-side state and the single writer of audio commands.
pub struct Handle {
    config: Config,
    commands: Option<Producer<Command>>,
    transport: Arc<Transport>,
    samples: Samples,
    sounds: Vec<Option<(SoundId, usize)>>,
    next_sound: u64,
    next_play: u64,
    last_queued_play: Option<Outcome>,
    listener: Listener,
    volumes: Volumes,
    listener_dirty: bool,
    volumes_dirty: bool,
    emitters: Emitters,
    sent: Vec<SentEmitter>,
    blocked_state: Option<usize>,
    state_writer: snapshot::Publisher,
    state_revision: u64,
    local_outcomes: Outcomes,
    outcome_scratch: Vec<(u64, Outcome)>,
    outcomes_read: [u64; 2],
    local_record: Record,
    mix_record: Cell<Record>,
}

#[derive(Clone, Copy, Default)]
struct SentEmitter {
    value: Option<(EmitterId, u64)>,
    epoch: u64,
}

impl Handle {
    /// Creates the game and pulling halves. Call [Mix::start_pulling] to admit plays.
    ///
    /// # Errors
    /// Returns [Error::Config] for invalid capacity or tuning.
    pub fn new(config: Config) -> Result<(Self, Mix), Error> {
        let bank_capacity = config
            .commands
            .checked_add(config.sounds)
            .and_then(|count| count.checked_add(config.one_shots))
            .and_then(|count| count.checked_add(config.emitters.checked_mul(4)?))
            .and_then(|count| count.checked_add(tail_capacity(config)?))
            .and_then(|count| count.checked_add(2))
            .filter(|&count| count <= isize::MAX as usize / 256)
            .ok_or(Error::Config)?;
        if config.commands == 0
            || config.outcomes == 0
            || config.outcomes > isize::MAX as usize / 256
            || config.ramp_frames == 0
            || !(0.0 < config.glide_step && config.glide_step <= 1.0)
            || !(0.0..1.0).contains(&config.knee)
            || !config.emitter_margin.is_finite()
            || config.emitter_margin < 0.0
        {
            return Err(Error::Config);
        }
        let samples = Samples::new(bank_capacity);
        let (commands, receiver) = RingBuffer::new(config.commands);
        let transport = Arc::new(Transport::new(config.outcomes, 0));
        let (state_writer, state_reader) = snapshot::channel(config.emitters);
        let mix = Mix::new(
            config,
            receiver,
            Arc::clone(&transport),
            samples.bank(),
            state_reader,
        );
        Ok((
            Self {
                config,
                commands: Some(commands),
                transport,
                samples,
                sounds: vec![None; config.sounds],
                next_sound: 0,
                next_play: 0,
                last_queued_play: None,
                listener: Listener::default(),
                volumes: Volumes::default(),
                listener_dirty: false,
                volumes_dirty: false,
                emitters: Emitters::new(config.emitters),
                sent: vec![SentEmitter::default(); config.emitters],
                blocked_state: None,
                state_writer,
                state_revision: 0,
                local_outcomes: Outcomes::new(config.outcomes),
                outcome_scratch: Vec::with_capacity(config.outcomes * 2),
                outcomes_read: [0; 2],
                local_record: Record::default(),
                mix_record: Cell::default(),
            },
            mix,
        ))
    }

    /// Registers owned interleaved 48 kHz PCM with one or two channels, without copying it.
    ///
    /// # Errors
    /// Returns a typed validation error or [Error::SoundTableFull].
    pub fn add_sound(&mut self, samples: Arc<[i16]>, channels: u8) -> Result<SoundId, Error> {
        let clip = Clip::new(samples, channels)?;
        let slot = self
            .sounds
            .iter()
            .position(Option::is_none)
            .ok_or(Error::SoundTableFull)?;
        self.next_sound += 1;
        let identity = SoundId(self.next_sound);
        self.sounds[slot] = Some((identity, self.samples.add(clip)));
        Ok(identity)
    }

    /// Replaces a sound for future voices. Existing voices retain their original samples.
    ///
    /// # Errors
    /// Returns a typed validation error or [Error::UnknownSound].
    pub fn replace_sound(
        &mut self,
        sound: SoundId,
        samples: Arc<[i16]>,
        channels: u8,
    ) -> Result<(), Error> {
        let clip = Clip::new(samples, channels)?;
        let index = self.sound_index(sound).ok_or(Error::UnknownSound)?;
        let replacement = (sound, self.samples.add(clip));
        let (_, retired) = self.sounds[index]
            .replace(replacement)
            .ok_or(Error::UnknownSound)?;
        self.samples.retire(retired);
        Ok(())
    }

    /// Removes a sound and ends its emitters. One-shots already submitted can finish.
    ///
    /// # Errors
    /// Returns [Error::UnknownSound] if it is no longer registered.
    pub fn remove_sound(&mut self, sound: SoundId) -> Result<(), Error> {
        let index = self.sound_index(sound).ok_or(Error::UnknownSound)?;
        let (_, retired) = self.sounds[index].take().ok_or(Error::UnknownSound)?;
        self.samples.retire(retired);
        self.emitters.remove_sound(sound);
        self.publish_state();
        self.flush_state();
        Ok(())
    }

    fn sound_index(&self, sound: SoundId) -> Option<usize> {
        self.sounds
            .iter()
            .position(|slot| slot.is_some_and(|(key, _)| key == sound))
    }

    fn sample_index(&self, sound: SoundId) -> Option<usize> {
        self.sounds[self.sound_index(sound)?].map(|(_, samples)| samples)
    }

    /// Requests one play and returns its number immediately.
    pub fn play(&mut self, mut play: Play) -> PlayNumber {
        self.finish_closed_connection();
        self.samples.collect();
        self.next_play += 1;
        let number = PlayNumber(self.next_play);
        let (epoch, pulling) = self.transport.state();
        let base = Outcome {
            number,
            event: play.event,
            tick: play.tick,
            epoch,
            fate: Fate::Started,
        };
        let failure = if let Some(index) = self.sample_index(play.sound)
            && self.validate_settings(&mut play.settings)
        {
            if !pulling {
                Fate::NothingPulling
            } else if self.send_transient(Command::Play(play, base, self.samples.lease(index))) {
                self.last_queued_play = Some(base);
                return number;
            } else {
                Fate::QueueFull
            }
        } else {
            self.local_record.bad_values += 1;
            Fate::BadValue
        };
        self.finish_local(base, failure);
        debug_assert!(
            spatial::settings_finite(play.settings),
            "nonfinite audio voice settings"
        );
        number
    }

    fn finish_local(&mut self, outcome: Outcome, fate: Fate) {
        self.local_record.fates[fate as usize] += 1;
        let serial = self.transport.serial.fetch_add(1, Ordering::SeqCst);
        self.local_outcomes
            .push(serial, Outcome { fate, ..outcome });
    }

    fn unread_play(&self) -> Option<Outcome> {
        let drained = self.transport.last_drained_play()?;
        self.last_queued_play.filter(|play| play.number.0 > drained)
    }

    fn finish_closed_connection(&mut self) {
        if self.transport.last_drained_play().is_none() {
            return;
        }
        if let Some(play) = self.unread_play() {
            self.finish_local(play, Fate::Interrupted);
        }
        self.last_queued_play = None;
        // Release the producer so the abandoned ring cannot keep the last sample lease pinned.
        self.commands = None;
    }

    /// Requests a ramp to silence for a sounding one-shot. Returns whether it was queued.
    pub fn stop_play(&mut self, number: PlayNumber) -> bool {
        self.finish_closed_connection();
        self.send_transient(Command::Stop(number))
    }

    /// Adds an emitter to the scene. Its voice is selected on the next update.
    ///
    /// # Errors
    /// Returns [Error::UnknownSound] or [Error::EmitterSettings] for invalid setup.
    pub fn start_emitter(
        &mut self,
        sound: SoundId,
        mut settings: VoiceSettings,
    ) -> Result<EmitterId, Error> {
        self.sample_index(sound).ok_or(Error::UnknownSound)?;
        if !self.validate_settings(&mut settings) {
            self.local_record.bad_values += 1;
            debug_assert!(
                spatial::settings_finite(settings),
                "nonfinite audio voice settings"
            );
            return Err(Error::EmitterSettings);
        }
        Ok(self.emitters.start(sound, settings))
    }

    /// Sets the emitter's volume, pitch and place together.
    pub fn set_emitter(&mut self, emitter: EmitterId, mut settings: VoiceSettings) {
        if !self.validate_settings(&mut settings) {
            self.local_record.bad_values += 1;
            debug_assert!(
                spatial::settings_finite(settings),
                "nonfinite audio voice settings"
            );
        } else if let Some(entry) = self.emitters.get_mut(emitter) {
            entry.settings = settings;
            entry.revision += 1;
        }
    }

    /// Moves an emitter or clears its place.
    pub fn move_emitter(&mut self, emitter: EmitterId, place: Option<crate::Place>) {
        if !spatial::place_valid(place) {
            self.local_record.bad_values += 1;
            debug_assert!(spatial::place_finite(place), "nonfinite emitter place");
        } else if let Some(entry) = self.emitters.get_mut(emitter) {
            entry.settings.place = place;
            entry.revision += 1;
        }
    }

    /// Holds or releases a loop at its current sample position.
    pub fn hold_emitter(&mut self, emitter: EmitterId, held: bool) {
        if let Some(entry) = self.emitters.get_mut(emitter)
            && entry.held != held
        {
            entry.held = held;
            entry.revision += 1;
        }
    }

    /// Ends an emitter, ramping out its voice if one was present.
    pub fn end_emitter(&mut self, emitter: EmitterId) {
        self.emitters.end(emitter);
    }

    /// Sets the spatial reference frame for all voices.
    pub fn set_listener(&mut self, listener: Listener) {
        if let Some(listener) = spatial::normalized(listener) {
            self.listener = listener;
            self.listener_dirty = true;
            self.publish_state();
            self.try_control(0, StateChange::Listener(listener));
        } else {
            self.local_record.bad_values += 1;
            debug_assert!(
                spatial::listener_finite(listener),
                "nonfinite audio listener"
            );
        }
    }

    /// Sets the three linear volumes. Values outside 0 to 1 are clamped.
    pub fn set_volumes(&mut self, mut volumes: Volumes) {
        if ![volumes.master, volumes.music, volumes.effects]
            .into_iter()
            .all(f32::is_finite)
        {
            self.local_record.bad_values += 1;
            debug_assert!(false, "nonfinite audio volumes");
            return;
        }
        volumes.master = self.clamp(volumes.master, 0.0, 1.0);
        volumes.music = self.clamp(volumes.music, 0.0, 1.0);
        volumes.effects = self.clamp(volumes.effects, 0.0, 1.0);
        self.volumes = volumes;
        self.volumes_dirty = true;
        self.publish_state();
        self.try_control(1, StateChange::Volumes(volumes));
    }

    /// Returns the desired volumes, including updates still waiting for queue space.
    pub fn volumes(&self) -> Volumes {
        self.volumes
    }

    /// Returns the latest valid listener.
    pub fn listener(&self) -> Listener {
        self.listener
    }

    /// Selects emitter voices and sends any pending state that the mix has room for.
    pub fn update(&mut self) {
        self.finish_closed_connection();
        self.samples.collect();
        self.emitters
            .update(self.listener, self.config.emitter_margin);
        self.publish_state();
        self.flush_state();
    }

    /// Returns whether an emitter was selected for a voice at the last update.
    pub fn emitter_voiced(&self, identity: EmitterId) -> bool {
        (0..self.config.emitters).any(|slot| {
            self.emitters
                .get_slot(slot)
                .is_some_and(|entry| entry.identity == identity)
        })
    }

    fn emitter_voice(
        emitters: &Emitters,
        sounds: &[Option<(SoundId, usize)>],
        samples: &Samples,
        index: usize,
    ) -> Option<EmitterVoice> {
        let entry = emitters.get_slot(index)?;
        let slot = sounds
            .iter()
            .flatten()
            .find_map(|&(key, slot)| (key == entry.sound).then_some(slot))
            .expect("a selected emitter has a registered sound");
        Some(EmitterVoice {
            identity: entry.identity.1,
            sound: entry.sound,
            settings: entry.settings,
            held: entry.held,
            samples: samples.lease(slot),
        })
    }

    fn publish_state(&mut self) {
        self.state_revision += 1;
        let snapshot = self.state_writer.back_mut();
        snapshot.listener = self.listener;
        snapshot.volumes = self.volumes;
        snapshot.revision = self.state_revision;
        for (index, voice) in snapshot.emitters.iter_mut().enumerate() {
            *voice = Self::emitter_voice(&self.emitters, &self.sounds, &self.samples, index);
        }
        self.state_writer.publish();
    }

    fn send_state(&mut self, change: StateChange) -> bool {
        self.transport.state().1 && self.send(Command::State(self.state_revision, change))
    }

    fn try_control(&mut self, state: usize, change: StateChange) {
        // A frame's immediate listener or volume write must leave room for a state
        // that was already blocked in the previous update.
        if self.blocked_state.is_some_and(|blocked| blocked != state) {
            return;
        }
        if self.send_state(change) {
            if state == 0 {
                self.listener_dirty = false;
            } else {
                self.volumes_dirty = false;
            }
            if self.blocked_state.is_some() {
                self.blocked_state = Some((state + 1) % (self.sent.len() + 2));
            }
        } else if self.transport.state().1 {
            self.blocked_state = Some(state);
        }
    }

    fn flush_state(&mut self) {
        let (epoch, pulling) = self.transport.state();
        if !pulling {
            return;
        }
        let count = self.sent.len() + 2;
        // Continue where the last full queue stopped, even when a setter supplied new state.
        for offset in 0..count {
            let state = (self.blocked_state.unwrap_or(0) + offset) % count;
            let applied = match state {
                0 if self.listener_dirty => {
                    self.listener_dirty = !self.send_state(StateChange::Listener(self.listener));
                    !self.listener_dirty
                }
                1 if self.volumes_dirty => {
                    self.volumes_dirty = !self.send_state(StateChange::Volumes(self.volumes));
                    !self.volumes_dirty
                }
                0 | 1 => true,
                _ => self.flush_emitter(state - 2, epoch),
            };
            if !applied {
                self.blocked_state = Some(state);
                return;
            }
        }
        self.blocked_state = None;
    }

    fn flush_emitter(&mut self, index: usize, epoch: u64) -> bool {
        let current = self
            .emitters
            .get_slot(index)
            .map(|e| (e.identity, e.revision));
        let sent = self.sent[index];
        // Resume can restore a different slot than the last command reached. Republish every
        // slot once in the new epoch, including empty ones that may need to end a restored voice.
        if sent.epoch == epoch && current == sent.value {
            return true;
        }
        let range_exit = sent
            .value
            .map(|(id, _)| id)
            .into_iter()
            .chain(self.emitters.range_exits[index])
            .filter_map(|id| self.emitters.get(id))
            .find(|e| spatial::loudness(e.settings, self.listener).is_none())
            .map(|e| e.identity.1);
        let voice = Self::emitter_voice(&self.emitters, &self.sounds, &self.samples, index);
        if !self.send_state(StateChange::Emitter(index, voice, range_exit)) {
            return false;
        }
        self.emitters.range_exits[index] = None;
        self.sent[index] = SentEmitter {
            value: current,
            epoch,
        };
        true
    }

    /// Replaces the pulling half and restores the handle's desired state. The new mix is stopped
    /// until [Mix::start_pulling] is called.
    ///
    /// # Errors
    /// Returns [Error::ForeignMix] if the supplied mix belongs to another handle.
    pub fn rebuild_mix(&mut self, mix: &mut Mix) -> Result<(), Error> {
        if !Arc::ptr_eq(&self.transport, &mix.transport) {
            return Err(Error::ForeignMix);
        }
        let record = mix.retire();
        self.last_queued_play = None;
        let (sender, receiver) = RingBuffer::new(self.config.commands);
        self.commands = Some(sender);
        let (state_writer, state_reader) = snapshot::channel(self.config.emitters);
        self.state_writer = state_writer;
        let mut replacement = Mix::new(
            self.config,
            receiver,
            Arc::clone(&self.transport),
            self.samples.bank(),
            state_reader,
        );
        replacement.rebuilt(record);
        self.emitters
            .update(self.listener, self.config.emitter_margin);
        self.publish_state();
        self.listener_dirty = false;
        self.volumes_dirty = false;
        self.blocked_state = None;
        for (index, sent) in self.sent.iter_mut().enumerate() {
            *sent = SentEmitter {
                value: self
                    .emitters
                    .get_slot(index)
                    .map(|e| (e.identity, e.revision)),
                epoch: record.epoch,
            };
        }
        *mix = replacement;
        self.samples.collect();
        Ok(())
    }

    /// Drains the retained outcomes without allocating. The callback receives each outcome once.
    pub fn read_outcomes(&mut self, mut read: impl FnMut(Outcome)) {
        self.finish_closed_connection();
        self.samples.collect();
        let rings = [&self.local_outcomes, &self.transport.outcomes];
        let mut arrivals = 0;
        self.outcome_scratch.clear();
        for (ring, consumed) in rings.into_iter().zip(&mut self.outcomes_read) {
            let end = ring.total();
            arrivals += end - *consumed;
            let start = (*consumed).max(end.saturating_sub(self.config.outcomes as u64));
            self.outcome_scratch
                .extend((start + 1..=end).filter_map(|number| ring.read(number)));
            *consumed = end;
        }
        self.outcome_scratch
            .sort_unstable_by_key(|&(serial, _)| serial);
        let start = self
            .outcome_scratch
            .len()
            .saturating_sub(self.config.outcomes);
        let kept = &self.outcome_scratch[start..];
        self.local_record.outcomes_lost += arrivals - kept.len() as u64;
        for &(_, outcome) in kept {
            read(outcome);
        }
    }

    /// Reads the latest published mix counters together with game-side counters.
    pub fn record(&self) -> Record {
        if let Some(record) = self.transport.stats.read() {
            self.mix_record.set(record);
        }
        let mut record = self.mix_record.get();
        for (mix, local) in record.fates.iter_mut().zip(self.local_record.fates) {
            *mix += local;
        }
        let (epoch, pulling) = self.transport.state();
        record.epoch = epoch;
        record.pulling = pulling;
        record.emitters = self.emitters.count;
        record.eligible_emitters = self.emitters.eligible;
        record.voiced_emitters = self.emitters.voiced();
        record.clamped = self.local_record.clamped;
        record.bad_values = self.local_record.bad_values;
        record.queue_high_water = self.local_record.queue_high_water;
        let unread = u64::from(self.unread_play().is_some());
        record.fates[Fate::Interrupted as usize] += unread;
        let pending = self.local_outcomes.total() + self.transport.outcomes.total()
            - self.outcomes_read.iter().sum::<u64>()
            + unread;
        record.outcomes_lost =
            self.local_record.outcomes_lost + pending.saturating_sub(self.config.outcomes as u64);
        record
    }

    fn send_transient(&mut self, command: Command) -> bool {
        // A producer that submits a play every frame must give deferred state a turn.
        if self.blocked_state.is_some() {
            self.flush_state();
        }
        self.send(command)
    }

    fn send(&mut self, command: Command) -> bool {
        let Some(commands) = &mut self.commands else {
            return false;
        };
        let slots = commands.slots();
        if slots == 0 {
            return false;
        }
        commands
            .push(command)
            .expect("only this producer occupies free slots");
        self.local_record.queue_high_water = self
            .local_record
            .queue_high_water
            .max(self.config.commands - slots + 1);
        true
    }

    fn validate_settings(&mut self, settings: &mut VoiceSettings) -> bool {
        if !spatial::settings_valid(*settings) {
            return false;
        }
        settings.volume = self.clamp(settings.volume, 0.0, 1.0);
        settings.pitch = self.clamp(settings.pitch, 0.5, 2.0);
        true
    }

    fn clamp(&mut self, value: f32, min: f32, max: f32) -> f32 {
        self.local_record.clamped += u64::from(value < min || value > max);
        value.clamp(min, max)
    }

    pub(crate) fn diagnostic_samples(&self, sound: SoundId) -> crate::samples::Lease {
        self.samples.lease(
            self.sample_index(sound)
                .expect("diagnostic registered its sound"),
        )
    }
}
