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

/// What content reports at one frame boundary while [`await_content`] waits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Progress {
    /// Not drawn yet; the generations staged and drawn so far, when known.
    Waiting {
        /// The newest staged generation.
        staged: Option<u64>,
        /// The drawn generation, if one other than the wanted one is drawn.
        drawn: Option<u64>,
    },
    /// This generation is drawn.
    Drawn(u64),
    /// The content failed; its newest messages.
    Failed(Vec<String>),
}

/// The one wait for content to be drawn: call `probe`, and while it reports
/// [`Progress::Waiting`] (or a drawn generation `wanted` does not accept),
/// call `draw` to draw one frame and probe again. A [`Progress::Failed`]
/// fails the wait, as does [`DRAWN_DEADLINE`]. Content already drawn returns
/// without drawing. Hosts whose content reports differently supply their own
/// probe; [`await_drawn`] is this wait over cooked statistics and endpoint frames.
/// # Errors
/// Returns the content failure, the deadline, or the first `draw` or `probe` error.
pub fn await_content(
    wanted: Drawn,
    mut draw: impl FnMut() -> Result<(), AwaitDrawnError>,
    mut probe: impl FnMut() -> Result<Progress, AwaitDrawnError>,
) -> Result<u64, AwaitDrawnError> {
    let deadline = Instant::now() + DRAWN_DEADLINE;
    loop {
        let (staged, drawn) = match probe()? {
            Progress::Failed(errors) => return Err(AwaitDrawnError::Content(errors)),
            Progress::Drawn(generation) => {
                if let Some(generation) = wanted.accepts(Some(generation)) {
                    return Ok(generation);
                }
                (None, Some(generation))
            }
            Progress::Waiting { staged, drawn } => (staged, drawn),
        };
        if Instant::now() >= deadline {
            return Err(AwaitDrawnError::Deadline {
                wanted,
                staged,
                drawn,
            });
        }
        draw()?;
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Draw the host's committed endpoint once with [`FrameHost::render_once`] and
/// wait for the device to finish it, reading nothing back: a frame boundary at
/// which contributions record staged uploads.
/// # Errors
/// Returns a frame or device fault.
pub fn draw_endpoint<W: FrameWorld, V: SceneSource<W::Page>>(
    host: &mut FrameHost<W, V>,
) -> Result<(), AwaitDrawnError> {
    let submitted = host.render_once().map_err(AwaitDrawnError::Frame)?;
    host.device_capabilities()
        .site()
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(Duration::from_secs(10)),
        })
        .map_err(|error| AwaitDrawnError::Device(format!("{error:?}")))?;
    Ok(())
}

/// The progress of the cooked contribution behind `stats`. Before any set has
/// ever drawn every loader failure counts; after, only one beyond `failures`,
/// the count when the caller began waiting, since the drawn set survives a
/// failed later generation and a device replacement redraws it.
/// # Errors
/// Returns [`AwaitDrawnError::Poisoned`] if the observations are poisoned.
pub fn cooked_progress(
    stats: &Mutex<CookedStats>,
    failures: Option<u64>,
) -> Result<Progress, AwaitDrawnError> {
    let stats = stats.lock().map_err(|_| AwaitDrawnError::Poisoned)?;
    if stats.failures != failures.unwrap_or(0) {
        return Ok(Progress::Failed(stats.errors.clone()));
    }
    Ok(match stats.drawn {
        Some(generation) => Progress::Drawn(generation),
        None => Progress::Waiting {
            staged: stats.staged,
            drawn: None,
        },
    })
}

/// The failure count [`cooked_progress`] compares against: `None` before any
/// set has ever drawn, so every failure counts.
/// # Errors
/// Returns [`AwaitDrawnError::Poisoned`] if the observations are poisoned.
pub fn cooked_failure_baseline(stats: &Mutex<CookedStats>) -> Result<Option<u64>, AwaitDrawnError> {
    let stats = stats.lock().map_err(|_| AwaitDrawnError::Poisoned)?;
    Ok((stats.drawn.is_some() || !stats.swaps.is_empty()).then_some(stats.failures))
}

/// Draw the host's committed endpoint until the contribution behind `stats`
/// has drawn `wanted`, and return that generation: [`await_content`] with
/// [`draw_endpoint`] and [`cooked_progress`]. Nothing is read back.
/// # Errors
/// Returns the first loader failure, the deadline, or a frame or device fault.
pub fn await_drawn<W: FrameWorld, V: SceneSource<W::Page>>(
    host: &mut FrameHost<W, V>,
    stats: &Mutex<CookedStats>,
    wanted: Drawn,
) -> Result<u64, AwaitDrawnError> {
    let failures = cooked_failure_baseline(stats)?;
    await_content(
        wanted,
        || draw_endpoint(host),
        || cooked_progress(stats, failures),
    )
}

#[cfg(test)]
mod tests {
    use super::{AwaitDrawnError, Drawn, Progress, await_content};

    #[test]
    fn a_probe_drawn_after_frames_returns_its_generation_after_drawing_them() {
        let mut frames = 0;
        let mut polls = 0;
        let generation = await_content(
            Drawn::Any,
            || {
                frames += 1;
                Ok(())
            },
            || {
                polls += 1;
                Ok(if polls > 3 {
                    Progress::Drawn(7)
                } else {
                    Progress::Waiting {
                        staged: Some(7),
                        drawn: None,
                    }
                })
            },
        )
        .expect("drawn");
        assert_eq!((generation, frames), (7, 3));
    }

    #[test]
    fn a_failed_probe_ends_the_wait_without_drawing() {
        let mut frames = 0;
        let error = await_content(
            Drawn::Any,
            || {
                frames += 1;
                Ok(())
            },
            || Ok(Progress::Failed(vec!["missing mesh".to_owned()])),
        )
        .expect_err("failed content");
        assert!(
            matches!(&error, AwaitDrawnError::Content(errors) if errors == &["missing mesh"]),
            "{error:?}"
        );
        assert_eq!(frames, 0);
    }

    #[test]
    fn a_drawn_generation_other_than_the_wanted_one_keeps_drawing() {
        let frames = std::cell::Cell::new(0);
        let generation = await_content(
            Drawn::Generation(2),
            || {
                frames.set(frames.get() + 1);
                Ok(())
            },
            || Ok(Progress::Drawn(if frames.get() < 2 { 1 } else { 2 })),
        )
        .expect("drawn");
        assert_eq!((generation, frames.get()), (2, 2));
    }
}
