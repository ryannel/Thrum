//! Drive an elapsed cloud inspection through the ordinary controls and camera.
use super::{HostedGame, Offscreen, WindowGame};
use crate::GameHost as _;
use serde_json::{Value, json};

#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Take {
    Sweep {
        key: String,
        from: f64,
        to: f64,
        #[serde(default)]
        logarithmic: bool,
    },
    Pan {
        from: f32,
        to: f32,
    },
    Walk,
    Lapse,
    Day,
    Preset,
    Cut,
    Profile,
}

impl Take {
    pub(super) fn requested(scenario: &str) -> Result<Option<Self>, String> {
        if scenario != "clouds-inspect" {
            return Ok(None);
        }
        let path = crate::content::launch_value("THRUM_CAPTURE_CLOUD_TAKE").map_err(|error| {
            format!("clouds-inspect requires THRUM_CAPTURE_CLOUD_TAKE: {error}")
        })?;
        let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
        serde_json::from_str(&text).map_err(|error| error.to_string())
    }

    pub(super) fn bounds(&self) -> (f64, f64, f64) {
        match self {
            Self::Sweep { .. } => (22.5, 22.0, 22.0),
            Self::Day => (5.4, 5.2, 4.6),
            _ => (8.5, 8.0, 8.0),
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "bounded capture interpolation narrows to engine coordinates"
    )]
    pub(super) fn apply(
        &self,
        driver: &mut Offscreen<HostedGame<WindowGame>>,
        eye: [f32; 3],
        elapsed: f64,
        discrete_done: &mut bool,
    ) -> Result<Option<Value>, String> {
        let phase = |start: f64, span: f64| ((elapsed - start) / span).clamp(0.0_f64, 1.0);
        let parameter = |driver: &mut Offscreen<HostedGame<WindowGame>>, value: Value| {
            driver
                .parameters(&value.to_string())
                .map_err(|error| error.to_string())?;
            Ok::<_, String>(Some(value))
        };
        match self {
            Self::Day => Ok(None),
            Self::Sweep {
                key,
                from,
                to,
                logarithmic,
            } => {
                let progress = phase(1.0, 20.0);
                let value = if *logarithmic {
                    if *from <= 0.0 || *to <= 0.0 {
                        return Err(
                            "a logarithmic control sweep needs positive endpoints".to_owned()
                        );
                    }
                    (to.ln() - from.ln()).mul_add(progress, from.ln()).exp()
                } else {
                    (to - from).mul_add(progress, *from)
                };
                parameter(
                    driver,
                    json!({key: value.clamp(from.min(*to), from.max(*to))}),
                )
            }
            Self::Pan { from, to } => {
                let yaw = (to - from).mul_add(phase(2.0, 0.8) as f32, *from);
                driver.host_mut().set_camera_yaw(yaw.to_radians())?;
                Ok(Some(json!({"camera.yaw": yaw})))
            }
            Self::Walk => {
                let next = [eye[0] + 40.0 * phase(1.0, 5.0) as f32, eye[1], eye[2]];
                crate::host::set_haze_eye(driver.host_mut(), next, false)?;
                Ok(Some(json!({"eye": next})))
            }
            _ if *discrete_done || elapsed < 2.0 => Ok(None),
            action => {
                *discrete_done = true;
                match action {
                    Self::Lapse => parameter(driver, json!({"clouds.time_lapse":60})),
                    Self::Preset => parameter(
                        driver,
                        serde_json::from_str(include_str!(
                            "../../../params/clouds-overcast.params.json"
                        ))
                        .map_err(|error| error.to_string())?,
                    ),
                    Self::Cut => {
                        let next = [eye[0] + 350.0, eye[1] + 100.0, eye[2]];
                        crate::host::set_haze_eye(driver.host_mut(), next, true)?;
                        Ok(Some(json!({"cut_eye": next})))
                    }
                    Self::Profile => {
                        let host = driver.host_mut();
                        let knob = host
                            .quality_knobs()
                            .into_iter()
                            .find(|knob| knob.name.ends_with("cloud layer scale"))
                            .ok_or("cloud layer scale is unavailable")?;
                        host.set_quality_knob(&knob.name, 75)
                            .map_err(|error| error.to_string())?;
                        Ok(Some(json!({"cloud layer scale": 75})))
                    }
                    _ => unreachable!("continuous takes handled before discrete edits"),
                }
            }
        }
    }
}
