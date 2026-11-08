//! The V1 demo's typed controls. One declaration serves the macOS panel, the browser page
//! and parameter files, with the same keys, labels and units everywhere.
//!
//! The model only records what a person or file asked for. The game applies those edits
//! through its checked host setters ([`GameControls::take_edits`]) and writes back what the
//! host actually shows ([`GameControls::observe`]), so the clock advancing or a key press
//! never fights a pending panel edit.

use std::f32::consts::{PI, TAU};

use thrum_controls::{ControlError, ControlId, Controls, Display, Kind, Readout, Slider, Toggle};
use thrum_render::synthetic::DisplayOverlay;

use crate::environment::DayCycle;

/// Steepest camera pitch the demo offers, in whole degrees so every view's slider has
/// integer stops. The host itself admits up to ±0.6 rad.
const PITCH_LIMIT_DEGREES: f32 = 34.0;

/// Edits made since the game last applied the controls. `None` leaves that value alone.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ControlEdits {
    /// Selected solar time as a day fraction (midnight 0, noon 0.5).
    pub time_of_day: Option<f32>,
    /// Camera pitch in radians.
    pub camera_pitch: Option<f32>,
    /// Camera yaw in radians.
    pub camera_yaw: Option<f32>,
    /// The person's pause preference.
    pub paused: Option<bool>,
    /// Whether exposure zones replace the tone-mapped scene.
    pub exposure_zones: Option<bool>,
}

impl ControlEdits {
    /// Whether anything needs applying.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The display overlay the exposure-zones edit selects, if it changed.
    #[must_use]
    pub fn display_overlay(&self) -> Option<DisplayOverlay> {
        self.exposure_zones.map(|on| {
            if on {
                DisplayOverlay::ExposureZones
            } else {
                DisplayOverlay::Off
            }
        })
    }
}

#[derive(Clone, Copy)]
struct Stops {
    min: f32,
    max: f32,
    step: f32,
    display: Display,
}

/// The demo's controls and the bookkeeping that keeps edits and observations apart.
pub struct GameControls {
    model: Controls,
    solar_time: Slider,
    yaw: Slider,
    pitch: Slider,
    paused: Toggle,
    exposure_zones: Toggle,
    sun_elevation: Readout,
    /// Per input control: the model revision last applied to or observed from the game.
    /// A control whose revision is newer holds a pending edit.
    settled: [u64; 5],
    /// Solar time the Sun elevation readout was computed for.
    sun_for: f32,
}

impl GameControls {
    /// Declare the demo's controls at their defaults: noon, level camera, running world.
    ///
    /// # Errors
    /// Returns a declaration refusal; the keys and ranges here are fixed, so none is expected.
    pub fn new() -> Result<Self, ControlError> {
        let mut model = Controls::new();
        let solar_time = model
            .slider("sky.solar_time", "Solar time", 0.0..=1.0)
            .default(0.5)
            // 0.01 h, the stop the browser page has always used.
            .step(1.0 / 2400.0)
            .display(Display::clock())
            .add()?;
        let one_degree = 1.0_f32.to_radians();
        let yaw = model
            .slider("camera.yaw", "Look around", -PI..=PI)
            .default(0.0)
            .step(one_degree)
            .display(Display::degrees())
            .add()?;
        let pitch_limit = PITCH_LIMIT_DEGREES.to_radians();
        let pitch = model
            .slider("camera.pitch", "Look up/down", -pitch_limit..=pitch_limit)
            .default(0.0)
            .step(one_degree)
            .display(Display::degrees())
            .add()?;
        let paused = model.toggle("world.paused", "Pause world", false)?;
        let exposure_zones = model.toggle("display.exposure_zones", "Exposure zones", false)?;
        let sun_elevation =
            model.readout("sky.sun_elevation", "Sun elevation", Display::degrees())?;
        let mut controls = Self {
            model,
            solar_time,
            yaw,
            pitch,
            paused,
            exposure_zones,
            sun_elevation,
            settled: [0; 5],
            sun_for: f32::NAN,
        };
        controls.show_sun(0.5);
        controls.settle_all();
        Ok(controls)
    }

    /// The typed model that views build from and edit.
    #[must_use]
    pub fn model(&self) -> &Controls {
        &self.model
    }

    /// Mutable model for views applying a person's edits and for parameter files.
    pub fn model_mut(&mut self) -> &mut Controls {
        &mut self.model
    }

    /// Write back the camera orientation the game shows, as `[pitch, yaw]` radians, such
    /// as the one the scene authored. A pending edit is kept.
    pub fn observe_camera(&mut self, [pitch, yaw]: [f32; 2]) {
        self.observe_slider(1, self.yaw, yaw);
        self.observe_slider(2, self.pitch, pitch);
    }

