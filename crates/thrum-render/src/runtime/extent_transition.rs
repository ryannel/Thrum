//! Debounce and single-owner preparation for a native surface generation.

use super::{FrameError, PreparedExtent, SurfaceSwitch};
use crate::synthetic::RenderScale;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

const DEBOUNCE: Duration = Duration::from_millis(100);

/// The platform facts a detached build must still match when it is taken.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtentRequest {
    /// Window backing grid.
    pub extent: [u32; 2],
    /// Prepared output scale applied to the backing grid.
    pub output_scale: RenderScale,
    /// Backing scale, expressed as the platform's f64 bits.
    pub backing_scale: u64,
    /// Display output format.
    pub format: wgpu::TextureFormat,
    /// Device generation on which to allocate.
    pub device: u64,
}

impl ExtentRequest {
    /// The drawable and scene allocation grid. A hidden window stays empty.
    #[must_use]
    pub fn output_extent(self) -> [u32; 2] {
        Self::scaled_extent(self.extent, self.output_scale)
    }

    /// Apply the prepared output step to a backing grid before a format is known.
    #[must_use]
    pub fn scaled_extent(backing: [u32; 2], scale: RenderScale) -> [u32; 2] {
        if backing.contains(&0) {
            backing
        } else {
            scale.extent(backing)
        }
    }
}

struct Build {
    request: ExtentRequest,
    prepares_candidate: bool,
    cancelled: Arc<AtomicBool>,
    job: JoinHandle<Result<Option<PreparedExtent>, FrameError>>,
}

/// The latest resize request, its optional preparation job and the surface
/// admission record. Call it from the render owner, at frame boundaries.
pub struct ExtentTransition {
    latest: ExtentRequest,
    changed: Duration,
    delay: Duration,
    build: Option<Build>,
    ready: Option<PreparedExtent>,
    discarded: Vec<PreparedExtent>,
    attempted: bool,
    /// Eligibility and call counts for the current switch.
    pub surface: SurfaceSwitch,
}

impl ExtentTransition {
    /// Start with the generation configured at attach.
    #[must_use]
    pub fn new(current: ExtentRequest) -> Self {
        Self {
            latest: current,
            changed: Duration::ZERO,
            delay: Duration::ZERO,
            build: None,
            ready: None,
            discarded: Vec::new(),
            attempted: true,
            surface: SurfaceSwitch::configured(),
        }
    }

    /// Cancel pending work on loss or shutdown.
    pub fn cancel(&mut self) {
        self.discarded.extend(self.ready.take());
        if let Some(build) = &self.build {
            build.cancelled.store(true, Ordering::Release);
        }
    }

    /// The new device's initial surface and generation were installed together.
    pub fn recovered(&mut self, request: ExtentRequest, configured: bool) {
        self.cancel();
        self.latest = request;
        self.attempted = true;
        self.surface = SurfaceSwitch::with_eligibility(configured);
    }

    /// Whether a preparation job still owns its inputs.
    #[must_use]
    pub fn busy(&self) -> bool {
        self.build.is_some()
    }

    /// Whether frames must exclude the detached build or its deferred switch.
    #[must_use]
    pub fn holds_governor(&self) -> bool {
        self.build
            .as_ref()
            .is_some_and(|build| build.prepares_candidate)
            || self.ready.is_some()
    }

    /// Take the candidate out for admission at this boundary.
    pub fn ready(&mut self) -> Option<PreparedExtent> {
        self.ready.take()
    }

    /// Keep an admitted candidate available after a deferral.
    pub fn defer(&mut self, ready: PreparedExtent) {
        self.ready = Some(ready);
    }

    /// Discard an unbound candidate on the preparation thread.
    pub fn discard(&mut self, ready: PreparedExtent) {
        self.discarded.push(ready);
    }

    /// Repeat preparation after the host's scene or capture inputs changed.
    pub fn refresh(&mut self) {
        self.attempted = false;
    }

    /// Update the latest request without constructing resources, including when
    /// a hidden window reports a zero extent.
    pub fn observe(&mut self, request: ExtentRequest, current: ExtentRequest, now: Duration) {
        if request != self.latest {
            let previous = self.latest;
            self.cancel();
            self.latest = request;
            self.changed = now;
            self.delay = if request.backing_scale == previous.backing_scale
                && request.format == previous.format
                && request.device == previous.device
            {
                DEBOUNCE
            } else {
                Duration::ZERO
            };
            self.surface.begin();
            self.attempted = request == current && self.surface.usable();
        }
    }

    /// Observe the request and advance detached preparation without waiting.
    /// An unchanged failed request is attempted only once.
    /// # Errors
    /// Returns the preparation error for the latest request.
    pub fn advance<Job>(
        &mut self,
        request: ExtentRequest,
        current: ExtentRequest,
        now: Duration,
        inputs: impl FnOnce() -> Result<Job, FrameError>,
    ) -> Result<(), FrameError>
    where
        Job: FnOnce(&AtomicBool) -> Result<Option<PreparedExtent>, FrameError> + Send + 'static,
    {
        self.observe(request, current, now);
        if self
            .build
            .as_ref()
            .is_some_and(|build| build.job.is_finished())
            && let Some(build) = self.build.take()
        {
            let result = build.job.join().map_err(|_| {
                FrameError::gpu(
                    "extent preparation",
                    crate::error::FrameGpuError::ExtentWorkerStopped,
                )
            });
            if build.request == request && !build.cancelled.load(Ordering::Acquire) {
                self.ready = result??;
            } else if let Ok(Ok(Some(obsolete))) = result {
                self.discard(obsolete);
            }
        }
        if self.build.is_some() {
            return Ok(());
        }
        let may_build = !self.attempted
            && !request.extent.contains(&0)
            && now.saturating_sub(self.changed) >= self.delay;
        let inputs = if may_build {
            self.attempted = true;
            Some(inputs()?)
        } else {
            None
        };
        if inputs.is_none() && self.discarded.is_empty() {
            return Ok(());
        }
        let prepares_candidate = inputs.is_some();
        let discarded = std::mem::take(&mut self.discarded);
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancelled);
        let job = thread::Builder::new()
            .name("thrum extent preparation".into())
            .spawn(move || {
                drop(discarded);
                inputs.map_or(Ok(None), |build| build(&flag))
            })
            .map_err(|error| FrameError::gpu("start extent preparation", error))?;
        self.build = Some(Build {
            request,
            prepares_candidate,
            cancelled,
            job,
        });
        Ok(())
    }
}

impl Drop for ExtentTransition {
    fn drop(&mut self) {
        self.cancel();
    }
}
