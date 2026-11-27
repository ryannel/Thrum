//! Output policy and the AppKit full-screen hold, independent of the render owner.

use std::time::Duration;
use thrum_render::{
    runtime::{ExtentRequest, SceneKind},
    synthetic::RenderScale,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct OutputScale {
    pub(super) step: RenderScale,
    pub(super) forced: bool,
}

pub(super) fn resolve(
    kind: SceneKind,
    fullscreen: bool,
    panel_hz: f32,
    registry: RenderScale,
    forced: Option<RenderScale>,
) -> OutputScale {
    let (step, forced) = match (kind, forced) {
        (SceneKind::ThreeD, Some(step)) => (step, true),
        (SceneKind::ThreeD, None) if fullscreen && panel_hz > 60.0 => (registry, false),
        (SceneKind::TwoD, _) | (SceneKind::ThreeD, None) => (RenderScale::Native, false),
    };
    OutputScale { step, forced }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct OutputRequest {
    pub(super) backing: [u32; 2],
    pub(super) scale: OutputScale,
}

impl OutputRequest {
    pub(super) fn extent(self) -> [u32; 2] {
        ExtentRequest::scaled_extent(self.backing, self.scale.step)
    }

    pub(super) fn generation(
        self,
        backing_scale: u64,
        format: wgpu::TextureFormat,
        device: u64,
    ) -> ExtentRequest {
        ExtentRequest {
            extent: self.backing,
            output_scale: self.scale.step,
            output_scale_forced: self.scale.forced,
            backing_scale,
            format,
            device,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ModeEvent {
    Start(bool),
    Complete(bool),
}

#[derive(Default)]
pub(super) struct ModeChange {
    fullscreen: bool,
    held_until: Option<Duration>,
    published: Option<OutputRequest>,
    publish_completion: bool,
}

impl ModeChange {
    const LIMIT: Duration = Duration::from_secs(3);

    pub(super) fn fullscreen(&self) -> bool {
        self.fullscreen
    }

    pub(super) fn held(&self) -> bool {
        self.held_until.is_some()
    }

    pub(super) fn expire(
        &mut self,
        now: Duration,
        reported_mode: impl FnOnce() -> bool,
    ) -> Option<bool> {
        if self.held_until.is_some_and(|at| now >= at) {
            let fullscreen = reported_mode();
            self.event(ModeEvent::Complete(fullscreen), now);
            Some(fullscreen)
        } else {
            None
        }
    }

    pub(super) fn deadline(&self) -> Option<Duration> {
        self.held_until
    }

    pub(super) fn event(&mut self, event: ModeEvent, now: Duration) {
        match event {
            ModeEvent::Start(_) => self.held_until = Some(now + Self::LIMIT),
            ModeEvent::Complete(fullscreen) => {
                self.publish_completion |= self.held_until.take().is_some();
                self.fullscreen = fullscreen;
            }
        }
    }

    /// During a hold this is also the pair a display-format or device change uses.
    pub(super) fn current(&self) -> Option<OutputRequest> {
        self.published
    }

    pub(super) fn observe(
        &mut self,
        backing: [u32; 2],
        scale: impl FnOnce(bool) -> OutputScale,
    ) -> Option<OutputRequest> {
        if self.held() {
            return None;
        }
        let next = OutputRequest {
            backing,
            scale: scale(self.fullscreen),
        };
        if !std::mem::take(&mut self.publish_completion) && self.published == Some(next) {
            return None;
        }
        self.published = Some(next);
        Some(next)
    }
}

#[cfg(test)]
mod tests;
