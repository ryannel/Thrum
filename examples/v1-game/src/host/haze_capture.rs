//! The diagnostic haze takes use identical elapsed-time camera and Sun samples
//! in the native and browser capture drivers.

#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum Motion {
    Pan,
    Walk,
    Cut,
    Scrub,
    BankWalk,
    BankPan,
    MistWalk,
    MistPan,
    Mover,
    Climb,
    Approach,
}

#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
#[derive(Clone, Copy, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HazeTake {
    eye: [f32; 3],
    solar_hours: f32,
    yaw_degrees: f32,
    motion: Option<Motion>,
}

#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
#[derive(Clone, Copy, serde::Serialize)]
pub(crate) struct HazePose {
    pub(crate) eye: [f32; 3],
    pub(crate) solar_hours: f32,
    pub(crate) yaw_degrees: f32,
    pub(crate) cut: bool,
}

#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
impl HazeTake {
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let take: Self = serde_json::from_str(text).map_err(|error| error.to_string())?;
        if !take.eye.iter().all(|lane| lane.is_finite())
            || !(0.0..=100_000.0).contains(&take.eye[1])
            || !(0.0..=24.0).contains(&take.solar_hours)
            || !(-125.0..=125.0).contains(&take.yaw_degrees)
        {
            return Err(
                "haze capture requires a finite eye, valid solar hour and yaw within ±125°"
                    .to_owned(),
            );
        }
        Ok(take)
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn runs_world(self) -> bool {
        matches!(self.motion, Some(Motion::Mover))
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn duration(self) -> Option<f64> {
        match self.motion {
            Some(Motion::MistPan | Motion::Climb | Motion::Approach) => Some(12.0),
            _ => None,
        }
    }

    pub(crate) fn at(self, elapsed_seconds: f32) -> HazePose {
        let progress = ((elapsed_seconds - 1.0) / 5.0).clamp(0.0, 1.0);
        let mut pose = HazePose {
            eye: self.eye,
            solar_hours: self.solar_hours,
            yaw_degrees: self.yaw_degrees,
            cut: false,
        };
        match self.motion {
            Some(Motion::Pan) => pose.yaw_degrees += 40.0 * progress,
            Some(Motion::Walk) => pose.eye[0] += 40.0 * progress,
            Some(Motion::BankWalk) => pose.eye[0] += 98.0 * progress,
            Some(Motion::BankPan) => pose.yaw_degrees += 95.0 * progress,
            Some(Motion::Climb) => {
                let climb = ((elapsed_seconds - 1.0) / 10.0).clamp(0.0, 1.0);
                // Follow the broad western flank of the east hill.
                let height = 108.0 * climb * climb * (3.0 - 2.0 * climb);
                pose.eye[1] += height + 12.0 * (std::f32::consts::PI * climb).sin();
                pose.eye[2] -= 1000.0 * climb;
            }
            Some(Motion::Approach) => {
                let walk = ((elapsed_seconds - 1.0) / 10.0).clamp(0.0, 1.0);
                pose.eye[2] -= 40.0 * walk;
            }
            Some(Motion::MistWalk) => pose.eye[0] += 100.0 * progress,
            Some(Motion::MistPan) => {
                pose.yaw_degrees += 90.0 * ((elapsed_seconds - 1.0) / 10.0).clamp(0.0, 1.0);
            }

            Some(Motion::Cut) if elapsed_seconds >= 3.0 => {
                // Keep the far shaft visible on both sides of the cut, at a
                // different screen position and a different camera distance.
                pose.eye[0] += 40.0;
                pose.eye[2] += 180.0;
                pose.yaw_degrees -= 24.0;
                pose.cut = true;
            }
            Some(Motion::Scrub) => pose.solar_hours = 16.6 + 2.2 * progress,
            _ => {}
        }
        pose
    }
}
