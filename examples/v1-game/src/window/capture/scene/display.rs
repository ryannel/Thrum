//! Frozen-world route checkpoints from the same submission that wrote the surface.
use super::{Driver, EXTENT, motion};
use crate::host::GameHost as _;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use thrum_render::{runtime::CaptureLimits, synthetic::CaptureSource};

pub(super) fn record(
    driver: &mut Driver,
    directory: &Path,
    files: &[PathBuf],
    headroom: Option<f32>,
    scene_linear: bool,
) -> Result<Vec<serde_json::Value>, String> {
    let source = if headroom.is_some() {
        CaptureSource::Edr
    } else if scene_linear {
        CaptureSource::SceneLinear
    } else {
        CaptureSource::Sdr
    };
    let target = motion::CaptureTarget::new(EXTENT);
    driver
        .host_mut()
        .set_display_headroom(headroom.unwrap_or(1.0));
    let mut records = Vec::new();
    for path in files {
        super::apply_still(driver.host_mut(), path)?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("still route name must be UTF-8")?
            .trim_end_matches(".params.json");
        let session = u64::try_from(records.len()).map_err(|error| error.to_string())?;
        driver.host_mut().enable_submitted_capture_from(
            session,
            CaptureLimits::default(),
            source,
        )?;
        let report = driver.step_at(Duration::ZERO)?;
        if !report.surface_presented || report.dropped_view {
            return Err(format!("still route refused {name}: {report:?}"));
        }
        let submission = driver.last_submission().ok_or("still was not submitted")?;
        let host = driver.host_mut();
        let image = target.submitted_image(host, submission, source)?;
        let suffix = if source == CaptureSource::Sdr {
            "rgba8"
        } else {
            "rgba16f"
        };
        let file = format!("{name}.{suffix}");
        std::fs::write(directory.join(&file), &image.rgba).map_err(|error| error.to_string())?;
        let [pitch, yaw] = host.camera_orientation();
        records.push(serde_json::json!({
            "file": file, "name": name, "source_tick": image.source.tick, "epoch": image.source.epoch,
            "submission": submission, "presenter_generation": image.presenter_generation,
            "exposure": image.exposure_gpu_words, "headroom": host.display_headroom(),
            "time_of_day": host.time_of_day(), "pitch": pitch, "yaw": yaw,
            "surface_presented": image.sample.surface_presented, "source": format!("{source:?}"),
        }));
        host.recycle_submitted_image(image);
        host.disable_submitted_capture();
    }
    Ok(records)
}
