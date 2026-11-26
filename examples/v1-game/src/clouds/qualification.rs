//! The CPU weather oracle shared by the game's portable Metal and Chrome proofs.

use crate::effect::qualification::read_texture_bytes;
use thrum_cloud_model::Field;
use thrum_clouds::{RowStamp, ShadeConfig};

/// The committed axes, rows and products of a submitted Weather Map.
#[derive(Clone, Copy)]
pub struct WeatherSnapshot<'a> {
    /// Column, optical and exact integer-identity textures.
    pub textures: &'a [wgpu::Texture; 3],
    /// Air-frame origin of each level.
    pub origins: &'a [[f64; 2]],
    /// Committed source tick, time and weather controls of each row.
    pub rows: &'a [Vec<RowStamp>],
}

/// Readback extrema from the CPU comparison, ordered as in M2's Metal test.
#[derive(Debug)]
pub struct WeatherCheck {
    /// Maximum absolute errors for base, top, coverage, type, age and optical depth.
    pub largest: [f64; 6],
    /// The same errors in the numerical fixture's dimensional scale.
    pub normalized: [f64; 6],
    /// Exact clear and cloudy texel counts, in that order.
    pub counts: [usize; 2],
}

/// A weather qualification could not read or validate the submitted map.
#[derive(Debug)]
pub enum WeatherCheckError {
    /// The supplied model profile was refused.
    Model(thrum_cloud_model::Error),
    /// A texture could not be copied and mapped.
    Readback(String),
    /// A submitted value disagreed with the comparison.
    Comparison(String),
}

impl std::fmt::Display for WeatherCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Model(reason) => write!(f, "weather model: {reason}"),
            Self::Readback(reason) => write!(f, "weather readback: {reason}"),
            Self::Comparison(reason) => f.write_str(reason),
        }
    }
}

impl std::error::Error for WeatherCheckError {}

/// Compare every weather column with the authority at its own source tick.
///
/// # Errors
/// Returns an incomplete readback, binary-mask mismatch, or first out-of-tolerance channel.
pub async fn inspect_weather(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    config: &ShadeConfig,
    frame: WeatherSnapshot<'_>,
) -> Result<WeatherCheck, WeatherCheckError> {
    let field = Field::new(config.seed, config.model, config.types.clone())
        .map_err(WeatherCheckError::Model)?;
    let mut weather = Vec::new();
    for texture in frame.textures {
        weather.push(
            read_texture_bytes(device, queue, texture, 0, 16)
                .await
                .map_err(WeatherCheckError::Readback)?,
        );
    }

    // M2's analytic Metal fixture has a 260 m maximum thickness, 0.015 / m
    // extinction and edge fractions [0.2, 0.3]. Compare dimensional errors in
    // those units, scaling by profile ceilings before looking at the pixels.
    // Base, coverage, kind and age keep their original absolute comparisons.
    let mut scale = [1.0_f64; 6];
    for kind in &config.types {
        scale[1] = scale[1].max(kind.depth[1] / 260.0);
        let tau =
            kind.depth[1] * kind.extinction() * (1.0 - kind.profile[0].midpoint(kind.profile[1]));
        scale[5] = scale[5].max(tau / (260.0 * 0.015 * 0.75));
    }
    weather_error(
        frame,
        &config.weather,
        &field,
        (config.seed, config.model),
        &weather,
        scale,
    )
    .map_err(WeatherCheckError::Comparison)
}

fn word(bytes: &[u8; 16], channel: usize) -> u32 {
    u32::from_le_bytes(bytes.as_chunks::<4>().0[channel])
}

