//! Wait for cooked content to be drawn, rendering without reading frames back.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use thrum_render::runtime::{FrameError, FrameHost, FrameWorld, SceneSource};

use crate::CookedStats;

/// How long [`await_drawn`] waits before it fails. Content that takes longer is
/// a fault to report, not a wait to extend at one call site.
pub const DRAWN_DEADLINE: Duration = Duration::from_secs(90);

/// Which cooked generation counts as drawn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Drawn {
    /// Any generation.
    Any,
    /// Exactly this generation.
    Generation(u64),
}

impl Drawn {
    fn accepts(self, drawn: Option<u64>) -> Option<u64> {
        match (self, drawn) {
            (Self::Any, Some(generation)) => Some(generation),
            (Self::Generation(wanted), Some(generation)) if generation == wanted => {
                Some(generation)
            }
            _ => None,
        }
    }
}

/// Why cooked content was not drawn.
#[derive(Debug)]
pub enum AwaitDrawnError {
    /// The loader recorded a failure while the caller waited; its newest messages.
    Content(Vec<String>),
    /// [`DRAWN_DEADLINE`] passed first.
    Deadline {
        /// What the caller waited for.
        wanted: Drawn,
        /// The newest staged generation when the wait ended.
        staged: Option<u64>,
        /// The drawn generation when the wait ended.
        drawn: Option<u64>,
    },
    /// The frame host could not draw or submit a frame.
    Frame(FrameError),
    /// The device did not complete a submitted frame.
    Device(String),
    /// A thread holding the observations panicked.
    Poisoned,
}

impl std::fmt::Display for AwaitDrawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Content(errors) => write!(f, "cooked content failed: {errors:?}"),
            Self::Deadline {
                wanted,
                staged,
                drawn,
            } => write!(
                f,
                "cooked content {wanted:?} was not drawn within {} s (staged {staged:?}, drawn {drawn:?})",
                DRAWN_DEADLINE.as_secs()
            ),
            Self::Frame(error) => write!(f, "cooked content frame: {error}"),
            Self::Device(error) => write!(f, "cooked content device wait: {error}"),
            Self::Poisoned => f.write_str("cooked content observations poisoned"),
        }
    }
}

impl std::error::Error for AwaitDrawnError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Frame(error) => Some(error),
            _ => None,
        }
    }
}

impl From<AwaitDrawnError> for String {
    fn from(error: AwaitDrawnError) -> Self {
        error.to_string()
    }
}

/// Draw the host's committed endpoint until the contribution behind `stats`
/// has drawn `wanted`, and return that generation. Each frame is submitted
/// with [`FrameHost::render_once`] and waited for on the device; nothing is
/// read back. A loader failure fails the wait, as does [`DRAWN_DEADLINE`]:
/// any failure before the first set ever draws and, after it, a failure
/// recorded during this wait. Content already drawn returns without
/// rendering.
/// # Errors
/// Returns the first loader failure, the deadline, or a frame or device fault.
pub fn await_drawn<W: FrameWorld, V: SceneSource<W::Page>>(
    host: &mut FrameHost<W, V>,
    stats: &Mutex<CookedStats>,
    wanted: Drawn,
) -> Result<u64, AwaitDrawnError> {
    let deadline = Instant::now() + DRAWN_DEADLINE;
    // Before any set has ever drawn every loader failure counts; after, only one recorded
    // during this wait, since the drawn set survives a failed later generation and a device
    // replacement redraws it.
    let failures = {
        let stats = stats.lock().map_err(|_| AwaitDrawnError::Poisoned)?;
        if stats.drawn.is_none() && stats.swaps.is_empty() {
            0
        } else {
            stats.failures
        }
    };
    loop {
        {
            let stats = stats.lock().map_err(|_| AwaitDrawnError::Poisoned)?;
            if stats.failures != failures {
                return Err(AwaitDrawnError::Content(stats.errors.clone()));
            }
            if let Some(generation) = wanted.accepts(stats.drawn) {
                return Ok(generation);
            }
            if Instant::now() >= deadline {
                return Err(AwaitDrawnError::Deadline {
                    wanted,
                    staged: stats.staged,
                    drawn: stats.drawn,
                });
            }
        }
        let submitted = host.render_once().map_err(AwaitDrawnError::Frame)?;
        host.device_capabilities()
            .site()
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(Duration::from_secs(10)),
            })
            .map_err(|error| AwaitDrawnError::Device(format!("{error:?}")))?;
        std::thread::sleep(Duration::from_millis(2));
    }
}
