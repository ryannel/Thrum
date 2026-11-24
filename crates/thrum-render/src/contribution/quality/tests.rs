use super::{QualityError, QualityInvalidation, QualityKnob, QualitySettings};

fn knob() -> QualityKnob {
    QualityKnob {
        effect: "Test cadence in ticks",
        name: "interval".into(),
        value: 2,
        range: 1..=8,
        invalidation: QualityInvalidation::EffectHistory,
        governor_steps: None,
    }
}

#[test]
fn invalid_requests_preserve_the_last_valid_staged_value() {
    let mut settings = QualitySettings::new(vec![knob()]).expect("valid declaration");
    settings.set("interval", 7).expect("valid staged request");
    assert_eq!(
        settings.set("interval", 9),
        Err(QualityError::OutOfRange {
            name: "interval".into(),
            value: 9,
            range: 1..=8,
        })
    );
    assert_eq!(
        settings.set("unknown", 1),
        Err(QualityError::UnknownKnob {
            name: "unknown".into()
        })
    );
    assert_eq!(settings.knobs()[0].value, 2);
    assert_eq!(settings.commit(), QualityInvalidation::EffectHistory);
    assert_eq!(settings.knobs()[0].value, 7);
    assert_eq!(settings.commit(), QualityInvalidation::None);
}

#[test]
fn malformed_declarations_report_the_control_name() {
    let duplicate = QualitySettings::new(vec![knob(), knob()]);
    assert!(
        matches!(duplicate, Err(QualityError::InvalidDefinition { name }) if name == "interval")
    );
    let mut invalid = knob();
    invalid.value = 9;
    assert!(
        matches!(QualitySettings::new(vec![invalid]), Err(QualityError::InvalidDefinition { name }) if name == "interval")
    );
}