fn weather_error(
    frame: WeatherSnapshot<'_>,
    profile: &thrum_clouds::WeatherProfile,
    field: &Field,
    (seed, model): (u32, thrum_cloud_model::Profile),
    pixels: &[Vec<u8>],
    scales: [f64; 6],
) -> Result<WeatherCheck, String> {
    // M2's Metal test in thrum-clouds/tests/support.rs.
    const LIMITS: [f64; 6] = [0.000_01, 0.0016, 0.000_009, 0.000_01, 0.21, 3.5e-6];
    let size = profile.size as usize;
    let count = size * size * profile.texels.len();
    if pixels.iter().any(|texture| texture.len() != count * 16)
        || frame.origins.len() != profile.texels.len()
        || frame.rows.len() != profile.texels.len()
        || frame.rows.iter().any(|rows| rows.len() != size)
    {
        return Err("incomplete weather readback".into());
    }
    let columns = pixels[0].as_chunks::<16>().0;
    let optical = pixels[1].as_chunks::<16>().0;
    let identities = pixels[2].as_chunks::<16>().0;
    let mut largest = [0.0_f64; 6];
    let mut normalized = [0.0_f64; 6];
    let mut counts = [0; 2];
    for (level, (&texel, origin)) in profile.texels.iter().zip(frame.origins).enumerate() {
        for y in 0..profile.size {
            let stamp = frame.rows[level][y as usize];
            for x in 0..profile.size {
                let at = (level * size + y as usize) * size + x as usize;
                let air = [
                    origin[0] + (f64::from(x) + 0.5) * texel,
                    origin[1] + (f64::from(y) + 0.5) * texel,
                ];
                let expected = field.procedural_column(
                    air,
                    stamp.time,
                    stamp.parameters,
                    stamp.procedural_share,
                );
                let reference = [
                    expected.base,
                    expected.top,
                    expected.coverage,
                    expected.kind,
                    expected.age,
                    expected.optical_depth,
                ];
                let values = [
                    word(&columns[at], 0),
                    word(&columns[at], 1),
                    word(&columns[at], 2),
                    word(&columns[at], 3),
                    word(&optical[at], 0),
                    word(&optical[at], 1),
                ]
                .map(|bits| f64::from(f32::from_bits(bits)));
                let cloudy = values[2] > 0.0;
                counts[usize::from(cloudy)] += 1;
                if cloudy != (expected.coverage > 0.0) {
                    return Err(format!(
                        "weather level {level} texel {x},{y} tick {}: clear/cloudy mismatch {:?} vs {reference:?}",
                        stamp.tick, values
                    ));
                }
                for i in 0..values.len() {
                    let raw = (values[i] - reference[i]).abs();
                    let error = raw / scales[i];
                    let bound = if i == 4 && values[i] > 0.0 && reference[i] > 0.0 {
                        1e-5
                    } else {
                        LIMITS[i]
                    };
                    if !error.is_finite() || error > bound {
                        return Err(format!(
                            "weather level {level} texel {x},{y} tick {} channel {i}: error {error} exceeds {bound}",
                            stamp.tick
                        ));
                    }
                    largest[i] = largest[i].max(raw);
                    normalized[i] = normalized[i].max(error);
                }
                for (j, fade) in expected.profile.into_iter().enumerate() {
                    let actual = f64::from(f32::from_bits(word(&optical[at], j + 2)));
                    if !actual.is_finite() || (actual - fade).abs() > 1e-5 {
                        return Err(format!(
                            "weather level {level} texel {x},{y} tick {}: vertical fade mismatch",
                            stamp.tick
                        ));
                    }
                }
                check_identity(air, stamp.time, seed, model, &identities[at])?;
            }
        }
    }
    Ok(WeatherCheck {
        largest,
        normalized,
        counts,
    })
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "signed lattice coordinates and generations use two's-complement GPU identities"
)]
fn check_identity(
    air: [f64; 2],
    time: f64,
    seed: u32,
    model: thrum_cloud_model::Profile,
    identity: &[u8; 16],
) -> Result<(), String> {
    let [i, j] = air.map(|value| (value / model.pitch).floor() as i64 as u32);
    let phase = time / model.period;
    let mut mask = 0;
    for di in -1_i32..=1 {
        for dj in -1_i32..=1 {
            let offset = birth_offset(i.wrapping_add_signed(di), j.wrapping_add_signed(dj), seed);
            if phase.fract() < offset {
                mask |= 1 << ((di + 1) * 3 + dj + 1);
            }
        }
    }
    let generation = (phase.floor() as i64 as u32).wrapping_sub((mask >> 4) & 1);
    if [i, j, generation, mask] != std::array::from_fn(|channel| word(identity, channel)) {
        return Err(format!("weather identity mismatch at {air:?} and {time}s"));
    }
    Ok(())
}

// Same CPU integer calculation as M2's Metal conformance test.
fn birth_offset(i: u32, j: u32, seed: u32) -> f64 {
    let mut x = i
        .wrapping_mul(0x9e37_79b1)
        .wrapping_add(j.wrapping_mul(0x85eb_ca77))
        .wrapping_add((1 + 131 * seed).wrapping_mul(0x27d4_eb2f));
    x = (x ^ (x >> 15)).wrapping_mul(0x2c1b_3c6d);
    x = (x ^ (x >> 12)).wrapping_mul(0x297a_2d39);
    f64::from(x ^ (x >> 15)) / 4_294_967_296.0
}
