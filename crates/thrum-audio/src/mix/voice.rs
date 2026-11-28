use crate::{
    Config, Listener, Outcome, PlayNumber, SoundId, VoiceSettings, samples::Lease, spatial,
};

pub(super) struct Voice {
    pub(super) sound: SoundId,
    pub(super) pending: Option<Outcome>,
    pub(super) stop_after_start: bool,
    pub(super) number: Option<PlayNumber>,
    pub(super) emitter: Option<u64>,
    samples: Lease,
    settings: VoiceSettings,
    position: f64,
    pitch: f32,
    gains: [f32; 2],
    targets: [f32; 2],
    envelope: Envelope,
    held: bool,
}

impl Voice {
    pub(super) fn new(
        sound: SoundId,
        samples: Lease,
        settings: VoiceSettings,
        listener: Listener,
        outcome: Option<Outcome>,
        emitter: Option<u64>,
    ) -> Self {
        let targets = spatial::gains(settings, listener);
        Self {
            sound,
            pending: outcome,
            stop_after_start: false,
            number: outcome.map(|play| play.number),
            emitter,
            samples,
            settings,
            position: 0.0,
            pitch: settings.pitch,
            gains: if emitter.is_some() { [0.0; 2] } else { targets },
            targets,
            envelope: Envelope::default(),
            held: false,
        }
    }

    pub(super) fn loudness(&self, listener: Listener) -> f32 {
        spatial::loudness(self.settings, listener).unwrap_or_default()
    }

    pub(super) fn retarget(&mut self, listener: Listener) {
        self.targets = spatial::gains(self.settings, listener);
    }

    pub(super) fn set(
        &mut self,
        settings: VoiceSettings,
        listener: Listener,
        held: bool,
        ramp: u32,
    ) {
        self.settings = settings;
        self.retarget(listener);
        if held != self.held {
            self.held = held;
            self.envelope.to(if held { 0.0 } else { 1.0 }, ramp);
        }
    }

    pub(super) fn settle(&mut self) {
        self.gains = self.targets;
        self.pitch = self.settings.pitch;
        self.envelope.value = if self.held { 0.0 } else { 1.0 };
        self.envelope.remaining = 0;
    }

    pub(super) fn ramp_out(&mut self, frames: u32) {
        self.targets = self.gains;
        self.envelope.to(0.0, frames);
    }

    pub(super) fn audible(&self) -> bool {
        self.envelope.value != 0.0 && self.gains.iter().any(|&gain| gain > 0.0)
    }

    pub(super) fn render(&mut self, config: Config, frame: &mut [f32; 2]) -> bool {
        let clip = self.samples.clip();
        if self.envelope.value != 0.0 {
            let repeated = self.emitter.is_some();
            let left = self.read(0, repeated);
            let right = if clip.channels == 1 {
                left
            } else {
                self.read(1, repeated)
            };
            let sample = if self.settings.place.is_some() && clip.channels == 2 {
                [f32::midpoint(left, right); 2]
            } else {
                [left, right]
            };
            for (channel, output) in frame.iter_mut().enumerate() {
                *output += (sample[channel] * self.gains[channel]) * self.envelope.value;
            }
        }

        // A hold freezes the position immediately. Its current sample fades out and is also
        // the first sample of the release ramp.
        if !self.held {
            let length = f64::from(clip.frames);
            self.position += f64::from(self.pitch);
            if self.position >= length {
                if self.emitter.is_none() {
                    return false;
                }
                // At most two wraps: the maximum pitch is 2 and a clip has at least one frame.
                self.position -= length;
                if self.position >= length {
                    self.position -= length;
                }
            }
        }
        self.pitch = spatial::glide(self.pitch, self.settings.pitch, config.glide_step);
        for (gain, target) in self.gains.iter_mut().zip(self.targets) {
            *gain = spatial::glide(*gain, target, config.glide_step);
        }
        self.envelope.advance();
        true
    }

    pub(super) fn tail_done(&self) -> bool {
        self.envelope.remaining == 0
    }

    fn read(&self, channel: usize, repeated: bool) -> f32 {
        let clip = self.samples.clip();
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "phase is within the clip's u32 frame range"
        )]
        let i = self.position as u32;
        let b = clip.sample(i, channel);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "cubic weights use the mix's f32 precision"
        )]
        let t = (self.position - f64::from(i)) as f32;
        if t == 0.0 {
            return b;
        }
        let last = clip.frames - 1;
        let before = i.checked_sub(1).unwrap_or(if repeated { last } else { 0 });
        let after = next(i, last, repeated);
        let a = clip.sample(before, channel);
        let c = clip.sample(after, channel);
        let d = clip.sample(next(after, last, repeated), channel);
        // Catmull-Rom in Horner form with exact samples at integer phase.
        b + 0.5
            * t
            * ((c - a) + t * ((2.0 * a - 5.0 * b + 4.0 * c - d) + t * (3.0 * (b - c) + d - a)))
    }
}

fn next(frame: u32, last: u32, repeated: bool) -> u32 {
    if frame < last {
        frame + 1
    } else if repeated {
        0
    } else {
        last
    }
}

pub(super) struct Envelope {
    pub(super) value: f32,
    start: f32,
    target: f32,
    length: u32,
    remaining: u32,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            value: 1.0,
            start: 1.0,
            target: 1.0,
            length: 1,
            remaining: 0,
        }
    }
}

impl Envelope {
    pub(super) fn to(&mut self, target: f32, frames: u32) {
        self.start = self.value;
        self.target = target;
        self.length = frames;
        self.remaining = frames;
    }

    pub(super) fn restart(&mut self, frames: u32) {
        self.value = 0.0;
        self.to(1.0, frames);
    }

    pub(super) fn advance(&mut self) {
        if self.remaining != 0 {
            self.remaining -= 1;
            self.value = self.target
                + (self.start - self.target) * (self.remaining as f32 / self.length as f32);
        }
    }
}
