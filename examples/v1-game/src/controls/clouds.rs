use thrum_cloud_model::Parameters;
use thrum_controls::{ControlError, Controls, Display, Slider};

/// Latest target for the weather tick and latest diagnostic selection, independently.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CloudEdits {
    /// Weather target in canonical engine units.
    pub weather: Option<Parameters>,
    /// Internal field shown in the sky view.
    pub overlay: Option<thrum_clouds::FieldOverlay>,
}

pub(super) struct CloudControls {
    weather: [Slider; 5],
    overlay: Option<Slider>,
    revision: u64,
}

impl CloudControls {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the authored controls use bounded f32 values"
    )]
    pub(super) fn declare(
        model: &mut Controls,
        initial: Parameters,
        sky: bool,
    ) -> Result<Self, ControlError> {
        let [x, z] = initial.wind;
        let direction = z.atan2(x).to_degrees().rem_euclid(360.0);
        let weather = [
            model
                .slider("clouds.coverage", "Coverage", 0.0..=1.0)
                .default(initial.coverage as f32)
                .step(0.001)
                .display(Display::scaled(100.0, 1, "%"))
                .add()?,
            model
                .slider("clouds.type", "Cloud type", 0.0..=1.0)
                .default(initial.kind as f32)
                .step(0.001)
                .display(Display::plain(3, ""))
                .add()?,
            model
                .slider("clouds.wind_speed", "Wind speed", 0.0..=40.0)
                .default(x.hypot(z).min(40.0) as f32)
                .step(0.1)
                .display(Display::plain(1, "m/s"))
                .add()?,
            model
                .slider("clouds.wind_direction", "Wind direction", 0.0..=360.0)
                .default(direction as f32)
                .step(1.0)
                .display(Display::plain(0, "°"))
                .add()?,
        ];
        let rate = model
            .slider("clouds.time_lapse", "Weather time-lapse", 1.0..=60.0)
            .default(initial.time_scale as f32)
            .step(0.1)
            .display(Display::plain(1, "×"))
            .add()?;
        let overlay = sky
            .then(|| {
                model
                    .slider("clouds.overlay", "Internal field", 0.0..=7.0)
                    .step(1.0)
                    .display(Display::labelled_stops(thrum_clouds::FieldOverlay::LABELS))
                    .add()
            })
            .transpose()?;
        Ok(Self {
            weather: [weather[0], weather[1], weather[2], weather[3], rate],
            overlay,
            revision: model.revision(),
        })
    }

    pub(super) fn take_edits(&mut self, model: &Controls) -> CloudEdits {
        let changed = |slider: Slider| model.revision_of(slider.id()) > self.revision;
        let weather = self.weather.iter().copied().any(changed).then(|| {
            let [coverage, kind, speed, direction, time_scale] =
                self.weather.map(|slider| f64::from(model.get(slider)));
            let (z, x) = direction.to_radians().sin_cos();
            Parameters {
                coverage,
                kind,
                wind: [speed * x, speed * z],
                time_scale,
            }
        });
        let overlay = self
            .overlay
            .filter(|slider| changed(*slider))
            .map(|slider| thrum_clouds::FieldOverlay::from_stop(model.get(slider)));
        self.revision = model.revision();
        CloudEdits { weather, overlay }
    }
}
