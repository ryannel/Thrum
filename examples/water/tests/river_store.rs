//! The scene's shipped declarations and its native preparation command.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn river_selection_and_native_keyed_store() {
    use std::{fs, process::Command};
    use thrum_water_model::RiverFile;
    use water_scene::{island::Wind, settings::Settings};

    assert_eq!(Settings::from_query(""), Settings::OpenSea);
    assert_eq!(
        Settings::from_query("?channel=trade"),
        Settings::Channel(Wind::Trade)
    );
    assert_eq!(
        Settings::from_query("?river=trade"),
        Settings::River(Wind::Trade)
    );
    let scratch = thrum_test_support::Scratch::new("river-store");
    let root = scratch.path();
    let run = || {
        let output = Command::new(env!("CARGO_BIN_EXE_water-scene"))
            .arg("--prepare-rivers")
            .arg(root)
            .output()
            .expect("native product preparation");
        let diagnostics = String::from_utf8(output.stderr).expect("diagnostics");
        assert!(output.status.success(), "{diagnostics}");
        let keys = String::from_utf8(output.stdout).expect("key list");
        (keys, diagnostics)
    };
    let (keys, cold) = run();
    assert!(
        cold.contains("solved Spring stream") && cold.contains("solved Lake river"),
        "{cold}"
    );
    let files: Vec<_> = keys
        .lines()
        .map(|key| {
            let bytes = fs::read(root.join(format!("{key}.river"))).expect("keyed river file");
            let expected = std::array::from_fn(|i| {
                u8::from_str_radix(&key[2 * i..2 * i + 2], 16).expect("hex key")
            });
            assert!(RiverFile::read(&bytes, &[0; 32]).is_err());
            RiverFile::read(&bytes, &expected).expect("read by key")
        })
        .collect();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].gauges().len(), 3);
    assert_eq!(files[1].gauge_range(), [12.0, 13.0]);
    assert!(
        files[0]
            .falls
            .iter()
            .any(|fall| fall.segments.iter().any(|segment| {
                let first = segment.path.first().expect("launch");
                let last = segment.path.last().expect("landing");
                (3.0..=5.5).contains(&(first.height - last.height))
            })),
        "stream includes the plunge"
    );
    let (warm, diagnostics) = run();
    assert_eq!(warm, keys);
    assert!(!diagnostics.contains("solved "), "{diagnostics}");
    let forcing = water_scene::channel::forcing(Wind::Calm);
    let (mut map, _) = water_scene::river::map(&forcing, Some(root)).expect("prepared river map");
    for (body, station, across, range) in [
        (3, [180.0, -42.5], [1.0, 0.0], files[0].gauge_range()),
        (5, [257.5, 5.0], [0.0, 1.0], files[1].gauge_range()),
    ] {
        let sections = range.map(|stage| {
            if body == 3 {
                map.set_gauge(0, stage).expect("spring gauge");
            } else {
                map.set_stage(1, stage - 12.0).expect("lake stage");
            }
            let width = (-600_i16..=600)
                .filter(|&i| {
                    let distance = f64::from(i) * 0.025;
                    let place = [
                        station[0] + across[0] * distance,
                        station[1] + across[1] * distance,
                    ];
                    map.sample(place)
                        .is_some_and(|sample| sample.body == body && sample.depth > 0.0)
                })
                .count();
            let row = map.sample(station).expect("section centre");
            assert_eq!(row.body, body);
            (
                row.depth,
                row.row.current[0].hypot(row.row.current[1]),
                width,
            )
        });
        let [low, high] = sections;
        assert!(
            high.0 > low.0 && high.1 > low.1 && high.2 > low.2,
            "depth, speed and wet width at {station:?}: {sections:?}"
        );
        println!(
            "river section {station:?}: depth, current speed and 25 mm wet stations {sections:?}"
        );
    }
}
