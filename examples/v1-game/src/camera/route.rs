//! The moving proof route: the authored valley camera as a pure function of the
//! committed tick and presentation alpha, loaded from a versioned route file.

use std::f64::consts::PI;

use serde::Deserialize;

use super::SceneCamera;
use crate::RouteOverlay;

/// The route file's format name; the held-preset loader refuses it and this loader refuses presets.
pub(crate) const ROUTE_FORMAT: &str = "thrum-v1-moving-route";
/// The only route file version this game reads.
pub(crate) const ROUTE_VERSION: u32 = 1;
/// Whole-run frame cap (pre-origin, routed and drain frames), below the web shell's
/// 600 retained samples per generation.
pub(crate) const ROUTE_FRAME_CAP: u64 = 560;
/// Drain steps after the last routed tick (`IN_FLIGHT` + 2).
pub(crate) const ROUTE_DRAIN_STEPS: u64 = 4;
/// Checkpoints must sit at least this many ticks after the origin and after each cut.
pub(crate) const CHECKPOINT_SETTLE_TICKS: u64 = 4;
const KEY0_TOLERANCE: f32 = 1e-4;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RouteFile {
    format: String,
    version: u32,
    cooked_root: String,
    cooked_generation: u64,
    tick_hz: u32,
    phase: [u32; 2],
    origin_tick: u64,
    start: StartFile,
    render_extent: [u32; 2],
    keys: Vec<KeyFile>,
    cuts: Vec<u64>,
    checkpoints: Vec<CheckpointFile>,
    overlay: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartFile {
    solar_hours: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyFile {
    tick: u64,
    eye: [f32; 3],
    yaw_degrees: f32,
    pitch_degrees: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointFile {
    tick: u64,
    name: String,
}

/// One camera key at an integer route tick (ticks after the origin).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RouteKey {
    /// Route tick, relative to the origin.
    pub tick: u64,
    /// Eye in world metres.
    pub eye: [f32; 3],
    /// Yaw in radians, the game's `camera.yaw` convention.
    pub yaw: f32,
    /// Pitch in radians, the game's `camera.pitch` convention.
    pub pitch: f32,
}

/// A named checkpoint at a route tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteCheckpoint {
    /// Route tick, relative to the origin.
    pub tick: u64,
    /// Unique name used in outputs.
    pub name: String,
}

/// The camera pose for one presented frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoutePose {
    /// Eye in world metres.
    pub eye: [f32; 3],
    /// Yaw in radians.
    pub yaw: f32,
    /// Pitch in radians.
    pub pitch: f32,
    /// History epoch: one from the origin (the unrouted view's epoch is zero, so the
    /// first routed frame resets history on every target), one more after each cut.
    pub epoch: u64,
}

/// A validated moving route.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraRoute {
    /// Canonical cooked root the native target checks.
    pub cooked_root: String,
    /// Cooked generation both targets check.
    pub cooked_generation: u64,
    /// Fixed in-tick phase as a fraction (numerator, denominator).
    pub phase: [u32; 2],
    /// Absolute committed tick of route tick zero.
    pub origin_tick: u64,
    /// Solar hour at the origin on both targets.
    pub solar_hours: f32,
    /// Render extent on both targets.
    pub render_extent: [u32; 2],
    /// Keys in order; a cut is two consecutive keys at one tick.
    pub keys: Vec<RouteKey>,
    /// Route ticks of the cuts.
    pub cuts: Vec<u64>,
    /// Checkpoints in tick order.
    pub checkpoints: Vec<RouteCheckpoint>,
    /// The proof overlay the game must be constructed with.
    pub overlay: RouteOverlay,
}

