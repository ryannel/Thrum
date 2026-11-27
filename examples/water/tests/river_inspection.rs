use thrum_app::sky::Camera;

pub(super) struct Shot {
    pub name: String,
    pub camera: Camera,
    pub start: f64,
}

pub(super) fn shot() -> Option<Shot> {
    let name = std::env::var("THRUM_WATER_RIVER_INSPECTION").ok()?;
    let (station, angle) = name
        .rsplit_once('-')
        .expect("river inspection station and angle");
    let (target, start, stream) = match station {
        "pool" => ([179.3, 16.8, -63.0], 17.0, true),
        "ledge" => ([179.0, 16.85, -52.8], 7.0, true),
        "rock" => ([182.6, 16.40, -42.4], 7.0, true),
        "halfway" => ([282.0, 4.5, 6.5], 14.0, false),
        "tidal" => ([340.3, -0.4, 7.8], 28.7, false),
        "lake-inlet" => ([181.25, 12.5, -20.0], 8.0, true),
        "lake-outlet" => ([207.5, 12.5, 5.0], 8.0, false),
        "sea-mouth" => ([347.5, 0.0, 5.0], 8.0, false),
        _ => panic!("unknown river inspection station {station}"),
    };
    let [along, up, across] = match angle {
        "above" => [-4.2, 6.0, 4.2],
        "bank" => [-4.5, 2.0, 3.6],
        "lee" if station == "rock" => [3.2, 2.0, 3.5],
        _ => panic!("unknown river inspection angle {angle}"),
    };
    let [x, y, z] = target;
    let position = if stream {
        [x - across, y + up, z + along]
    } else {
        [x + along, y + up, z + across]
    };
    let camera = Camera::look_at(position, target, 50.0);
    Some(Shot {
        name,
        camera,
        start,
    })
}
