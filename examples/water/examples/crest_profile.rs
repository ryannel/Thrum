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
    let bound = thrum_sea_model::contraction_bound(&spectrum.long, 1.0)?;
    let safe_q = (0.85 / bound).min(4.0);
    forcing.swells.clear();
    let wind = forcing.band_variances().map_err(|e| format!("{e:?}"))?;
    let mut fetch_study = Vec::new();
    for fetch in [5_000.0, 10_000.0, 25_000.0, 100_000.0] {
        forcing.fetch = fetch;
        let bands = forcing.band_variances().map_err(|e| format!("{e:?}"))?;
        let wind_height = 4.0 * bands.iter().sum::<f64>().sqrt();
        let peak = (0..1000)
            .map(|i| 0.01 * (1000.0_f64).powf(f64::from(i) / 999.0))
            .max_by(|a, b| forcing.log_density(*a).total_cmp(&forcing.log_density(*b)))
            .expect("nonempty wavenumber range");
        fetch_study.push(serde_json::json!({"fetch_m":fetch,"wind_significant_height_m":wind_height,"wind_peak_wavelength_m":std::f64::consts::TAU/peak,"wind_long_variance_share":bands[0]/bands.iter().sum::<f64>()}));
    }
    let data = serde_json::json!({"long_components":spectrum.long.len(),"long_variance_share":spectrum.long.iter().map(|c|0.5*c.amplitude.powi(2)).sum::<f64>()/spectrum.variance(),"wind_variance_above_20_m_share":wind[0]/wind.iter().sum::<f64>(),"choppiness":q,"sum_amplitude_wavenumber":sum,"trace_bound":q*sum,"contraction_bound":q*bound,"ensemble_rms_horizontal_derivative":rms,"largest_individual_qak":largest,"largest_individual_crest_trough_curvature_ratio":((1.0+largest)/(1.0-largest)).powi(2),"q_at_contraction_0_85":safe_q,"scope":"The single-component curvature ratio and ensemble RMS do not bound the assembled surface's appearance. This diagnoses the selected numerical profile; it does not prove impossibility or pass M2.2."});
    let mut data = data;
    data["fetch_study"] = serde_json::json!(fetch_study);
    println!("{}", serde_json::to_string_pretty(&data)?);
    Ok(())
}
