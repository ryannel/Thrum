//! Submission-owned schedule for the Sun-following charts.

use crate::pass_key::{AtmosphereStage, REFLECTION_MIPS};

use super::AtmosphereFrame;

pub(super) const STAGES: usize = 2 + REFLECTION_MIPS as usize;
const ALL: u16 = (1 << STAGES) - 1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Inputs {
    direction: [f32; 3],
    altitude: f32,
    illuminance: f32,
    colour: [f32; 3],
    profile: u64,
    aerosol: crate::atmosphere::Aerosol,
    night: bool,
}

impl From<AtmosphereFrame> for Inputs {
    fn from(frame: AtmosphereFrame) -> Self {
        Self {
            direction: frame.toward_sun,
            altitude: frame.camera_altitude_m,
            illuminance: frame.solar_illuminance_lux,
            colour: frame.solar_colour,
            profile: frame.profile_epoch,
            aerosol: frame.aerosol.unwrap_or_default(),
            night: frame.nightglow_active(),
        }
    }
}

impl Inputs {
    fn same_sky(self, other: Self) -> bool {
        self.direction.map(f32::to_bits) == other.direction.map(f32::to_bits)
            && self.altitude.to_bits() == other.altitude.to_bits()
    }

    fn same_diffuse_light(self, other: Self) -> bool {
        self.illuminance.to_bits() == other.illuminance.to_bits()
            && self.colour.map(f32::to_bits) == other.colour.map(f32::to_bits)
    }

    fn same_air(self, other: Self) -> bool {
        self.profile == other.profile && self.aerosol == other.aerosol && self.night == other.night
    }

    fn jumped(self, other: Self) -> bool {
        let previous = glam::Vec3::from_array(other.direction);
        let next = glam::Vec3::from_array(self.direction);
        let dot = previous.dot(next);
        dot < 0.0 || previous.cross(next).length() > dot * 0.034_920_77
    }

    fn change(self, previous: Self, tuning: Tuning) -> f32 {
        let sun = glam::Vec3::from_array(self.direction);
        let old_sun = glam::Vec3::from_array(previous.direction);
        let angle = sun.cross(old_sun).length().atan2(sun.dot(old_sun));
        let relative = |new: f32, old: f32| {
            let larger = new.max(old);
            if larger > 0.0 {
                (new - old).abs() / larger
            } else {
                0.0
            }
        };
        let light = self.colour.into_iter().zip(previous.colour).fold(
            relative(self.illuminance, previous.illuminance),
            |worst, (new, old)| worst.max(relative(new, old)),
        );
        (angle / tuning.sun_tolerance(sun.y))
            .max((self.altitude - previous.altitude).abs() / tuning.altitude)
            .max(light / tuning.light)
    }
}

#[derive(Clone, Copy)]
struct Tuning {
    sun: f32,
    altitude: f32,
    light: f32,
    twilight: Option<Twilight>,
}

#[derive(Clone, Copy)]
struct Twilight {
    /// Sine of the lowest and highest elevations using the tight tolerance.
    tight_band: [f32; 2],
    /// Sine of the elevations where each side returns to the widest tolerance.
    widest_at: [f32; 2],
    sun: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            sun: 0.2_f32.to_radians(),
            altitude: 10.0,
            light: 0.01,
            twilight: Some(Twilight {
                tight_band: [-5.5_f32, 6.0].map(|degrees| degrees.to_radians().sin()),
                widest_at: [-7.0_f32, 10.0].map(|degrees| degrees.to_radians().sin()),
                sun: 0.009_f32.to_radians(),
            }),
        }
    }
}

