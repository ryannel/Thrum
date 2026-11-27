use thrum_render::contribution::{QualityInvalidation, QualityKnob, QualitySettings};

pub(crate) const WINDOW: &str = "water.caustic_window_metres";
pub(crate) const CELL: &str = "water.caustic_cell_scale";
pub(crate) const RUNGS: &str = "water.caustic_rungs";
pub(crate) const CADENCE: &str = "water.caustic_cadence";
pub(crate) const STEPS: &str = "water.shaft_steps";
pub(crate) const RESOLUTION: &str = "water.shaft_resolution_divisor";

pub(crate) fn settings(shafts: bool) -> QualitySettings {
    let knob = |name: &str, effect, value, range, invalidation| QualityKnob {
        name: name.into(),
        effect,
        value,
        range,
        invalidation,
        governor_steps: None,
    };
    let reset = QualityInvalidation::EffectHistory;
    let mut knobs = vec![
        knob(
            WINDOW,
            "World width in metres of the caustic window",
            18,
            12..=24,
            reset,
        ),
        knob(
            CELL,
            "Caustic cell scale level (1, 2 or 4 times finest)",
            1,
            1..=3,
            reset,
        ),
        knob(
            RUNGS,
            "Count of active caustic depth rungs",
            13,
            5..=13,
            reset,
        ),
        knob(
            CADENCE,
            "Display frames between caustic evaluations",
            1,
            1..=4,
            reset,
        ),
    ];
    if shafts {
        knobs.extend([
            knob(
                STEPS,
                "Signed shafts march steps; zero keeps directional scattering only",
                16,
                0..=32,
                QualityInvalidation::None,
            ),
            knob(
                RESOLUTION,
                "Shaft render extent divisor",
                2,
                2..=4,
                QualityInvalidation::None,
            ),
        ]);
    }
    QualitySettings::new(knobs).expect("underwater Sun quality declarations")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Values {
    pub(crate) window: u32,
    pub(crate) cell_scale: u32,
    pub(crate) rungs: u32,
    pub(crate) cadence: u32,
    pub(crate) shaft_steps: u32,
    pub(crate) shaft_divisor: u32,
}

impl Values {
    pub(crate) fn read(settings: &QualitySettings) -> Self {
        let knobs = settings.knobs();
        Self {
            window: knobs[0].value,
            cell_scale: knobs[1].value - 1,
            rungs: knobs[2].value,
            cadence: knobs[3].value,
            shaft_steps: knobs.get(4).map_or(0, |knob| knob.value),
            shaft_divisor: knobs.get(5).map_or(2, |knob| knob.value),
        }
    }
}
