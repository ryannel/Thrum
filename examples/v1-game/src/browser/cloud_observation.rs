//! Target qualification of the products in the actual page's submission.

use std::{
    cell::RefCell,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    rc::Rc,
};

use half::f16;
use thrum_cloud_model::Field;
use thrum_clouds::{CloudSubmission, RowStamp};
use wasm_bindgen::prelude::*;

use super::{BrowserGame, game, js_error};
use crate::effect::qualification::{read_texture_bytes, read_texture_layers};

#[wasm_bindgen]
impl BrowserGame {
    /// Arm the next submitted cloud frame for qualification.
    ///
    /// # Errors
    /// Refuses a page without cloud products.
    pub fn observe_next_cloud_submission(&mut self, ground: bool) -> Result<(), JsValue> {
        game::cloud_input(&self.shell)
            .ok_or_else(|| js_error("cloud observation requires Shade or Sky"))?
            .observe_next_submission();
        if ground && self.cloud_observation.is_none() {
            self.cloud_observation = Some(Rc::default());
        }
        Ok(())
    }

    /// Compare the frame's actual products with their CPU references.
    /// The caller waits for the result before allowing another frame to write them.
    pub fn inspect_cloud_submission(&self) -> Option<js_sys::Promise> {
        let frame = game::cloud_input(&self.shell)?.take_submission()?;
        let previous = self.cloud_observation.as_ref().map(Rc::clone);
        Some(wasm_bindgen_futures::future_to_promise(async move {
            let record = inspect(frame, previous.as_deref())
                .await
                .map_err(js_error)?;
            Ok(JsValue::from_str(&record.to_string()))
        }))
    }
}

async fn inspect(
    frame: CloudSubmission,
    previous: Option<&RefCell<PreviousSun>>,
) -> Result<serde_json::Value, String> {
    let config = crate::clouds::shade_config();
    let field =
        Field::new(config.seed, config.model, config.types).map_err(|error| error.to_string())?;
    let mut weather = Vec::new();
    for texture in &frame.weather {
        weather.push(read_texture_bytes(&frame.device, &frame.queue, texture, 0, 16).await?);
    }
    let weather_ages = row_ages(
        &frame,
        &frame.weather_rows,
        &[config.weather.maximum_age; 4],
        config.weather.size,
    )?;
    let (errors, counts) = weather_error(
        &frame,
        &config.weather,
        &field,
        (config.seed, config.model),
        &weather,
    )?;
    let ground = if let Some(previous) = previous {
        let limits = config.sun.levels.map(|level| level.maximum_age);
        let ages = row_ages(&frame, &frame.sun_rows, &limits, config.sun.size)?;
        let check = ground_error(&frame, &config.sun).await?;
        let changes = previous.borrow_mut().observe(&frame, check.fingerprints)?;
        serde_json::json!({
            "max_depth_error": check.max_depth_error, "shade_texels": check.shade_texels,
            "rows": ages, "unchanged_stamp_and_changed_value_counts": changes,
        })
    } else {
        serde_json::Value::Null
    };
    let visible_texels = if ground.is_null() {
        let sky_front = frame
            .sky_front
            .as_ref()
            .ok_or("first cloud frame needs Sky")?;
        let bytes = read_texture_bytes(&frame.device, &frame.queue, sky_front, 0, 8).await?;
        bytes
            .as_chunks::<8>()
            .0
            .iter()
            .filter(|texel| depth(texel, 1) > 0.005)
            .count()
    } else {
        0
    };
    Ok(serde_json::json!({
        "generation": frame.generation, "tick": frame.state.tick(),
        "weather_time": frame.state.time(), "weather_max_errors": errors,
        "weather_texels": counts, "weather_rows": weather_ages, "ground": ground,
        "visible_trace_texels": visible_texels,
    }))
}

#[derive(Default)]
pub(super) struct PreviousSun {
    generation: u64,
    levels: [Vec<(u64, u64)>; 3],
}

