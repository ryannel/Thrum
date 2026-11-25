//! AppKit owns the per-window display record and the retained presentation layer.
use objc2::{Message as _, rc::Retained, sel};
use objc2_app_kit::NSView;
use objc2_foundation::{MainThreadMarker, NSObjectProtocol as _};
use objc2_quartz_core::{CADynamicRangeHigh, CAMetalLayer};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
pub(super) mod proof;

use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

/// Current and potential headroom published atomically as one display sample.
#[derive(Default)]
pub(super) struct Headroom(AtomicU64);

impl Headroom {
    fn publish(&self, current: f32, potential: f32) {
        let bits = u64::from(current.to_bits()) | (u64::from(potential.to_bits()) << 32);
        self.0.store(bits, Ordering::Release);
    }

    pub(super) fn load(&self) -> (f32, f32) {
        let bytes = self.0.load(Ordering::Acquire).to_le_bytes();
        let [a, b, c, d, e, f, g, h] = bytes;
        (
            f32::from_le_bytes([a, b, c, d]).max(1.0),
            f32::from_le_bytes([e, f, g, h]).max(1.0),
        )
    }
}

pub(super) struct MacDisplay {
    view: Retained<NSView>,
    layer: Retained<CAMetalLayer>,
    pub(super) headroom: Arc<Headroom>,
    applied: Option<f32>,
    forced: Option<DisplayOverride>,
    proof: Option<std::fs::File>,
}

/// Typed screen record override. Production uses the window's real screen when absent.
#[derive(Clone, Copy, Debug)]
pub enum DisplayOverride {
    /// A display with a current and potential headroom of 1.
    Sdr,
    /// A display with constant current headroom and potential headroom of 16.
    Constant(f32),
    /// A change to the current and potential headroom on a particular display tick.
    Step {
        /// Current headroom before the change.
        before: f32,
        /// Current headroom after the change.
        after: f32,
        /// Display tick that first observes the change.
        tick: u64,
        /// Potential headroom after the change.
        potential: f32,
    },
    /// A move from an SDR screen to one with EDR.
    Promote {
        /// Display tick of the move.
        tick: u64,
    },
}

/// An invalid macOS display override.
#[derive(Debug)]
pub enum DisplayOverrideError {
    /// The supplied text has an unsupported form, headroom or tick.
    InvalidValue(String),
}

impl std::fmt::Display for DisplayOverrideError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidValue(value) => write!(formatter, "invalid display override {value:?}"),
        }
    }
}

impl std::error::Error for DisplayOverrideError {}

impl std::str::FromStr for DisplayOverride {
    type Err = DisplayOverrideError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let number = |text: &str| {
            text.parse::<f32>()
                .ok()
                .filter(|n| (1.0..=16.0).contains(n))
        };
        let forced = match value.split(':').collect::<Vec<_>>().as_slice() {
            ["sdr"] => Some(Self::Sdr),
            ["promote", tick] => tick.parse().ok().map(|tick| Self::Promote { tick }),
            ["step", before, after, tick, potential] => number(before)
                .zip(number(after))
                .zip(tick.parse().ok())
                .zip(number(potential))
                .map(|(((before, after), tick), potential)| Self::Step {
                    before,
                    after,
                    tick,
                    potential,
                }),
            [headroom] => number(headroom).map(Self::Constant),
            _ => None,
        };
        forced.ok_or_else(|| DisplayOverrideError::InvalidValue(value.to_owned()))
    }
}

/// Launch options for qualifying the macOS display path.
#[derive(Clone, Debug)]
pub struct WindowDisplayOptions {
    /// Overrides the live screen limits while retaining them as a witness.
    pub forced: DisplayOverride,
    /// Existing output directory for display and render-owner records.
    pub output: Option<std::path::PathBuf>,
    /// Include submitted drawable extrema from the engine tap.
    pub capture: bool,
    /// Retain the first presented capture's raw pixels beside the capture record.
    pub capture_first_pixels: bool,
}

impl DisplayOverride {
    fn sample(self, tick: u64) -> (f32, f32) {
        match self {
            Self::Constant(current) => (current, 16.0),
            Self::Step {
                after,
                tick: change,
                potential,
                ..
            } if tick >= change => (after, potential),
            Self::Step { before, .. } => (before, 16.0),
            Self::Promote { tick: change } if tick >= change => (2.0, 16.0),
            Self::Sdr | Self::Promote { .. } => (1.0, 1.0),
        }
    }
}

