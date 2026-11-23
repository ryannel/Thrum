//! Target qualification of the products in the actual page's submission.

use std::{
    cell::RefCell,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    rc::Rc,
};

use half::f16;
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
        game::cloud_input(&self.page.shell)
            .ok_or_else(|| js_error("cloud observation requires Shade or Sky"))?
            .observe_next_submission(ground);
        if ground && self.cloud_observation.is_none() {
            self.cloud_observation = Some(Rc::default());
        }
        Ok(())
    }

    /// Compare the frame's actual products with their CPU references.
    /// The caller waits for the result before allowing another frame to write them.
    pub fn inspect_cloud_submission(&self) -> Option<js_sys::Promise> {
        let frame = game::cloud_input(&self.page.shell)?.take_submission()?;
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
    let weather_ages = row_ages(
        &frame,
        &frame.weather_rows,
        &[config.weather.maximum_age; 4],
        config.weather.size,
    )?;
    let weather = crate::clouds::qualification::inspect_weather(
        &frame.device,
        &frame.queue,
        &config,
        crate::clouds::qualification::WeatherSnapshot {
            textures: &frame.weather,
            origins: &frame.weather_origins,
            rows: &frame.weather_rows,
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    let ground = if let Some(previous) = previous {
        let limits = config.sun.levels.map(|level| level.maximum_age);
        let ages = row_ages(&frame, &frame.sun_rows, &limits, config.sun.size)?;
        let replay = previous.borrow().has_new_rows(&frame);
        let reference = if replay {
            let texture = frame
                .replay_ground_slices()
                .ok_or("missing ground reference")?;
            Some(read_texture_bytes(&frame.device, &frame.queue, texture, 0, 4).await?)
        } else {
            None
        };
        let check = ground_error(&frame, &config.sun, reference.as_deref()).await?;
        let changes = previous.borrow_mut().observe(&frame, check.fingerprints)?;
        serde_json::json!({
            "max_depth_error": check.max_depth_error, "shade_texels": check.shade_texels,
            "reference_texels": check.reference_texels,
            "max_reference_depth_error": check.max_reference_depth_error,
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
        "weather_time": frame.state.time(), "weather_max_errors": weather.largest,
        "weather_normalized_errors": weather.normalized,
        "weather_texels": weather.counts, "weather_rows": weather_ages, "ground": ground,
        "visible_trace_texels": visible_texels,
    }))
}

#[derive(Default)]
pub(super) struct PreviousSun {
    generation: u64,
    levels: [Vec<(RowStamp, u64)>; 3],
}

impl PreviousSun {
    fn has_new_rows(&self, frame: &CloudSubmission) -> bool {
        self.generation != frame.generation
            || frame
                .sun_rows
                .iter()
                .zip(&self.levels)
                .any(|(stamps, previous)| {
                    stamps
                        .iter()
                        .zip(previous)
                        .any(|(stamp, (before, _))| stamp.tick != before.tick)
                })
    }

    fn observe(
        &mut self,
        frame: &CloudSubmission,
        fingerprints: [Vec<u64>; 3],
    ) -> Result<[[usize; 2]; 3], String> {
        let mut changes = [[0; 2]; 3];
        let same_device = self.generation == frame.generation;
        let next = fingerprints.into_iter().zip(&frame.sun_rows);
        for (level, ((fingerprints, stamps), old)) in next.zip(&mut self.levels).enumerate() {
            let rows = stamps.iter().copied().zip(fingerprints).collect::<Vec<_>>();
            for (row, &(stamp, fingerprint)) in rows.iter().enumerate() {
                // A retained row inherits the reference comparison of its write.
                // A new row must claim this submission's source, never an unseen tick.
                if same_device
                    && let Some(&(before, old_fingerprint)) = old.get(row)
                    && stamp.tick == before.tick
                {
                    if stamp.time.to_bits() != before.time.to_bits()
                        || stamp.parameters != before.parameters
                        || fingerprint != old_fingerprint
                    {
                        return Err(format!(
                            "Sun-path level {level} row {row}: retained source or value changed at tick {}",
                            stamp.tick
                        ));
                    }
                    changes[level][0] += 1;
                    continue;
                }
                if stamp.tick != frame.state.tick()
                    || stamp.time.to_bits() != frame.state.time().to_bits()
                    || stamp.parameters != frame.state.parameters()
                {
                    return Err(format!(
                        "Sun-path level {level} row {row}: new row claims tick {} at submission tick {}",
                        stamp.tick,
                        frame.state.tick()
                    ));
                }
                if same_device && let Some(&(_, old_fingerprint)) = old.get(row) {
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

struct GroundCheck {
    max_depth_error: f64,
    shade_texels: usize,
    fingerprints: [Vec<u64>; 3],
    reference_texels: [usize; 3],
    max_reference_depth_error: f64,
}

async fn ground_error(
    frame: &CloudSubmission,
    profile: &thrum_clouds::SunProfile,
    reference: Option<&[u8]>,
) -> Result<GroundCheck, String> {
    let size = profile.size as usize;
    let mut reference_texels = [0; 3];
    let mut max_reference_depth_error = 0.0_f64;
    if reference.is_some_and(|bytes| bytes.len() != 3 * size * size * 4) {
        return Err("incomplete ground reference readback".into());
    }
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
        if bytes.len() != size * size * 8 {
            return Err("incomplete Sun-path readback".into());
        }
        let depths = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|texel| depth(texel, 3))
            .collect::<Vec<_>>();
        if let Some(expected) = reference {
            let (count, maximum) = compare_ground_reference(frame, level, size, &depths, expected)?;
            reference_texels[level] = count;
            max_reference_depth_error = max_reference_depth_error.max(maximum);
        }
        sources.push(depths);
    }
    let pixels = read_texture_bytes(&frame.device, &frame.queue, &frame.ground, 0, 8).await?;
    let ground = profile.ground_size as usize;
    let half = ground / 2;
    if pixels.len() != ground * ground * 8 {
        return Err("incomplete ground readback".into());
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
        // Storage backends can round the interpolant to either adjacent binary16
        // value. Allow one output ULP plus the f32 interpolation's roundoff.
        let rounded = f16::from_f64(expected);
        let step = f16::from_bits(rounded.to_bits() + 1).to_f64() - rounded.to_f64();
        let bound = expected * (2.0 * f64::from(f32::EPSILON)) + step;
        if !error.is_finite()
            || actual < 0.0
            || (expected == 0.0 && actual != 0.0)
            || error > bound
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
        reference_texels,
        max_reference_depth_error,
    })
}

fn compare_ground_reference(
    frame: &CloudSubmission,
    level: usize,
    size: usize,
    actual: &[f64],
    expected: &[u8],
) -> Result<(usize, f64), String> {
    let expected = &expected.as_chunks::<4>().0[level * size * size..(level + 1) * size * size];
    let mut maximum = 0.0_f64;
    let mut count = 0;
    for (y, stamp) in frame.sun_rows[level].iter().enumerate() {
        if stamp.tick != frame.state.tick() {
            continue;
        }
        let row = y * size..(y + 1) * size;
        for (x, (&actual, &expected)) in actual[row.clone()].iter().zip(&expected[row]).enumerate()
        {
            let expected = f64::from(f32::from_le_bytes(expected));
            let error = (actual - expected).abs();
            let rounded = f16::from_f64(expected);
            let ulp = f16::from_bits(rounded.to_bits() + 1).to_f64() - rounded.to_f64();
            // A binary16 store can use either adjacent value. The independent
            // f32 kernels also differ in filtering and accumulation order.
            let tolerance = ulp + expected * 0.000_1 + 0.000_01;
            if !error.is_finite() || actual < 0.0 || error > tolerance {
                return Err(format!(
                    "Sun-path reference mismatch at level {level} texel {x},{y} tick {}: depth {actual}, expected {expected}, bound {tolerance}",
                    stamp.tick
                ));
            }
            maximum = maximum.max(error);
            count += 1;
        }
    }
    Ok((count, maximum))
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
