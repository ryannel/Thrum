//! Explicit cooked-scene qualification controls, using the game's public host API.
//! A held world is only used for still/pan inspection, never lifecycle or timing proof.

use std::time::Instant;

use v1_game::{FrameHost, ThreadedFrameHost};

pub(super) struct View {
    yaw: f32,
    solar: f32,
    pan: Option<Instant>,
}

impl View {
    pub(super) fn prepare(host: &mut FrameHost) -> Result<Option<Self>, String> {
        if std::env::var("THRUM_V1_ROUTE").as_deref() != Ok("1")
            || std::env::var("THRUM_COOKED_DRAW_HELD").as_deref() != Ok("1")
        {
            return Ok(None);
        }
        let number = |key, default: &str| {
            std::env::var(key)
                .unwrap_or_else(|_| default.to_owned())
                .parse::<f32>()
                .map_err(|error| format!("{key}: {error}"))
        };
        let yaw = number("THRUM_COOKED_DRAW_YAW", "0")?;
        let solar = number("THRUM_COOKED_DRAW_SOLAR", "12")?;
        host.pause()?;
        host.set_time_of_day(solar / 24.0)?;
        host.set_camera_yaw(yaw.to_radians())?;
        Ok(Some(Self {
            yaw,
            solar,
            pan: None,
        }))
    }

    pub(super) fn update(
        &mut self,
        host: &mut ThreadedFrameHost,
        start: bool,
    ) -> Result<(), String> {
        if start {
            self.pan = Some(Instant::now());
        }
        if let Some(origin) = self.pan {
            // A two-second start hold, eight-second 0–36 degree pan, then an open-ended hold.
            self.yaw = ((origin.elapsed().as_secs_f32() - 2.0) / 8.0).clamp(0.0, 1.0) * 36.0;
            host.set_camera_yaw(self.yaw.to_radians())?;
        }
        Ok(())
    }

    pub(super) fn witness(&self) -> String {
        format!(
            "\ncooked_qualification_held=true\ncooked_yaw={}\ncooked_solar={}\ncooked_pan_seconds={}",
            self.yaw,
            self.solar,
            self.pan
                .map_or(0.0, |origin| origin.elapsed().as_secs_f32())
        )
    }
}