impl PreviousSun {
    fn observe(
        &mut self,
        frame: &CloudSubmission,
        fingerprints: [Vec<u64>; 3],
    ) -> Result<[[usize; 2]; 3], String> {
        let mut changes = [[0; 2]; 3];
        let same_device = self.generation == frame.generation;
        let next = fingerprints.into_iter().zip(&frame.sun_rows);
        for (level, ((fingerprints, stamps), old)) in next.zip(&mut self.levels).enumerate() {
            let rows = stamps
                .iter()
                .map(|stamp| stamp.tick)
                .zip(fingerprints)
                .collect::<Vec<_>>();
            if same_device {
                for (row, (&(tick, fingerprint), &(old_tick, old_fingerprint))) in
                    rows.iter().zip(old.iter()).enumerate()
                {
                    if tick < old_tick || (tick == old_tick && fingerprint != old_fingerprint) {
                        return Err(format!(
                            "Sun-path level {level} row {row}: value changed at the same source tick, or source regressed ({old_tick} to {tick})"
                        ));
                    }
                    changes[level][0] += usize::from(tick == old_tick);
                    changes[level][1] += usize::from(fingerprint != old_fingerprint);
                }
            }
            *old = rows;
        }
        self.generation = frame.generation;
        Ok(changes)
    }
}

fn row_ages(
    frame: &CloudSubmission,
    levels: &[Vec<RowStamp>],
    limits: &[f64],
    size: u32,
) -> Result<Vec<serde_json::Value>, String> {
    if levels.len() != limits.len() {
        return Err("missing cloud product level".into());
    }
    levels.iter().zip(limits).enumerate().map(|(level, (rows, &limit))| {
        if rows.len() != size as usize {
            return Err(format!("level {level}: missing cloud product rows"));
        }
        let mut oldest = frame.state.tick();
        let mut maximum_age = 0.0_f64;
        let mut current = 0;
        for row in rows {
            let age = frame.state.time() - row.time;
            if row.tick > frame.state.tick() || !(-1e-8..=limit + 1e-8).contains(&age) {
                return Err(format!("level {level}: row tick {} at {} is {age}s old (limit {limit}s) at tick {}", row.tick, row.time, frame.state.tick()));
            }
            oldest = oldest.min(row.tick);
            maximum_age = maximum_age.max(age);
            current += u32::from(row.tick == frame.state.tick());
        }
        Ok(serde_json::json!({"oldest_tick": oldest, "max_age": maximum_age, "current_rows": current}))
    }).collect()
}

fn word(bytes: &[u8; 16], channel: usize) -> u32 {
    u32::from_le_bytes(bytes.as_chunks::<4>().0[channel])
}

