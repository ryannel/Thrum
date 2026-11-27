use thrum_cloud_model::Parameters;
use thrum_controls::{ControlError, Controls, Display, Slider, Toggle};

/// Latest target for the weather tick and latest diagnostic selection, independently.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CloudEdits {
    /// Weather target in canonical engine units.
    pub weather: Option<Parameters>,
    /// Targets for the ground condensate and aerosol.
    pub ground: Option<crate::clouds::GroundTargets>,
    /// Internal field shown in the sky view.
    pub overlay: Option<thrum_clouds::FieldOverlay>,
}

pub(super) struct CloudControls {
    weather: [Slider; 5],
    overlay: Option<Slider>,
    ground: Option<GroundControls>,
    revision: u64,
    pub(super) replay: CloudEdits,
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
        ground: Option<crate::clouds::GroundTargets>,
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
            model
                .slider("clouds.time_lapse", "Weather time-lapse", 1.0..=60.0)
                .default(initial.time_scale as f32)
                .step(0.1)
                .display(Display::plain(1, "×"))
                .add()?,
        ];
        let overlay_end = if ground.is_some() { 10_u16 } else { 7 };
        let overlay = sky
            .then(|| {
                model
                    .slider(
                        "clouds.overlay",
                        "Internal field",
                        0.0..=f32::from(overlay_end),
                    )
                    .step(1.0)
                    .display(Display::labelled_stops(
                        &thrum_clouds::FieldOverlay::LABELS[..=usize::from(overlay_end)],
                    ))
                    .add()
            })
            .transpose()?;
        Ok(Self {
            weather,
            overlay,
            ground: ground
                .map(|initial| GroundControls::declare(model, initial))
                .transpose()?,
            revision: model.revision(),
            replay: CloudEdits::default(),
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
        let ground = self
            .ground
            .as_ref()
            .and_then(|ground| ground.edit(model, self.revision));
        self.revision = model.revision();
        self.replay = CloudEdits {
            weather: weather.or(self.replay.weather),
            ground: ground.or(self.replay.ground),
            overlay: overlay.or(self.replay.overlay),
        };
        CloudEdits {
            weather,
            ground,
            overlay,
        }
    }
}

struct GroundControls {
    amounts: [Slider; 2],
    moisture: Slider,
    visual_range: Slider,
    angstrom_exponent: Slider,
    radiative_day: Toggle,
}

impl GroundControls {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "typed controls use bounded f32 values"
    )]
    fn declare(
        model: &mut Controls,
        initial: crate::clouds::GroundTargets,
    ) -> Result<Self, ControlError> {
        let mut amount = |key, label, value: f64| {
            model
                .slider(key, label, 0.0..=1.0)
                .default(value as f32)
                .step(0.01)
                .display(Display::plain(2, ""))
                .add()
        };
        let amounts = [
            amount("clouds.mist_amount", "Mist amount", initial.amounts[0])?,
            amount("clouds.fog_amount", "Fog bank amount", initial.amounts[1])?,
        ];
        let moisture = amount("clouds.moisture", "Moisture", initial.moisture)?;
        Ok(Self {
            amounts,
            moisture,
            visual_range: model
                .slider("clouds.visual_range", "Clear-air visual range", 3.0..=300.0)
                .default((initial.visual_range / 1000.0) as f32)
                // Fine stops keep dense haze sweeps from stepping a kilometre at once.
                .step(1.0 / 64.0)
                .display(Display::plain(2, "km"))
                .add()?,
            angstrom_exponent: model
                .slider("clouds.angstrom_exponent", "Ångström exponent", 0.0..=2.0)
                .default(initial.angstrom_exponent as f32)
                .step(0.01)
                .display(Display::plain(2, ""))
                .add()?,
            radiative_day: model.toggle(
                "clouds.radiative_day",
                "Radiative day",
                initial.radiative_day,
            )?,
        })
    }

    fn edit(&self, model: &Controls, revision: u64) -> Option<crate::clouds::GroundTargets> {
        self.amounts
            .map(Slider::id)
            .into_iter()
            .chain([
                self.moisture.id(),
                self.visual_range.id(),
                self.angstrom_exponent.id(),
                self.radiative_day.id(),
            ])
            .any(|id| model.revision_of(id) > revision)
            .then(|| crate::clouds::GroundTargets {
                amounts: self.amounts.map(|slider| f64::from(model.get(slider))),
                moisture: f64::from(model.get(self.moisture)),
                visual_range: f64::from(model.get(self.visual_range)) * 1000.0,
                angstrom_exponent: f64::from(model.get(self.angstrom_exponent)),
                radiative_day: model.is_on(self.radiative_day),
            })
    }
}
