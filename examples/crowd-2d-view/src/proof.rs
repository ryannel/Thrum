//! Record the identities and costs of the frame samples produced by the shared view.

use serde_json::{Value, json};
use thrum_render::runtime::{FrameError, FrameHost, FrameSample};

use crate::{CrowdScene, CrowdWorld};

/// Report the immutable publication currently extracted into sprite rows.
#[must_use]
pub fn publication(host: &FrameHost<CrowdWorld, CrowdScene>, digest: bool) -> Value {
    let row = host.source().extraction();
    json!({
        "tick": row.tick, "live": row.live, "mapped": row.mapped,
        "reused": row.reused, "skipped_interpolation": row.skipped,
        "digest": digest.then(|| format!("{:016x}", host.world().digest())),
    })
}

/// Actual draw and timing data for a completed submission.
#[must_use]
pub fn sample(frame: &FrameSample) -> Value {
    let stats = frame.sprites;
    let timing = frame.gpu_pass_ns.as_ref();
    json!({
        "tick": frame.tick, "submission": frame.submission_order, "extent": frame.render_extent,
        "alpha": frame.presentation_alpha, "observation_ns": frame.observation.as_nanos(),
        "world_rows": stats.map(|s| s.world_rows), "draws": stats.map(|s| s.draws),
        "shaded_pixels": stats.map(|s| s.shaded_pixels), "tick_ns": frame.tick_cpu.as_nanos(),
        "extract_ns": frame.extract_cpu.as_nanos(), "order_ns": stats.map(|s| s.order_cpu.as_nanos()),
        "upload_bytes": frame.uploaded_bytes, "sprite_gpu_ns": timing.and_then(|t| t.get("sprites")),
        "frame_cpu_ns": frame.wall.as_nanos(),
        "frame_gpu_ns": timing.and_then(thrum_render::timing::SceneGpuTimings::timed_pass_span_nanoseconds),
        "timing_submission": timing.and_then(thrum_render::timing::SceneGpuTimings::frame).map(|t| t.submission_order),
        "presented": frame.surface_presented, "dropped": frame.dropped_presentation,
        "timing_requested": frame.timing_requested, "timing_fault": frame.timing_fault,
    })
}

/// List the compiled passes of this replay's prepared 2D scene.
///
/// # Errors
/// Returns a failure to inspect the current graph.
pub fn passes(host: &FrameHost<CrowdWorld, CrowdScene>) -> Result<Vec<String>, FrameError> {
    Ok(host
        .scene_graph()?
        .pass_names()
        .map(str::to_owned)
        .collect())
}

/// Diagnostic rectangle of one committed world row, before interpolation.
#[derive(Clone, Copy)]
pub struct BodyRect {
    /// Physical component slot.
    pub slot: usize,
    /// Stable body identity, so recycled slots start with fresh history.
    pub identity: u128,
    /// World-space bounding centre.
    pub centre: [f64; 2],
    /// Square sprite half-size at native row scale.
    pub half: f64,
    /// Actual world sort value.
    pub sort: f64,
    /// Actual sprite layer.
    pub layer: i32,
}

/// Visit committed rectangles without consulting the renderer's order.
///
/// # Errors
/// Returns a missing-publication error.
///
/// # Panics
/// The reference rule package failed to provide its registered body column.
pub fn rectangles(world: &mut CrowdWorld, output: &mut Vec<BodyRect>) -> Result<(), FrameError> {
    use thrum_render::runtime::FrameWorld;
    let page = world.publication()?.page;
    output.clear();
    page.visit(|handle, identity| {
        let body = page
            .component::<crowd_2d::Body>(handle)
            .expect("registered body column");
        let row = crate::scene::row(body, identity.0);
        let half = f64::from(row.scale[0]);
        output.push(BodyRect {
            slot: handle.slot as usize,
            identity: identity.0,
            centre: [row.position[0] + half, row.position[1] - half],
            half,
            sort: row.sort.expect("explicit body sort"),
            layer: row.layer,
        });
    });
    Ok(())
}

/// World-space band used by the crowd view.
pub const ORDER_BAND: f64 = crate::scene::ORDER_BAND;

/// Clock and source metadata from the renderer-owned submitted-image tap.
#[must_use]
pub fn image_sample(image: &thrum_render::runtime::SubmittedImage) -> Value {
    let mut value = sample(&image.sample);
    value["submit_ns"] = json!(image.submitted_elapsed.as_nanos());
    value["device_generation"] = json!(image.device_generation);
    value["world_epoch"] = json!(image.source.epoch.to_string());
    value["timeline"] = json!(image.source.timeline);
    value
}