impl CameraRoute {
    /// Parse and validate a route file against the authored camera it moves.
    /// # Errors
    /// Refuses another format or version, a clock rate other than `tick_hz`, any
    /// phase but one half, a malformed key or cut sequence, a yaw step over π, a key
    /// the authored camera refuses, an unsettled or out-of-range checkpoint, or a run
    /// over the frame cap.
    pub fn parse(text: &str, camera: SceneCamera, tick_hz: u32) -> Result<Self, String> {
        let file: RouteFile =
            serde_json::from_str(text).map_err(|error| format!("route file: {error}"))?;
        if file.format != ROUTE_FORMAT || file.version != ROUTE_VERSION {
            return Err(format!(
                "route file must be {ROUTE_FORMAT} version {ROUTE_VERSION}"
            ));
        }
        if file.tick_hz != tick_hz {
            return Err(format!(
                "route tick_hz {} is not the clock's {tick_hz}",
                file.tick_hz
            ));
        }
        if file.phase != [1, 2] {
            return Err("route phase must be [1, 2]".to_owned());
        }
        if !(0.0..24.0).contains(&file.start.solar_hours) {
            return Err("route start.solar_hours must be within 0–24".to_owned());
        }
        if file.render_extent.contains(&0) {
            return Err("route render_extent must be nonzero".to_owned());
        }
        let overlay = overlay(&file.overlay)?;
        let keys: Vec<RouteKey> = file
            .keys
            .iter()
            .map(|key| RouteKey {
                tick: key.tick,
                eye: key.eye,
                yaw: key.yaw_degrees.to_radians(),
                pitch: key.pitch_degrees.to_radians(),
            })
            .collect();
        validate_keys(&keys, &file.cuts, camera)?;
        let last = keys.last().map_or(0, |key| key.tick);
        let frames = file
            .origin_tick
            .saturating_add(last)
            .saturating_add(ROUTE_DRAIN_STEPS);
        if frames > ROUTE_FRAME_CAP {
            return Err(format!(
                "route needs {frames} frames, over the {ROUTE_FRAME_CAP}-frame cap"
            ));
        }
        let mut checkpoints: Vec<RouteCheckpoint> = file
            .checkpoints
            .into_iter()
            .map(|checkpoint| RouteCheckpoint {
                tick: checkpoint.tick,
                name: checkpoint.name,
            })
            .collect();
        checkpoints.sort_by_key(|checkpoint| checkpoint.tick);
        for (index, checkpoint) in checkpoints.iter().enumerate() {
            let settled = std::iter::once(0)
                .chain(file.cuts.iter().copied())
                .all(|start| {
                    checkpoint.tick < start || checkpoint.tick >= start + CHECKPOINT_SETTLE_TICKS
                });
            if !settled || checkpoint.tick > last {
                return Err(format!(
                    "checkpoint {} must be within the route and {CHECKPOINT_SETTLE_TICKS} ticks after the origin and each cut",
                    checkpoint.name
                ));
            }
            if checkpoint.name.is_empty()
                || checkpoints[..index]
                    .iter()
                    .any(|other| other.name == checkpoint.name)
            {
                return Err("checkpoint names must be unique and nonempty".to_owned());
            }
        }
        Ok(Self {
            cooked_root: file.cooked_root,
            cooked_generation: file.cooked_generation,
            phase: file.phase,
            origin_tick: file.origin_tick,
            solar_hours: file.start.solar_hours,
            render_extent: file.render_extent,
            keys,
            cuts: file.cuts,
            checkpoints,
            overlay,
        })
    }

    /// The render extent and overlay a route file declares, read before the game is
    /// prepared so the host can be built to match. The full parse still validates them.
    /// # Errors
    /// Refuses a file that is not a route or an unknown overlay.
    pub fn declared(text: &str) -> Result<([u32; 2], RouteOverlay), String> {
        let file: RouteFile =
            serde_json::from_str(text).map_err(|error| format!("route file: {error}"))?;
        if file.format != ROUTE_FORMAT {
            return Err(format!("route file must be {ROUTE_FORMAT}"));
        }
        Ok((file.render_extent, overlay(&file.overlay)?))
    }

    /// Check that key 0 equals the authored camera's current pose.
    /// # Errors
    /// Names the differing component.
    pub fn check_start(&self, camera: SceneCamera, pitch: f32, yaw: f32) -> Result<(), String> {
        let key = self.keys[0];
        let eye = camera.eye();
        let close = |a: f32, b: f32| (a - b).abs() <= KEY0_TOLERANCE;
        if !(0..3).all(|axis| close(key.eye[axis], eye[axis])) {
            return Err("route key 0 eye differs from the authored camera".to_owned());
        }
        if !close(key.yaw, yaw) || !close(key.pitch, pitch) {
            return Err("route key 0 orientation differs from the authored camera".to_owned());
        }
        Ok(())
    }