    /// Write back the committed solar time and its Sun elevation. A pending edit is kept, and
    /// values within half a stop are ignored, so views update once per stop rather than
    /// once per tick.
    pub fn observe_time(&mut self, time_of_day: f32) {
        self.observe_slider(0, self.solar_time, time_of_day);
        self.show_sun(self.model.get(self.solar_time));
    }

    /// Write back the pause preference the game holds. A pending edit is kept.
    pub fn observe_paused(&mut self, paused: bool) {
        if self.model.revision_of(self.paused.id()) <= self.settled[3] {
            self.model.set_flag(self.paused, paused);
            self.settled[3] = self.model.revision_of(self.paused.id());
        }
    }

    /// Edits a person or file made since the last call; each is reported once.
    pub fn take_edits(&mut self) -> ControlEdits {
        let mut edits = ControlEdits::default();
        if self.pending(0, self.solar_time.id()) {
            edits.time_of_day = Some(self.model.get(self.solar_time));
        }
        if self.pending(1, self.yaw.id()) {
            edits.camera_yaw = Some(self.model.get(self.yaw));
        }
        if self.pending(2, self.pitch.id()) {
            edits.camera_pitch = Some(self.model.get(self.pitch));
        }
        if self.pending(3, self.paused.id()) {
            edits.paused = Some(self.model.is_on(self.paused));
        }
        if self.pending(4, self.exposure_zones.id()) {
            edits.exposure_zones = Some(self.model.is_on(self.exposure_zones));
        }
        self.show_sun(self.model.get(self.solar_time));
        edits
    }

    /// Every current value as an edit, for replaying presentation state onto a
    /// replacement device.
    #[must_use]
    pub fn replay(&self) -> ControlEdits {
        ControlEdits {
            time_of_day: Some(self.model.get(self.solar_time)),
            camera_pitch: Some(self.model.get(self.pitch)),
            camera_yaw: Some(self.model.get(self.yaw)),
            paused: None,
            exposure_zones: Some(self.model.is_on(self.exposure_zones)),
        }
    }

    /// Turn the camera by `radians`, wrapping around a full circle (the Q and E keys).
    pub fn turn(&mut self, radians: f32) {
        let yaw = (self.model.get(self.yaw) + radians + PI).rem_euclid(TAU) - PI;
        self.edit(self.yaw, yaw);
    }

    /// Tilt the camera by `radians`, stopping at the demo's pitch limit (the R and F keys).
    pub fn tilt(&mut self, radians: f32) {
        self.edit(self.pitch, self.model.get(self.pitch) + radians);
    }

    /// Move the selected solar time by `hours`, wrapping around midnight (the [ and ] keys).
    pub fn advance_hours(&mut self, hours: f32) {
        let time = (self.model.get(self.solar_time) + hours / 24.0).rem_euclid(1.0);
        self.edit(self.solar_time, time);
    }

    /// A key edit behaves like a panel edit: it lands on the slider's nearest stop, and a
    /// key held against a limit simply stays there.
    fn edit(&mut self, slider: Slider, engine: f32) {
        let Some(stops) = self.stops(slider) else {
            return;
        };
        let engine = engine.clamp(stops.min, stops.max);
        // `stops` only exists for a slider, so the edit always fits.
        drop(
            self.model
                .edit_display(slider.id(), stops.display.to_display(engine)),
        );
    }

    fn stops(&self, slider: Slider) -> Option<Stops> {
        match self.model.specs().get(slider.id().index())?.kind() {
            Kind::Slider {
                min,
                max,
                step,
                display,
            } => Some(Stops {
                min: *min,
                max: *max,
                step: *step,
                display: *display,
            }),
            _ => None,
        }
    }

    fn pending(&mut self, slot: usize, id: ControlId) -> bool {
        let revision = self.model.revision_of(id);
        if revision > self.settled[slot] {
            self.settled[slot] = revision;
            true
        } else {
            false
        }
    }

    /// Store an observed slider value unless an edit is pending. Values within half a stop
    /// of the model are left alone, so a running clock updates views once per stop, not
    /// once per tick. Returns whether the model changed.
    fn observe_slider(&mut self, slot: usize, slider: Slider, value: f32) -> bool {
        if self.model.revision_of(slider.id()) > self.settled[slot] {
            return false;
        }
        let Some(stops) = self.stops(slider) else {
            return false;
        };
        if !value.is_finite() || (value - self.model.get(slider)).abs() < stops.step * 0.5 {
            return false;
        }
        // The host can hold values just outside the demo's stops, such as a pitch the
        // scene authored; the model shows the nearest one it offers.
        let changed = self
            .model
            .set(slider, value.clamp(stops.min, stops.max))
            .unwrap_or(false);
        self.settled[slot] = self.model.revision_of(slider.id());
        changed
    }

