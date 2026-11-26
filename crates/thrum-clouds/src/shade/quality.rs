use super::{Error, QualityInvalidation, QualityKnob, QualitySettings, Shade};
use std::ops::RangeInclusive;

pub(super) const WEATHER: &str = "cloud weather map cadence";
pub(super) const SUN: [&str; 3] = [
    "cloud near Sun path cadence",
    "cloud middle Sun path cadence",
    "cloud far Sun path cadence",
];
pub(super) const REFRESH: &str = "cloud refresh exponent";
pub(super) const LIGHT: &str = "cloud local light samples";
const SCALE: &str = "cloud layer scale";
const STEP: &str = "cloud trace step length";
const CAP: &str = "cloud trace sample cap";
pub(super) const OVERLAY: &str = "cloud sky overlay cadence";
const PLACED: &str = "cloud placed range";

fn knob(
    name: &str,
    effect: &'static str,
    value: u32,
    range: RangeInclusive<u32>,
    invalidation: QualityInvalidation,
) -> QualityKnob {
    QualityKnob {
        name: name.into(),
        effect,
        value,
        range,
        invalidation,
        governor_steps: None,
    }
}

pub(super) fn base() -> Result<QualitySettings, Error> {
    let age = |name| {
        knob(
            name,
            "Maximum row age as percent of the profile",
            100,
            50..=200,
            QualityInvalidation::Age,
        )
    };
    settings(std::iter::once(age(WEATHER)).chain(SUN.map(age)).collect())
}

pub(super) fn scale(ground: bool) -> QualityKnob {
    let (effect, range) = if ground {
        (
            "Ground traces a stop for each scene render pixel",
            100..=100,
        )
    } else {
        ("Percent of prepared layer width and height", 50..=100)
    };
    knob(
        SCALE,
        effect,
        100,
        range,
        QualityInvalidation::EffectHistory,
    )
}

pub(super) fn sky(profile: crate::SkyProfile, ground: bool) -> [QualityKnob; 6] {
    let reset = QualityInvalidation::EffectHistory;
    [
        knob(
            REFRESH,
            "Refresh every cloud texel within 4^value frames",
            profile.refresh_stride.trailing_zeros(),
            0..=2,
            reset,
        ),
        knob(
            LIGHT,
            "Samples used for each cloud local light",
            profile.light_samples,
            0..=3,
            reset,
        ),
        scale(ground),
        knob(
            STEP,
            "Minimum march step as percent of the profile",
            100,
            50..=200,
            reset,
        ),
        knob(
            CAP,
            "Maximum samples per visible ray",
            crate::layer::MAX_TRACE_SAMPLES,
            256..=1024,
            reset,
        ),
        knob(
            OVERLAY,
            "Submissions per full refresh of the sky overlay",
            crate::layer::DEFAULT_OVERLAY_SLICES,
            1..=16,
            QualityInvalidation::Age,
        ),
    ]
}

pub(super) fn placed() -> QualityKnob {
    knob(
        PLACED,
        "Percent of the authored placed range",
        100,
        25..=100,
        QualityInvalidation::EffectHistory,
    )
}

pub(super) fn settings(knobs: Vec<QualityKnob>) -> Result<QualitySettings, Error> {
    QualitySettings::new(knobs).map_err(|_| Error::Unsupported("cloud quality declaration"))
}

impl Shade {
    pub(super) fn fix_ground_scale(&mut self) {
        self.configure_quality([scale(true)])
            .expect("fixed Ground scale replaces the existing validated scale knob");
    }

    pub(super) fn configure_quality(
        &mut self,
        replacements: impl IntoIterator<Item = QualityKnob>,
    ) -> Result<(), Error> {
        let mut knobs = self.quality.knobs().to_vec();
        for replacement in replacements {
            if let Some(knob) = knobs.iter_mut().find(|knob| knob.name == replacement.name) {
                *knob = replacement;
            } else {
                knobs.push(replacement);
            }
        }
        self.quality = settings(knobs)?;
        Ok(())
    }

    pub(super) fn apply_quality(&mut self) {
        let Some(gpu) = &mut self.gpu else {
            return;
        };
        let value = |name| {
            self.quality
                .knobs()
                .iter()
                .find(|knob| knob.name == name)
                .map(|knob| knob.value)
        };
        let required = |name| value(name).expect("prepared capability declares this knob");
        let fraction = |name| f64::from(required(name)) / 100.0;
        gpu.weather
            .set_maximum_age(self.config.weather.maximum_age * fraction(WEATHER));
        gpu.sun.set_maximum_ages(std::array::from_fn(|i| {
            self.config.sun.levels[i].maximum_age * fraction(SUN[i])
        }));
        if let Some((gpu_shapes, profile)) = gpu.shapes.as_mut().zip(self.shapes.as_ref()) {
            let range = profile.range.map(|distance| distance * fraction(PLACED));
            if gpu_shapes.set_range(range) {
                gpu.sun.invalidate();
                if let Some(layer) = &mut gpu.sky {
                    layer.invalidate();
                }
            }
        }
        if let Some((layer, profile)) = gpu.sky.as_mut().zip(self.sky) {
            layer.set_quality(crate::layer::Quality {
                stride: 1 << required(REFRESH),
                light_samples: required(LIGHT),
                layer_percent: required(SCALE),
                step: profile.step
                    * f32::from(u16::try_from(required(STEP)).expect("bounded percentage"))
                    / 100.0,
                cap: required(CAP),
            });
            layer.set_overlay_cadence(required(OVERLAY));
        }
    }
}
