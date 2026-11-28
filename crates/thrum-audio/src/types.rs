use std::{error, fmt, num::NonZeroUsize};

/// Sizes and tuning chosen when the mix is created.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Voice slots for sounds that happen.
    pub one_shots: usize,
    /// Voice slots for emitters.
    pub emitters: usize,
    /// In-flight commands.
    pub commands: usize,
    /// Most recent outcomes kept between reads.
    pub outcomes: usize,
    /// Simultaneously registered sounds.
    pub sounds: usize,
    /// Length of a fade in or out, in frames.
    pub ramp_frames: u32,
    /// Maximal change in a gliding gain or pitch per frame.
    pub glide_step: f32,
    /// Magnitude where the final soft clip starts.
    pub knee: f32,
    /// Fraction by which a challenger must exceed a voiced emitter.
    pub emitter_margin: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            one_shots: 48,
            emitters: 16,
            commands: 256,
            outcomes: 512,
            sounds: 256,
            ramp_frames: 128,
            glide_step: 1.0 / 128.0,
            knee: 0.75,
            emitter_margin: 0.125,
        }
    }
}

/// Why audio setup could not be completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// A configuration value or capacity is invalid.
    Config,
    /// The sample data contains no frame.
    EmptySound,
    /// The channel count is outside mono or stereo.
    Channels,
    /// Samples do not form complete frames, or there are too many frames.
    Frames,
    /// The sound table is full.
    SoundTableFull,
    /// The sound identity is not registered.
    UnknownSound,
    /// The proposed emitter settings are invalid.
    EmitterSettings,
    /// A mix from a different handle was supplied.
    ForeignMix,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Config => "invalid mix configuration",
            Self::EmptySound => "sound has no frames",
            Self::Channels => "sound must have one or two channels",
            Self::Frames => "sound must contain complete frames and fit in a 32-bit frame count",
            Self::SoundTableFull => "mix sound table is full",
            Self::UnknownSound => "sound identity is not registered",
            Self::EmitterSettings => "invalid emitter settings",
            Self::ForeignMix => "mix belongs to a different handle",
        })
    }
}

impl error::Error for Error {}

/// An identity returned by the sound table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundId(pub(crate) u64);

/// An identity returned when an emitter is started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmitterId(pub(crate) usize, pub(crate) u64);

/// The handle's number for one play call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlayNumber(pub u64);

/// A sound's position and the distance at which it is silent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    /// Position in game units.
    pub position: [f32; 3],
    /// Audible radius in game units.
    pub radius: f32,
}

/// The mix's position and right direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Listener {
    /// Position in game units.
    pub position: [f32; 3],
    /// Right direction, normalized when set.
    pub right: [f32; 3],
    /// Distance at which the pan softens.
    pub near: f32,
}

impl Default for Listener {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            right: [1.0, 0.0, 0.0],
            near: 0.25,
        }
    }
}

/// Linear mix gains.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Volumes {
    /// Applied to the entire mix.
    pub master: f32,
    /// Applied to music streams.
    pub music: f32,
    /// Applied to sounds that happen and emitters.
    pub effects: f32,
}

impl Default for Volumes {
    fn default() -> Self {
        Self {
            master: 1.0,
            music: 1.0,
            effects: 1.0,
        }
    }
}

/// Desired settings for a voice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoiceSettings {
    /// Linear gain in 0 to 1.
    pub volume: f32,
    /// Playback speed in 0.5 to 2.
    pub pitch: f32,
    /// Optional position and audible radius.
    pub place: Option<Place>,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pitch: 1.0,
            place: None,
        }
    }
}

/// A request to play a sound once.
#[derive(Clone, Copy, Debug)]
pub struct Play {
    /// Sound to play.
    pub sound: SoundId,
    /// Volume, pitch and optional place.
    pub settings: VoiceSettings,
    /// Maximum sounding one-shots of this sound, if specified.
    pub limit: Option<NonZeroUsize>,
    /// Opaque identity assigned by the caller.
    pub event: u64,
    /// Opaque simulation tick assigned by the caller, if any.
    pub tick: Option<u64>,
}

impl Play {
    /// A play at unit volume and pitch, with no limit or place.
    pub fn new(sound: SoundId, event: u64) -> Self {
        Self {
            sound,
            settings: VoiceSettings::default(),
            limit: None,
            event,
            tick: None,
        }
    }
}

/// The disposition of one play call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Fate {
    /// The first frame was mixed.
    Started,
    /// The sound's simultaneous voice limit was reached.
    Limit,
    /// The one-shot share of the pool had no quieter voice to replace.
    Pool,
    /// The sound's place was outside its audible radius.
    OutOfRange,
    /// The output was not pulling when the call was made.
    NothingPulling,
    /// An output epoch ended before this play started.
    Interrupted,
    /// There was no room in the command queue.
    QueueFull,
    /// A parameter was invalid or the sound had been removed.
    BadValue,
}

impl Fate {
    pub(crate) const ALL: [Self; 8] = [
        Self::Started,
        Self::Limit,
        Self::Pool,
        Self::OutOfRange,
        Self::NothingPulling,
        Self::Interrupted,
        Self::QueueFull,
        Self::BadValue,
    ];
}

/// What became of a play, matched by its play number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Unique number of the call.
    pub number: PlayNumber,
    /// Opaque event identity.
    pub event: u64,
    /// Opaque event tick.
    pub tick: Option<u64>,
    /// Output epoch when called.
    pub epoch: u64,
    /// What happened to the play.
    pub fate: Fate,
}

/// Mix counters and the latest published state.
#[derive(Clone, Copy, Debug, Default)]
pub struct Record {
    /// Sounding voices, including releases until their ramps finish.
    pub voices: usize,
    /// Most voices sounding together in a rendered frame, including releases.
    pub peak_voices: usize,
    /// Live emitter count.
    pub emitters: usize,
    /// Eligible emitters from the last update.
    pub eligible_emitters: usize,
    /// Selected emitters from the last update.
    pub voiced_emitters: usize,
    /// Voices displaced by a louder one-shot.
    pub displaced: u64,
    /// Sounding one-shots cut at an interruption.
    pub interrupted_voices: u64,
    /// Outcomes dropped because they exceeded the retained window.
    pub outcomes_lost: u64,
    /// Individual values clamped.
    pub clamped: u64,
    /// Invalid calls, including plays with bad values.
    pub bad_values: u64,
    /// Output frames with either sample at or above the clip knee.
    pub clipped_frames: u64,
    /// Maximum occupied command slots.
    pub queue_high_water: usize,
    /// Frames rendered while pulling.
    pub frames: u64,
    /// Current output epoch.
    pub epoch: u64,
    /// Whether a pull can render frames.
    pub pulling: bool,
    pub(crate) fates: [u64; 8],
}

impl Record {
    /// Cumulative number of plays with this fate.
    pub fn plays(&self, fate: Fate) -> u64 {
        self.fates[fate as usize]
    }
}