    /// The pose for committed tick `tick` presented at `alpha`, using the same
    /// previous-to-current interpolation as the field flythrough: route time
    /// `s = (tick − 1 − origin) + alpha`, clamped to the first and last keys.
    #[must_use]
    pub fn pose(&self, tick: u64, alpha: f32) -> RoutePose {
        #[expect(
            clippy::cast_precision_loss,
            reason = "route ticks are bounded by the 560-frame cap"
        )]
        let s = (tick as f64 - 1.0 - self.origin_tick as f64 + f64::from(alpha)).max(0.0);
        self.pose_at(s)
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "route ticks are bounded by the 560-frame cap"
    )]
    fn pose_at(&self, s: f64) -> RoutePose {
        let epoch = 1 + self.cuts.iter().filter(|&&cut| s >= cut as f64).count() as u64;
        let mut from = self.keys[0];
        let mut to = from;
        let mut t = 0.0;
        if let Some(pair) = self
            .keys
            .windows(2)
            .find(|pair| pair[0].tick < pair[1].tick && s < pair[1].tick as f64)
        {
            from = pair[0];
            to = pair[1];
            t = ((s - from.tick as f64) / (to.tick - from.tick) as f64).clamp(0.0, 1.0);
        } else if let Some(last) = self.keys.last() {
            from = *last;
            to = *last;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "lerp in f64, stored as f32"
        )]
        let lerp = |a: f32, b: f32| (f64::from(a) + (f64::from(b) - f64::from(a)) * t) as f32;
        RoutePose {
            eye: std::array::from_fn(|axis| lerp(from.eye[axis], to.eye[axis])),
            yaw: lerp(from.yaw, to.yaw),
            pitch: lerp(from.pitch, to.pitch),
            epoch,
        }
    }
}

fn overlay(name: &str) -> Result<RouteOverlay, String> {
    match name {
        "off" => Ok(RouteOverlay::Off),
        "proof" => Ok(RouteOverlay::Proof),
        "motion" => Ok(RouteOverlay::Motion),
        "unmirrored" => Ok(RouteOverlay::Unmirrored),
        "light-off" => Ok(RouteOverlay::LightOff),
        _ => Err("route overlay must be off, proof, unmirrored or light-off".to_owned()),
    }
}

fn validate_keys(keys: &[RouteKey], cuts: &[u64], camera: SceneCamera) -> Result<(), String> {
    if keys.first().map(|key| key.tick) != Some(0) {
        return Err("route keys must start at route tick 0".to_owned());
    }
    if cuts.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("route cuts must be strictly increasing".to_owned());
    }
    for key in keys {
        let finite = key
            .eye
            .iter()
            .chain([&key.yaw, &key.pitch])
            .all(|v| v.is_finite());
        if !finite || key.pitch.abs() >= std::f32::consts::FRAC_PI_2 {
            return Err(format!(
                "route key at tick {} is not a usable pose",
                key.tick
            ));
        }
        camera
            .at(key.eye)
            .map_err(|error| format!("route key at tick {}: {error}", key.tick))?;
    }
    let mut pairs = Vec::new();
    for (index, pair) in keys.windows(2).enumerate() {
        let [a, b] = [pair[0], pair[1]];
        if a.tick > b.tick {
            return Err("route keys must not go back in time".to_owned());
        }
        if a.tick == b.tick {
            if !cuts.contains(&a.tick) || (index > 0 && pairs.last() == Some(&(index - 1))) {
                return Err(format!(
                    "two keys at tick {} must be a declared cut pair",
                    a.tick
                ));
            }
            pairs.push(index);
        } else if f64::from((b.yaw - a.yaw).abs()) > PI {
            return Err(format!(
                "yaw step over π between ticks {} and {}",
                a.tick, b.tick
            ));
        }
    }
    if pairs.len() != cuts.len() {
        return Err("each route cut must be one key pair".to_owned());
    }
    if keys.last().map(|key| key.tick) == cuts.last().copied() {
        return Err("a route must continue after its last cut".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
