//! Small reproduction of the selected long-wave deformation budget, not a visual pass.
use std::error::Error;
use thrum_sea_model::Spectrum;

#[expect(
    clippy::print_stdout,
    reason = "this diagnostic emits its measured profile record"
)]
fn main() -> Result<(), Box<dyn Error>> {
    let mut forcing = water_scene::forcing();
    let spectrum = Spectrum::build(&forcing).map_err(|e| format!("{e:?}"))?;
    let q = water_scene::profile(&spectrum).waves.choppiness;
    let steepness: Vec<_> = spectrum
        .long
        .iter()
        .map(|c| c.amplitude * c.wave_vector[0].hypot(c.wave_vector[1]))
        .collect();
    let sum: f64 = steepness.iter().sum();
    let rms = q * (0.5 * steepness.iter().map(|v| v * v).sum::<f64>()).sqrt();
    let largest = q * steepness.iter().copied().fold(0.0, f64::max);
    let safe_q = 0.45 / sum;
    forcing.swells.clear();
    let wind = forcing.band_variances().map_err(|e| format!("{e:?}"))?;
    let data = serde_json::json!({"long_components":spectrum.long.len(),"long_variance_share":spectrum.long.iter().map(|c|0.5*c.amplitude.powi(2)).sum::<f64>()/spectrum.variance(),"wind_variance_above_20_m_share":wind[0]/wind.iter().sum::<f64>(),"choppiness":q,"sum_amplitude_wavenumber":sum,"contraction_bound":q*sum,"ensemble_rms_horizontal_derivative":rms,"largest_individual_qak":largest,"largest_individual_crest_trough_curvature_ratio":((1.0+largest)/(1.0-largest)).powi(2),"untried_q_at_contraction_0_45":safe_q,"ensemble_rms_at_untried_q":rms*safe_q/q,"scope":"The single-component curvature ratio and ensemble RMS do not bound the assembled surface's appearance. The larger permitted q has not been rendered. This diagnoses the selected budget; it does not prove impossibility or pass M2.2."});
    println!("{}", serde_json::to_string_pretty(&data)?);
    Ok(())
}