impl MacDisplay {
    #[expect(
        unsafe_code,
        reason = "the winit AppKit handle is a main-thread NSView pointer"
    )]
    pub(super) fn attach(
        window: &Window,
        options: Option<&WindowDisplayOptions>,
    ) -> Result<Self, String> {
        let _main = MainThreadMarker::new().ok_or("display must be read on AppKit main thread")?;
        let handle = window.window_handle().map_err(|error| error.to_string())?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return Err("macOS window did not provide an NSView".into());
        };
        // SAFETY: The handle is valid while the winit window is borrowed and names an NSView.
        let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() }.retain();
        let backing = view.layer().ok_or("Metal view has no backing layer")?;
        let layer = match backing.downcast::<CAMetalLayer>() {
            Ok(layer) => layer,
            Err(backing) => {
                // SAFETY: This is the live AppKit backing layer on the main thread.
                unsafe { backing.sublayers() }
                    .and_then(|layers| {
                        layers.iter().find_map(|layer| {
                            layer
                                .downcast_ref::<CAMetalLayer>()
                                .map(CAMetalLayer::retain)
                        })
                    })
                    .ok_or("Metal view has no CAMetalLayer")?
            }
        };
        let proof = options
            .and_then(|options| options.output.as_ref())
            .map(|output| std::fs::File::create(output.join("display.jsonl")))
            .transpose()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            view,
            layer,
            headroom: Arc::new(Headroom::default()),
            applied: None,
            forced: options.map(|options| options.forced),
            proof,
        })
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "EDR uniform consumes f32, and headroom is bounded to float drawable range"
    )]
    pub(super) fn sample(&mut self, tick: u64, backing_scale: f64) {
        let (current, potential, refresh) = self
            .view
            .window()
            .and_then(|window| window.screen())
            .map_or((1.0, 1.0, 0), |screen| {
                let current = screen.maximumExtendedDynamicRangeColorComponentValue();
                let potential = screen.maximumPotentialExtendedDynamicRangeColorComponentValue();
                (
                    current.clamp(1.0, 65504.0) as f32,
                    potential.clamp(1.0, 65504.0) as f32,
                    screen.maximumFramesPerSecond(),
                )
            });
        let (effective, available) = self
            .forced
            .map_or((current, potential), |forced| forced.sample(tick));
        self.headroom.publish(effective.min(available), available);
        if let Some(record) = &mut self.proof {
            use std::io::Write as _;
            let receipt = serde_json::json!({
                "tick": tick, "real_current": current, "real_potential": potential,
                "refresh_hz": refresh, "backing_scale": backing_scale,
                "current": effective.min(available), "potential": available, "forced": self.forced.is_some(),
            });
            drop(writeln!(record, "{receipt}"));
        }
    }

    #[expect(
        unsafe_code,
        reason = "the availability-checked AppKit EDR range constant is an Objective-C static"
    )]
    pub(super) fn configured(&mut self, edr: bool) {
        if !edr {
            self.applied = None;
            return;
        }
        let (headroom, _) = self.headroom.load();
        if !self
            .layer
            .respondsToSelector(sel!(setPreferredDynamicRange:))
            || !self.layer.respondsToSelector(sel!(setContentsHeadroom:))
        {
            return;
        }
        if self.applied.is_none() {
            // SAFETY: The matching selector establishes these OS 26 APIs are present.
            self.layer
                .setPreferredDynamicRange(unsafe { CADynamicRangeHigh });
        }
        if self
            .applied
            .is_none_or(|previous| previous.to_bits() != headroom.to_bits())
        {
            self.layer.setContentsHeadroom(f64::from(headroom));
            self.applied = Some(headroom);
        }
        if let Some(record) = &mut self.proof {
            use std::io::Write as _;
            let range = self.layer.preferredDynamicRange();
            // SAFETY: Both OS 26 selectors were checked above.
            let preferred_high = &*range == unsafe { CADynamicRangeHigh };
            let receipt = serde_json::json!({
                "main_thread": MainThreadMarker::new().is_some(),
                "layer_headroom": self.layer.contentsHeadroom(),
                "preferred_high": preferred_high,
                "legacy_wants_edr": Self::legacy_wants_edr(&self.layer),
            });
            drop(writeln!(record, "{receipt}"));
        }
    }

    fn legacy_wants_edr(layer: &CAMetalLayer) -> bool {
        layer.wantsExtendedDynamicRangeContent()
    }
}
