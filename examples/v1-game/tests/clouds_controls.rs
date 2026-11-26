//! Shared control declarations and parameter files used by the Mac panel and Chrome.
use thrum_controls::Value;
use v1_game::{clouds::WeatherPreset, controls::GameControls};

#[test]
fn mac_and_web_share_one_cloud_control_declaration() {
    let controls = GameControls::with_clouds(Some(WeatherPreset::Fair.parameters()), true)
        .expect("valid qualification input");
    let described: serde_json::Value =
        serde_json::from_str(&controls.model().describe_json()).expect("valid qualification input");
    let specs = described["controls"]
        .as_array()
        .expect("valid qualification input");
    for (key, label, min, max, unit) in [
        ("clouds.coverage", "Coverage", 0, 100, "%"),
        ("clouds.type", "Cloud type", 0, 1, ""),
        ("clouds.wind_speed", "Wind speed", 0, 40, "m/s"),
        ("clouds.wind_direction", "Wind direction", 0, 360, "°"),
        ("clouds.time_lapse", "Weather time-lapse", 1, 60, "×"),
        ("clouds.overlay", "Internal field", 0, 7, ""),
        ("sky.solar_time", "Solar time", 0, 24, "h"),
    ] {
        let spec = specs
            .iter()
            .find(|spec| spec["key"] == key)
            .expect("valid qualification input");
        assert_eq!(spec["label"], label);
        assert_eq!(spec["min"].as_f64(), Some(f64::from(min)));
        assert_eq!(spec["max"].as_f64(), Some(f64::from(max)));
        assert_eq!(spec["display"]["unit"], unit);
    }
    let off = GameControls::new().expect("valid qualification input");
    assert!(off.model().find("clouds.coverage").is_none());
}

#[test]
fn parameter_files_apply_weather_presets_and_independent_diagnostics() {
    let mut controls = GameControls::with_clouds(Some(WeatherPreset::Fair.parameters()), true)
        .expect("valid qualification input");
    for (preset, file) in [
        (
            WeatherPreset::Overcast,
            include_str!("../params/clouds-overcast.params.json"),
        ),
        (
            WeatherPreset::Clear,
            include_str!("../params/clouds-clear.params.json"),
        ),
        (
            WeatherPreset::Fair,
            include_str!("../params/clouds-fair.params.json"),
        ),
        (
            WeatherPreset::Broken,
            include_str!("../params/clouds-broken.params.json"),
        ),
    ] {
        let report = controls
            .model_mut()
            .apply_file(file)
            .expect("valid qualification input");
        assert!(report.problems.is_empty(), "{report}");
        let actual = controls
            .take_edits()
            .clouds
            .weather
            .expect("valid qualification input");
        let expected = preset.parameters();
        assert!((actual.coverage - expected.coverage).abs() < 1e-6);
        assert!((actual.kind - expected.kind).abs() < 1e-6);
        assert_eq!(actual.wind, expected.wind);
        assert_eq!(actual.time_scale, expected.time_scale);
    }

    let report = controls
        .model_mut()
        .apply_file(include_str!("../params/clouds-low-sun.params.json"))
        .expect("valid qualification input");
    assert!(report.problems.is_empty(), "{report}");
    let edits = controls.take_edits();
    let weather = edits.clouds.weather.expect("valid qualification input");
    assert!(weather.wind[0].abs() < 1e-12);
    assert_eq!(weather.wind[1], 20.0);
    assert_eq!(weather.time_scale, 8.0);
    assert_eq!(
        edits.clouds.overlay,
        Some(thrum_clouds::FieldOverlay::SunPath)
    );
    assert!(edits.time_of_day.is_some());
    assert!(controls.take_edits().is_empty());

    let key = controls
        .model()
        .find("clouds.time_lapse")
        .expect("valid qualification input");
    let refused = controls
        .model_mut()
        .apply_file(r#"{"clouds.time_lapse": 61}"#)
        .expect("valid qualification input");
    assert_eq!(refused.problems.len(), 1);
    assert_eq!(controls.model().value(key), &Value::Number(8.0));
}
