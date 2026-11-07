use super::*;

fn demo() -> (Controls, Slider, Toggle, Text, Readout) {
    let mut controls = Controls::new();
    let time = controls
        .slider("sky.solar_time", "Solar time", 0.0..=1.0)
        .default(0.5)
        .step(1.0 / 1440.0)
        .display(Display::clock())
        .add()
        .unwrap();
    let paused = controls.toggle("world.paused", "Pause", false).unwrap();
    let title = controls.text("demo.title", "Title", "Valley").unwrap();
    let sun = controls
        .readout("sky.sun_elevation", "Sun elevation", Display::degrees())
        .unwrap();
    (controls, time, paused, title, sun)
}

#[test]
fn declarations_refuse_bad_keys_labels_ranges_and_duplicates() {
    let (mut controls, ..) = demo();
    assert!(matches!(
        controls.toggle("Sky.Time", "x", false),
        Err(ControlError::InvalidKey(_))
    ));
    assert!(matches!(
        controls.toggle("sky..t", "x", false),
        Err(ControlError::InvalidKey(_))
    ));
    assert!(matches!(
        controls.toggle("a.b", " ", false),
        Err(ControlError::InvalidLabel(_))
    ));
    assert!(matches!(
        controls.toggle("world.paused", "Again", true),
        Err(ControlError::DuplicateKey(_))
    ));
    assert!(matches!(
        controls.slider("a.b", "A", 1.0..=0.0).add(),
        Err(ControlError::InvalidRange(_))
    ));
    assert!(matches!(
        controls.slider("a.c", "A", 0.0..=1.0).default(2.0).add(),
        Err(ControlError::InvalidRange(_))
    ));
    for scale in [0.0, -1.0, f64::NAN] {
        assert!(matches!(
            controls
                .slider("a.d", "A", 0.0..=1.0)
                .display(Display::scaled(scale, 1, ""))
                .add(),
            Err(ControlError::InvalidRange(_))
        ));
    }
    assert_eq!(
        controls.specs().len(),
        4,
        "refused declarations add nothing"
    );
}

#[test]
fn code_edits_refuse_invalid_values_without_changing_state() {
    let (mut controls, time, ..) = demo();
    let before = controls.revision();
    assert!(controls.set(time, f32::NAN).is_err());
    assert!(controls.set(time, 1.5).is_err());
    assert_eq!(controls.revision(), before);
    assert!(controls.set(time, 0.25).unwrap());
    assert!(
        !controls.set(time, 0.25).unwrap(),
        "same value is not a change"
    );
    assert_eq!(controls.revision(), before + 1);
}

#[test]
fn people_edits_snap_clamp_and_count() {
    let (mut controls, time, ..) = demo();
    assert!(controls.edit_display(time.id(), 18.5).unwrap());
    assert!((controls.get(time) - 18.5 / 24.0).abs() < 1e-5);
    assert_eq!(controls.clamped_edits(), 0);
    controls.edit_display(time.id(), 30.0).unwrap();
    assert!((controls.get(time) - 1.0).abs() < 1e-6);
    controls.edit_display(time.id(), f64::NAN).unwrap();
    assert_eq!(controls.get(time), 0.0);
    assert_eq!(controls.clamped_edits(), 2);
}

#[test]
fn people_edits_of_the_wrong_kind_are_explained() {
    let (mut controls, time, paused, title, sun) = demo();
    let before = controls.revision();
    let message = |result: Result<bool, ControlError>| result.unwrap_err().to_string();
    assert!(message(controls.edit_value(time.id(), Value::Flag(true))).contains("is a slider"));
    assert!(message(controls.edit_display(paused.id(), 1.0)).contains("is a toggle"));
    assert!(message(controls.edit_value(sun.id(), Value::Text("x".into()))).contains("readout"));
    assert!(
        message(controls.edit_value(title.id(), Value::Text("a\nb".into()))).contains("one line")
    );
    assert_eq!(controls.revision(), before, "refused edits change nothing");
}

#[test]
fn people_edits_with_another_models_id_are_refused() {
    let (mut small, ..) = demo();
    let mut larger = Controls::new();
    let mut last = None;
    for index in 0..6 {
        last = Some(
            larger
                .toggle(&format!("extra.flag{index}"), "Flag", false)
                .unwrap(),
        );
    }
    let foreign = last.unwrap().id();
    let before = small.revision();
    for result in [
        small.edit_display(foreign, 1.0),
        small.edit_value(foreign, Value::Flag(true)),
    ] {
        let message = result.unwrap_err().to_string();
        assert!(
            message.contains("control ID 5 is not declared"),
            "{message}"
        );
    }
    assert_eq!(small.revision(), before, "refused edits change nothing");
}

