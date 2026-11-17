//! Independent f64 reference for samples read from the browser's submitted sea fields.
use serde_json::{Value, json};
use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("request");
    let request: Value = serde_json::from_str(&input).expect("JSON request");
    let time = request["time"].as_f64().expect("display seconds");
    let spectrum = thrum_sea_model::Spectrum::build(&water_scene::forcing()).expect("spectrum");
    let profile = water_scene::profile(&spectrum);
    let long = request["long"]
        .as_array()
        .expect("rest points")
        .iter()
        .map(|v| {
            let max_k = v[2].as_f64().expect("cutoff");
            let waves = thrum_sea_model::LongWaves::new(
                spectrum
                    .long
                    .iter()
                    .copied()
                    .filter(|c| c.wave_vector[0].hypot(c.wave_vector[1]) <= max_k)
                    .collect(),
                profile.waves.choppiness,
            )
            .expect("long waves");
            let value = waves
                .forward([v[0].as_f64().expect("x"), v[1].as_f64().expect("z")], time)
                .expect("forward sheet");
            json!({"displacement":value.displacement,"velocity":value.velocity,"slope":value.slope})
        })
        .collect::<Vec<_>>();
    let cascades = spectrum.cascades.iter().map(|band| {
        let samples = (0..32).map(|i| {
            let pixel = (i * 4051 + 137) % 65536;
            let point = [f64::from(pixel % 256) + 0.5, f64::from(pixel / 256) + 0.5].map(|v| v * band.length() / 256.0);
            let value = band.direct(point, time);
            json!({"pixel":pixel,"displacement":value.displacement,"velocity":value.velocity})
        }).collect::<Vec<_>>();
        json!({"rms":band.variance().sqrt(),"samples":samples})
    }).collect::<Vec<_>>();
    serde_json::to_writer(
        io::stdout().lock(),
        &json!({"long":long,"cascades":cascades}),
    )
    .expect("write oracle response");
}