impl Tuning {
    fn sun_tolerance(self, sun_y: f32) -> f32 {
        let Some(twilight) = self.twilight else {
            return self.sun;
        };
        let [low, high] = twilight.tight_band;
        let [night, day] = twilight.widest_at;
        let weight = if sun_y < low {
            (low - sun_y) / (low - night)
        } else if sun_y > high {
            (sun_y - high) / (day - high)
        } else {
            0.0
        };
        (self.sun - twilight.sun).mul_add(weight.clamp(0.0, 1.0), twilight.sun)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Source {
    inputs: Inputs,
    submission: u64,
    overlay_revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Refresh {
    snapshot: Source,
    spread: u32,
    cursor: u32,
    change: f32,
}

#[derive(Clone, Default)]
pub(super) struct ChartSchedule {
    last: Option<Source>,
    sources: [Option<Source>; STAGES],
    refresh: Option<Refresh>,
    cadence: u32,
    tuning: Tuning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WholeUpdate {
    Initialization,
    CameraCut,
    AirChange,
    SunJump,
    Recovery,
    Motion,
}

impl WholeUpdate {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Initialization => "initialization",
            Self::CameraCut => "camera cut",
            Self::AirChange => "air change",
            Self::SunJump => "Sun jump",
            Self::Recovery => "recovery",
            Self::Motion => "motion cut",
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct ChartSubmission {
    live: Source,
    written: Source,
    next_refresh: Option<Refresh>,
    cadence: u32,
    pub(super) position: Option<(u32, u32)>,
    pub(super) change: Option<f32>,
    writes: u16,
    pub(super) whole_update: Option<WholeUpdate>,
}

impl ChartSubmission {
    pub(super) fn is_live_snapshot(&self) -> bool {
        self.written.submission == self.live.submission
    }

    pub(super) fn writes(&self, stage: AtmosphereStage) -> Option<bool> {
        let index = match stage {
            AtmosphereStage::SkyView => 0,
            AtmosphereStage::Diffuse => 1,
            AtmosphereStage::Reflection(level) => 2 + level as usize,
            _ => return None,
        };
        Some(self.writes & (1 << index) != 0)
    }
}

impl ChartSchedule {
    pub(super) fn was_governed(&self) -> bool {
        self.cadence > 1
    }

    fn due(&self, index: usize, snapshot: Source, live: Source, overlay_light: bool) -> bool {
        self.sources[index].is_none_or(|previous| {
            !previous.inputs.same_air(snapshot.inputs)
                || !previous.inputs.same_sky(snapshot.inputs)
                || ((index == 1 || (index > 1 && overlay_light))
                    && !previous.inputs.same_diffuse_light(snapshot.inputs))
                || (index > 0 && previous.overlay_revision != live.overlay_revision)
        })
    }

    pub(super) fn plan(
        &self,
        inputs: Inputs,
        cadence: u32,
        camera_cut: bool,
        overlay_present: bool,
        overlay_written: bool,
    ) -> ChartSubmission {
        let live = Source {
            inputs,
            submission: self.last.map_or(0, |last| last.submission + 1),
            overlay_revision: self.last.map_or(0, |last| last.overlay_revision)
                + u64::from(overlay_written),
        };
        let overlay_light = cadence > 1 && overlay_present;
        let dirty = (0..STAGES).any(|index| self.due(index, live, live, overlay_light));
        // Overlay writes are discovered after the sky pass. A write this frame
        // can dirty the overlay readers, but cannot retroactively replace the
        // sky. Changes outstanding at the boundary still trigger recovery.
        let before_overlay = Source {
            overlay_revision: live.overlay_revision - u64::from(overlay_written),
            ..live
        };
        let prior_dirty =
            (0..STAGES).any(|index| self.due(index, live, before_overlay, overlay_light));
        let whole_update = match self.last {
            None => Some(WholeUpdate::Initialization),
            Some(last) if !inputs.same_air(last.inputs) => Some(WholeUpdate::AirChange),
            Some(_) if cadence > 1 && camera_cut => Some(WholeUpdate::CameraCut),
            Some(last) if cadence > 1 && inputs.jumped(last.inputs) => Some(WholeUpdate::SunJump),
            Some(_)
                if cadence > 1
                    && cadence < self.cadence
                    && (prior_dirty || self.refresh.is_some()) =>
            {
                Some(WholeUpdate::Recovery)
            }
            Some(_)
                if cadence > 1
                    && self.refresh.is_some_and(|refresh| {
                        inputs.change(refresh.snapshot.inputs, self.tuning) > 1.0
                    }) =>
            {
                Some(WholeUpdate::Motion)
            }
            _ => None,
        };
        let refresh = if cadence > 1 && whole_update.is_none() {
            self.refresh.or_else(|| {
                dirty.then(|| {
                    let change = self
                        .last
                        .map_or(0.0, |last| inputs.change(last.inputs, self.tuning));
                    let spread = (2..=cadence)
                        .rev()
                        .find(|spread| f64::from(*spread) * f64::from(change) <= 1.0)
                        .unwrap_or(1);
                    Refresh {
                        snapshot: live,
                        spread,
                        cursor: 0,
                        change,
                    }
                })
            })
        } else {
            None
        };
        let mut writes = 0;
        let mut written = live;
        let mut next_refresh = None;
        let position = refresh.map(|refresh| (refresh.spread, refresh.cursor));
        let change = refresh.map(|refresh| refresh.change);
        if let Some(mut refresh) = refresh {
            written = Source {
                overlay_revision: live.overlay_revision,
                ..refresh.snapshot
            };
            // Bound each submission by ceil(9 / spread), ending on the
            // final submission with the final reflection level.
            let count = u32::try_from(STAGES).expect("nine chart stages");
            let first = (refresh.cursor * count).div_ceil(refresh.spread);
            refresh.cursor += 1;
            let end = (refresh.cursor * count).div_ceil(refresh.spread);
            for index in first as usize..end as usize {
                if self.due(index, refresh.snapshot, live, overlay_light) {
                    writes |= 1 << index;
                }
            }
            next_refresh = (refresh.cursor < refresh.spread).then_some(refresh);
        } else if cadence == 1 || whole_update.is_some() {
            for index in 0..STAGES {
                if whole_update.is_some() || self.due(index, live, live, overlay_light) {
                    writes |= 1 << index;
                }
            }
        }
        debug_assert_eq!(writes & !ALL, 0);
        ChartSubmission {
            live,
            written,
            next_refresh,
            cadence,
            position,
            change,
            writes,
            whole_update,
        }
    }

    pub(super) fn submitted(&mut self, submission: &ChartSubmission) {
        self.last = Some(submission.live);
        self.refresh = submission.next_refresh;
        self.cadence = submission.cadence;
        for (index, source) in self.sources.iter_mut().enumerate() {
            if submission.writes & (1 << index) != 0 {
                *source = Some(submission.written);
            }
        }
    }
}

#[cfg(test)]
mod tests;