#[test]
fn consumers_see_only_controls_changed_since_their_revision() {
    let (mut controls, time, paused, title, sun) = demo();
    let seen = controls.revision();
    controls.set_flag(paused, true);
    controls.show(sun, 0.3);
    assert_eq!(
        controls.changed_since(seen).collect::<Vec<_>>(),
        [paused.id(), sun.id()]
    );
    let seen = controls.revision();
    controls.set_text(title, "Dusk").unwrap();
    controls.edit_display(time.id(), 6.0).unwrap();
    assert_eq!(
        controls.changed_since(seen).collect::<Vec<_>>(),
        [time.id(), title.id()]
    );
    assert_eq!(controls.changed_since(controls.revision()).count(), 0);
}

#[test]
fn parameter_files_apply_valid_entries_and_explain_the_rest() {
    let (mut controls, time, paused, title, _) = demo();
    let report = controls
        .apply_file(
            r#"{"sky.solar_time": 18.5, "world.paused": true, "demo.title": "Dusk",
                "sky.solar_tim": 3, "sky.sun_elevation": 10, "world.paused2": 1}"#,
        )
        .unwrap();
    assert_eq!(report.changed, 3);
    assert_eq!(report.to_string(), "3 changed, 0 unchanged, 3 refused");
    assert!((controls.get(time) - 18.5 / 24.0).abs() < 1e-5);
    assert!(controls.is_on(paused));
    assert_eq!(controls.text_of(title), "Dusk");
    let messages: Vec<String> = report.problems.iter().map(ToString::to_string).collect();
    assert_eq!(messages.len(), 3, "{messages:?}");
    assert!(
        messages[0].contains("did you mean \"sky.solar_time\""),
        "{}",
        messages[0]
    );
    assert!(messages[1].contains("readout"), "{}", messages[1]);
    assert!(
        messages[2].contains("did you mean \"world.paused\""),
        "{}",
        messages[2]
    );
    let again = controls.apply_file(r#"{"sky.solar_time": 18.5}"#).unwrap();
    assert_eq!((again.changed, again.unchanged), (0, 1));
    assert!(matches!(
        controls.apply_file("[1]"),
        Err(ControlError::InvalidFile(_))
    ));
    let out_of_range = controls.apply_file(r#"{"sky.solar_time": 40}"#).unwrap();
    assert!(
        out_of_range.problems[0]
            .to_string()
            .contains("outside 0–24")
    );
}

#[test]
fn views_get_display_units_and_formatted_text() {
    let (mut controls, time, _, _, sun) = demo();
    controls.show(sun, std::f32::consts::FRAC_PI_6);
    let described: serde_json::Value = serde_json::from_str(&controls.describe_json()).unwrap();
    let first = &described["controls"][0];
    assert_eq!(first["kind"], "slider");
    assert_eq!(first["group"], "sky");
    assert_eq!(first["max"], 24.0);
    assert_eq!(first["text"], "12:00");
    assert_eq!(described["controls"][3]["text"], "30°");
    let seen = controls.revision();
    controls.set(time, 0.75).unwrap();
    let changes: serde_json::Value = serde_json::from_str(&controls.changes_json(seen)).unwrap();
    assert_eq!(changes["changed"][0]["key"], "sky.solar_time");
    assert_eq!(changes["changed"][0]["text"], "18:00");
    assert_eq!(changes["changed"][0]["value"], 18.0);
}

#[test]
fn views_get_authored_numbers_not_f32_noise() {
    let mut controls = Controls::new();
    let one_degree = 1.0_f32.to_radians();
    controls
        .slider(
            "camera.pitch",
            "Pitch",
            -34.0 * one_degree..=34.0 * one_degree,
        )
        .default(0.0)
        .step(one_degree)
        .display(Display::degrees())
        .add()
        .unwrap();
    let described: serde_json::Value = serde_json::from_str(&controls.describe_json()).unwrap();
    let pitch = &described["controls"][0];
    assert_eq!(pitch["step"], 1.0);
    assert_eq!(pitch["min"], -34.0);
    assert_eq!(pitch["max"], 34.0);
    assert_eq!(pitch["value"], 0.0);
}

#[test]
fn views_group_controls_by_first_declared_prefix() {
    let (controls, time, paused, title, sun) = demo();
    let groups = controls.groups();
    let keys: Vec<(&str, Vec<ControlId>)> = vec![
        ("sky", vec![time.id(), sun.id()]),
        ("world", vec![paused.id()]),
        ("demo", vec![title.id()]),
    ];
    assert_eq!(groups, keys);
}

#[test]
fn a_repeated_person_edit_is_still_reported() {
    let (mut controls, time, ..) = demo();
    let seen = controls.revision();
    assert!(
        !controls.edit_display(time.id(), 12.0).unwrap(),
        "already 12:00"
    );
    assert_eq!(
        controls.changed_since(seen).collect::<Vec<_>>(),
        [time.id()]
    );
    let seen = controls.revision();
    assert!(
        !controls.set(time, 0.5).unwrap(),
        "code edits of the same value are silent"
    );
    assert_eq!(controls.changed_since(seen).count(), 0);
}
