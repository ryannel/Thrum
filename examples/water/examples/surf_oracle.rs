//! Evaluate the running island's reference sheet at submitted cache nodes.
use serde_json::json;
use std::io;
use thrum_sea_model::LongWaves;
use thrum_water_model::{Field, Sheet};
use water_scene::island::{Wind, surf_inputs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input: serde_json::Value = serde_json::from_reader(io::stdin().lock())?;
    let time = input["time"].as_f64().ok_or("missing presentation time")?;
    let (map, spectrum) = surf_inputs(Wind::Trade)?;
    let chop = water_scene::profile(&spectrum).waves.choppiness;
    let field = Field {
        layers: [
            LongWaves::new(spectrum.long, chop),
            LongWaves::new(spectrum.cascades[0].components().to_vec(), 0.0),
            LongWaves::new(spectrum.cascades[1].components().to_vec(), 0.0),
            LongWaves::new(spectrum.cascades[2].components().to_vec(), 0.0),
        ]
        .map(|layer| layer.expect("valid wave spectrum")),
    };
    let sheets = [0, 1, 2].map(|body| map.surf_sheet(&field, body, time).expect("surf"));
    let nodes: Vec<(f64, f64, u32)> = serde_json::from_value(input["nodes"].clone())?;
    let answers: Vec<_> = nodes
        .into_iter()
        .map(|(x, z, slot)| {
            let point = [x, z];
            let body = match slot {
                0 => 0,
                2 => 1,
                3 => 2,
                _ => return None,
            };
            map.sample(point)
                .filter(|sample| sample.body == body && sample.bed.is_some())
                .map(|_| {
                    let sample = sheets[body].forward(point);
                    [sample.world[0] - x, sample.height, sample.world[1] - z]
                })
        })
        .collect();
    serde_json::to_writer(io::stdout().lock(), &json!({"answers":answers}))?;
    Ok(())
}
