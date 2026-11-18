//! CPU island conformance samples consumed by the Chrome proof.
use serde_json::json;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let map = water_scene::island::map(water_scene::island::Wind::Trade)?;
    let mut state = 0x4fa7_9271u32;
    let mut uniform = || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (state >> 8) as f32 / 16_777_216.0
    };
    let mut points = (0..4096)
        .map(|_| [uniform() * 660.0 - 230.0, uniform() * 500.0 - 250.0])
        .collect::<Vec<_>>();
    let wet = |point: [f32; 2]| {
        map.sample(point.map(f64::from))
            .filter(|s| s.depth > 0.0)
            .map(|s| s.body)
    };
    let mut crossings = [0; 3];
    for j in -224_i16..224 {
        let z = f32::from(j) + 0.173;
        let mut from = [-198.0, z];
        for i in 1_i16..240 {
            let to = [f32::from(i) * 2.5 - 198.0, z];
            if wet(from) != wet(to) {
                let [mut lo, mut hi] = [from[0], to[0]];
                let before = wet(from);
                for body in [before, wet(to)].into_iter().flatten() {
                    crossings[body] += 1;
                }
                for _ in 0..15 {
                    let mid = lo.midpoint(hi);
                    if wet([mid, z]) == before {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                for offset in [-0.04, -0.002, 0.002, 0.04] {
                    points.push([lo.midpoint(hi) + offset, z]);
                }
            }
            from = to;
        }
    }
    let expected = points
        .iter()
        .map(|p| {
            let p = p.map(f64::from);
            let bed = map.bed().height(p);
            let row = map.sample(p);
            json!([
                row.map(|a| a.body),
                row.map(|a| a.row.level),
                bed,
                row.filter(|a| a.bed.is_some()).map(|a| a.depth),
                row.map(|a| a.row.current)
            ])
        })
        .collect::<Vec<_>>();
    serde_json::to_writer(
        io::stdout().lock(),
        &json!({"points":points,"expected":expected,"shore_crossings":crossings}),
    )?;
    writeln!(io::stdout().lock())?;
    Ok(())
}