    /// Publish the Sun elevation for the solar time the controls show, whether the clock or
    /// a person set it. Recomputed only when that time changes.
    fn show_sun(&mut self, time_of_day: f32) {
        if time_of_day.to_bits() == self.sun_for.to_bits() {
            return;
        }
        self.sun_for = time_of_day;
        let elevation = DayCycle::offset_for_time(0, time_of_day).map_or(0.0, |offset| {
            DayCycle::sample(0, 0.0, offset).toward_sun[1]
                .clamp(-1.0, 1.0)
                .asin()
        });
        self.model.show(self.sun_elevation, elevation);
    }

    fn settle_all(&mut self) {
        let ids = [
            self.solar_time.id(),
            self.yaw.id(),
            self.pitch.id(),
            self.paused.id(),
            self.exposure_zones.id(),
        ];
        for (slot, id) in ids.into_iter().enumerate() {
            self.settled[slot] = self.model.revision_of(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GameControls;

    #[test]
    fn edits_are_reported_once_and_observations_are_not_edits() {
        let mut controls = GameControls::new().expect("fixed declarations");
        assert!(controls.take_edits().is_empty());
        controls.observe_time(0.6);
        controls.observe_paused(false);
        assert!(controls.take_edits().is_empty(), "the clock is not an edit");
        let id = controls.model().find("sky.solar_time").expect("declared");
        controls.model_mut().edit_display(id, 18.48).unwrap();
        let edits = controls.take_edits();
        assert!((edits.time_of_day.expect("edited") - 18.48 / 24.0).abs() < 1e-6);
        assert!(controls.take_edits().is_empty());
    }

    #[test]
    fn a_pending_edit_survives_an_observation() {
        let mut controls = GameControls::new().expect("fixed declarations");
        controls.turn(90.0_f32.to_radians());
        controls.observe_camera([0.0, 0.0]);
        let yaw = controls.take_edits().camera_yaw.expect("still pending");
        assert!((yaw - 90.0_f32.to_radians()).abs() < 1e-5);
    }

    #[test]
    fn keys_wrap_time_and_yaw_and_stop_pitch() {
        let mut controls = GameControls::new().expect("fixed declarations");
        controls.advance_hours(13.0);
        controls.turn(-190.0_f32.to_radians());
        for _ in 0..10 {
            controls.tilt(5.0_f32.to_radians());
        }
        let edits = controls.take_edits();
        assert!((edits.time_of_day.expect("time") - 1.0 / 24.0).abs() < 1e-5);
        assert!((edits.camera_yaw.expect("yaw") - 170.0_f32.to_radians()).abs() < 1e-4);
        assert!((edits.camera_pitch.expect("pitch") - 34.0_f32.to_radians()).abs() < 1e-5);
        assert_eq!(
            controls.model().clamped_edits(),
            0,
            "keys stay on the stops"
        );
    }

    #[test]
    fn sun_elevation_follows_the_observed_clock() {
        let mut controls = GameControls::new().expect("fixed declarations");
        let id = controls
            .model()
            .find("sky.sun_elevation")
            .expect("declared");
        assert_eq!(
            controls.model().formatted(id),
            "45°",
            "noon elevation at −45° latitude"
        );
        controls.observe_time(0.25);
        assert_eq!(controls.model().formatted(id), "0°", "sunrise");
        let time = controls.model().find("sky.solar_time").expect("declared");
        controls.model_mut().edit_display(time, 12.0).unwrap();
        let edits = controls.take_edits();
        assert_eq!(
            controls.model().formatted(id),
            "45°",
            "an edit updates it at once"
        );
        assert!(edits.time_of_day.is_some());
        controls.model_mut().edit_display(time, 12.0).unwrap();
        controls.observe_time(0.25);
        assert_eq!(
            controls.model().formatted(id),
            "45°",
            "a person's pending edit is what the readout describes"
        );
    }

    #[test]
    fn the_example_parameter_file_applies_cleanly_and_typos_are_explained() {
        let mut controls = GameControls::new().expect("fixed declarations");
        let example = include_str!("../params/sunset.params.json");
        let report = controls
            .model_mut()
            .apply_file(example)
            .expect("a JSON object");
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        let edits = controls.take_edits();
        assert!(edits.time_of_day.is_some() && edits.camera_yaw.is_some());
        let report = controls
            .model_mut()
            .apply_file(r#"{"camera.yw": 10, "camera.pitch": 80}"#)
            .expect("a JSON object");
        let messages: Vec<String> = report.problems.iter().map(ToString::to_string).collect();
        assert!(messages[0].contains("outside -34–34"), "{}", messages[0]);
        assert!(
            messages[1].contains("did you mean \"camera.yaw\""),
            "{}",
            messages[1]
        );
    }
}
