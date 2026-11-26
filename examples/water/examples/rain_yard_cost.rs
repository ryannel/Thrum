//! Release preparation timing for the shipped rain-yard triangles and shelters.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (elapsed, cells, hollows) = water_scene::island::measure_rain_yard_preparation()?;
    println!(
        "{{\"prepare_seconds\":{},\"cells\":{cells},\"hollows\":{hollows}}}",
        elapsed.as_secs_f64(),
    );
    Ok(())
}
