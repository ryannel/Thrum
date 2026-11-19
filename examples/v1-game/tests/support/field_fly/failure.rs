//! Preserve a failing oracle's actual data even when Cargo/libtest output is truncated.

use std::io::Write;
use std::path::PathBuf;

#[derive(Clone, Copy, Default, serde::Serialize)]
pub(super) struct FrameObservation {
    tick: u64,
    presentation_alpha: f32,
    uploaded_rows: u32,
    moving: u32,
    dropped_presentation: u32,
    clamped_shadow_casters: u32,
    counts: Option<(u32, u32, u32)>,
}

impl From<&v1_game::FrameSample> for FrameObservation {
    fn from(sample: &v1_game::FrameSample) -> Self {
        Self {
            tick: sample.tick,
            presentation_alpha: sample.presentation_alpha,
            uploaded_rows: sample.uploaded_rows,
            moving: sample.moving,
            dropped_presentation: sample.dropped_presentation,
            clamped_shadow_casters: sample.clamped_shadow_casters,
            counts: sample
                .camera_culling
                .as_ref()
                .map(|c| (c.active, c.drawn, c.prepass)),
        }
    }
}

fn directory() -> PathBuf {
    std::env::var_os("THRUM_FIELD_FAILURE_DIR")
        .map_or_else(
            || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tmp/culling-m1/failures"),
            PathBuf::from,
        )
        .join(std::process::id().to_string())
}

pub(super) fn panic_record(
    meshes: bool,
    occlusion: bool,
    tick: u64,
    payload: &(dyn std::any::Any + Send),
) {
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("non-string panic payload");
    let directory = directory();
    let path = directory.join(format!(
        "meshes-{meshes}-occlusion-{occlusion}-{tick}-panic.json"
    ));
    let record = serde_json::json!({
        "field_meshes": meshes,
        "with_occlusion": occlusion,
        "tick": tick,
        "message": message,
    });
    let saved = std::fs::create_dir_all(&directory)
        .and_then(|()| std::fs::write(&path, serde_json::to_vec_pretty(&record)?));
    // Direct writes bypass libtest's thread-local print capture. The loop's
    // bounded Cargo stderr tail then retains the assertion and artifact path.
    let summary: String = message.chars().take(220).collect();
    let mut stderr = std::io::stderr().lock();
    drop(writeln!(
        stderr,
        "FIELD_ORACLE_FAILURE tick={tick}: {summary}"
    ));
    drop(writeln!(
        stderr,
        "record={} saved={saved:?}",
        path.display()
    ));
}

pub(super) fn pixels(
    mode: &str,
    tick: u64,
    direct: &[u8],
    culled: &[u8],
    extent: [u32; 2],
) -> std::io::Result<()> {
    let directory = directory();
    std::fs::create_dir_all(&directory)?;
    for (name, bytes) in [("direct", direct), ("culled", culled)] {
        let stem = directory.join(format!("{mode}-{tick}-{name}"));
        std::fs::write(stem.with_extension("rgba8"), bytes)?;
        super::write_ppm(&stem.with_extension("ppm"), bytes, extent);
    }
    let examples: Vec<_> = direct
        .as_chunks::<4>()
        .0
        .iter()
        .zip(culled.as_chunks::<4>().0)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .take(32)
        .map(|(index, (a, b))| {
            serde_json::json!({
                "x": index % extent[0] as usize,
                "y": index / extent[0] as usize,
                "direct": a,
                "culled": b,
            })
        })
        .collect();
    let record =
        serde_json::json!({"mode": mode, "tick": tick, "extent": extent, "examples": examples});
    std::fs::write(
        directory.join(format!("{mode}-{tick}-pixels.json")),
        serde_json::to_vec_pretty(&record)?,
    )
}

pub(super) fn context(
    mode: &str,
    tick: u64,
    frames: &[FrameObservation],
    direct_stable: bool,
    culled_stable: bool,
) -> std::io::Result<()> {
    let record = serde_json::json!({
        "mode": mode,
        "tick": tick,
        "frames": frames,
        "direct_stable": direct_stable,
        "culled_stable": culled_stable,
    });
    std::fs::write(
        directory().join(format!("{mode}-{tick}-context.json")),
        serde_json::to_vec_pretty(&record)?,
    )
}
