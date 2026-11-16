//! Locate inspectable crest sections in the actual field; not an appearance proof.
use std::error::Error;
use thrum_sea_model::{LongWaves, Spectrum};

#[expect(
    clippy::print_stdout,
    reason = "this field diagnostic emits its source section"
)]
fn main() -> Result<(), Box<dyn Error>> {
    let spectrum = Spectrum::build(&water_scene::forcing())?;
    let q = water_scene::profile(&spectrum).waves.choppiness;
    let field = LongWaves::new(spectrum.long, q)?;
    let time = 599.0 / 60.0;
    let mut sections = Vec::new();
    for z in -40..=40 {
        for x in -40..=40 {
            let p = [f64::from(x) * 4.0, f64::from(z) * 4.0];
            let centre = field.evaluate(p, time)?;
            if centre.height < 0.2 {
                continue;
            }
            let left = field.evaluate([p[0] - 0.5, p[1]], time)?.height;
            let right = field.evaluate([p[0] + 0.5, p[1]], time)?.height;
            let curvature = -(left - 2.0 * centre.height + right) / 0.25;
            sections.push((curvature, p, centre.height));
        }
    }
    sections.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("source display time={time}; q={q}");
    for (curvature, p, h) in sections.into_iter().take(8) {
        println!("point={p:?} height={h:.5} crest curvature={curvature:.5}");
        for dx in (-12..=12).step_by(2) {
            let at = [p[0] + f64::from(dx), p[1]];
            let s = field.evaluate(at, time)?;
            println!("  x={:.3} h={:.6} slope={:.6}", at[0], s.height, s.slope[0]);
        }
    }
    Ok(())
}
