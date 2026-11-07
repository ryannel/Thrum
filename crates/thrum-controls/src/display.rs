//! How an engine value is shown to a person, shared by every platform view.

/// Display mapping from a control's canonical engine value to what a person reads and edits.
///
/// Engine values stay in the game's own units (radians, day fractions, metres). Every view
/// shows `engine × scale` with `decimals` places and `unit`, and maps an edit back by dividing.
/// [`Display::clock`] shows a 0–1 day fraction as `HH:MM`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Display {
    scale: f64,
    decimals: u8,
    unit: &'static str,
    clock: bool,
}

impl Display {
    /// Show the engine value unchanged with `decimals` places and an optional unit suffix.
    #[must_use]
    pub const fn plain(decimals: u8, unit: &'static str) -> Self {
        Self {
            scale: 1.0,
            decimals,
            unit,
            clock: false,
        }
    }

    /// Show `engine × scale` (for example metres as kilometres with `0.001`).
    #[must_use]
    pub const fn scaled(scale: f64, decimals: u8, unit: &'static str) -> Self {
        Self {
            scale,
            decimals,
            unit,
            clock: false,
        }
    }

    /// Show radians as whole degrees.
    #[must_use]
    pub const fn degrees() -> Self {
        Self::scaled(180.0 / std::f64::consts::PI, 0, "°")
    }

    /// Show a day fraction (midnight 0, noon 0.5) as a 24-hour `HH:MM` clock.
    #[must_use]
    pub const fn clock() -> Self {
        Self {
            scale: 24.0,
            decimals: 2,
            unit: "h",
            clock: true,
        }
    }

    /// Multiplier from engine value to display value.
    #[must_use]
    pub const fn scale(self) -> f64 {
        self.scale
    }

    /// Decimal places shown for a plain or scaled value.
    #[must_use]
    pub const fn decimals(self) -> u8 {
        self.decimals
    }

    /// Unit suffix shown after the value (empty for none).
    #[must_use]
    pub const fn unit(self) -> &'static str {
        self.unit
    }

    /// Whether the value is shown as a 24-hour clock.
    #[must_use]
    pub const fn is_clock(self) -> bool {
        self.clock
    }

    /// Display value for an engine value.
    #[must_use]
    pub fn to_display(self, engine: f32) -> f64 {
        f64::from(engine) * self.scale
    }

    /// Engine value for a display value.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "controls hold f32 engine values; display edits are already range-checked"
    )]
    pub fn to_engine(self, display: f64) -> f32 {
        (display / self.scale) as f32
    }

    /// Text a person reads for an engine value, such as `18:30`, `42°` or `1.50 km`.
    #[must_use]
    pub fn format(self, engine: f32) -> String {
        let shown = self.to_display(engine);
        if self.clock {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a clamped 0–24 hour value has at most 1,440 minutes"
            )]
            let minutes = (shown.clamp(0.0, 24.0) * 60.0).round() as u32;
            return format!("{:02}:{:02}", (minutes / 60) % 24, minutes % 60);
        }
        let places = usize::from(self.decimals);
        if self.unit.is_empty() {
            format!("{shown:.places$}")
        } else if self.unit == "°" {
            format!("{shown:.places$}°")
        } else {
            format!("{shown:.places$} {}", self.unit)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Display;

    #[test]
    fn clock_shows_day_fraction_as_hours_and_minutes() {
        let clock = Display::clock();
        assert_eq!(clock.format(0.5), "12:00");
        assert_eq!(clock.format(0.770_833_3), "18:30");
        assert_eq!(clock.format(1.0), "00:00");
        assert!((clock.to_engine(6.0) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn degrees_and_scaled_units_round_trip() {
        let degrees = Display::degrees();
        assert_eq!(degrees.format(std::f32::consts::FRAC_PI_4), "45°");
        assert!((degrees.to_engine(90.0) - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        let km = Display::scaled(0.001, 2, "km");
        assert_eq!(km.format(1500.0), "1.50 km");
        assert_eq!(Display::plain(1, "").format(2.26), "2.3");
    }
}