fn weather_error(
    frame: &CloudSubmission,
    profile: &thrum_clouds::WeatherProfile,
    field: &Field,
    (seed, model): (u32, thrum_cloud_model::Profile),
    pixels: &[Vec<u8>],
) -> Result<([f64; 6], [usize; 2]), String> {
    // M2's Metal test in thrum-clouds/tests/support.rs.
    const LIMITS: [f64; 6] = [0.000_01, 0.0016, 0.000_009, 0.000_01, 0.21, 3.5e-6];
    let size = profile.size as usize;
    let count = size * size * profile.texels.len();
    if pixels.iter().any(|texture| texture.len() != count * 16)
        || frame.weather_origins.len() != profile.texels.len()
    {
        return Err("incomplete weather readback".into());
    }
    let columns = pixels[0].as_chunks::<16>().0;
    let optical = pixels[1].as_chunks::<16>().0;
    let identities = pixels[2].as_chunks::<16>().0;
    let mut largest = [0.0_f64; 6];
    let mut counts = [0; 2];
    for (level, (&texel, origin)) in profile
        .texels
        .iter()
        .zip(&frame.weather_origins)
        .enumerate()
    {
        for y in 0..profile.size {
            let stamp = frame.weather_rows[level][y as usize];
            for x in 0..profile.size {
                let at = (level * size + y as usize) * size + x as usize;
                let air = [
                    origin[0] + (f64::from(x) + 0.5) * texel,
                    origin[1] + (f64::from(y) + 0.5) * texel,
                ];
                let expected = field.column(air, stamp.time, stamp.parameters);
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
                    let error = (values[i] - reference[i]).abs();
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
                    largest[i] = largest[i].max(error);
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
    Ok((largest, counts))
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

struct GroundCheck {
    max_depth_error: f64,
    shade_texels: usize,
    fingerprints: [Vec<u64>; 3],
}

async fn ground_error(
    frame: &CloudSubmission,
    profile: &thrum_clouds::SunProfile,
) -> Result<GroundCheck, String> {
    let mut sources = Vec::new();
    let mut fingerprints = std::array::from_fn(|_| Vec::new());
    for (level, texture) in frame.sun.iter().enumerate() {
        let end = texture.depth_or_array_layers();
        let bytes =
            read_texture_layers(&frame.device, &frame.queue, texture, 0, 8, end - 1..end).await?;
        fingerprints[level] = bytes
            .chunks_exact(texture.width() as usize * 8)
            .map(|row| {
                let mut hasher = DefaultHasher::new();
                row.hash(&mut hasher);
                hasher.finish()
            })
            .collect();
        sources.push(
            bytes
                .as_chunks::<8>()
                .0
                .iter()
                .map(|texel| depth(texel, 3))
                .collect::<Vec<_>>(),
        );
    }
    let pixels = read_texture_bytes(&frame.device, &frame.queue, &frame.ground, 0, 8).await?;
    let ground = profile.ground_size as usize;
    let half = ground / 2;
    let size = profile.size as usize;
    if pixels.len() != ground * ground * 8
        || sources.iter().any(|source| source.len() != size * size)
    {
        return Err("incomplete Sun path or ground readback".into());
    }
    let mut max_error = 0.0_f64;
    let mut shade = 0;
    for (index, texel) in pixels.as_chunks::<8>().0.iter().enumerate() {
        let (x, y) = (index % ground, index / ground);
        let tile = (y / half) * 2 + x / half;
        let expected = sources.get(tile).map_or(0.0, |source| {
            resample(source, size, half, x % half, y % half)
        });
        let actual = depth(texel, 0);
        let error = (actual - expected).abs();
        // Ground stores the interpolated optical depth back in binary16.
        if !error.is_finite()
            || actual < 0.0
            || error > expected.abs().mul_add(0.000_6, 6e-8)
            || texel[2..] != [0; 6]
        {
            return Err(format!(
                "ground field texel {x},{y}: depth {actual}, expected {expected}"
            ));
        }
        max_error = max_error.max(error);
        shade += usize::from(actual > 0.001);
    }
    Ok(GroundCheck {
        max_depth_error: max_error,
        shade_texels: shade,
        fingerprints,
    })
}

fn depth(texel: &[u8; 8], channel: usize) -> f64 {
    f16::from_le_bytes(texel.as_chunks::<2>().0[channel]).to_f64()
}

#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "fixed texture extents and clamped, nonnegative source-texel coordinates"
)]
fn resample(source: &[f64], size: usize, half: usize, x: usize, y: usize) -> f64 {
    let taps = |pixel: usize| {
        let p = (pixel as f64 + 0.5) * size as f64 / half as f64 - 0.5;
        let last = (size - 1) as f64;
        (
            p.floor().clamp(0.0, last) as usize,
            (p.floor() + 1.0).clamp(0.0, last) as usize,
            p - p.floor(),
        )
    };
    let (x0, x1, fx) = taps(x);
    let (y0, y1, fy) = taps(y);
    let lerp = |a: f64, b: f64, t: f64| a * (1.0 - t) + b * t;
    lerp(
        lerp(source[y0 * size + x0], source[y0 * size + x1], fx),
        lerp(source[y1 * size + x0], source[y1 * size + x1], fx),
        fy,
    )
}
