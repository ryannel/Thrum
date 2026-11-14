//! Governor-facing controls and executor timing outcomes, independent of the state oracle.
use super::{Duration, EffectHandle, FrameHost, Scene, World, error, fields};

pub(super) async fn run(
    host: &mut FrameHost<World, Scene>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    handle: &EffectHandle,
) -> Result<String, String> {
    let knobs = host.quality_knobs();
    if knobs.len() != 3 || knobs.iter().any(|knob| !knob.range.contains(&knob.value)) {
        return Err("quality discovery/range".into());
    }
    let count = host.pipeline_creation_count();
    let old = knobs[0].value;
    host.set_quality_knob(&knobs[0].name, 7)?;
    if host.quality_knobs()[0].value != old {
        return Err("quality changed outside a frame boundary".into());
    }
    if host.set_quality_knob(&knobs[0].name, 0).is_ok()
        || host.set_quality_knob("unknown", 1).is_ok()
    {
        return Err("quality accepted an invalid request".into());
    }
    host.advance(Duration::from_secs(200))
        .map_err(error)?
        .encode_scene()
        .map_err(error)?
        .submit(None)
        .map_err(error)?;
    if host.quality_knobs()[0].value != 7 {
        return Err("quality did not commit at boundary".into());
    }
    drop(fields(device, queue, handle).await?);
    let mut timing = None;
    for _ in 0..8 {
        if let Some(sample) = host.poll_completed().map_err(error)? {
            timing = sample.gpu_pass_ns;
            break;
        }
        drop(fields(device, queue, handle).await?);
    }
    if device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
        let timing = timing.ok_or("missing native timing outcome")?;
        if timing.execution("effect 0: trail") != Some(thrum_render::timing::PassExecution::NotRun)
        {
            return Err(format!(
                "equal-tick producer must report not run: {:?}",
                timing.execution("effect 0: trail")
            ));
        }
        if !matches!(
            timing.execution("effect 2: consumer"),
            Some(thrum_render::timing::PassExecution::Timed(_))
        ) {
            return Err("consumer did not report executed timing".into());
        }
    }
    host.set_quality_knob(&knobs[1].name, 2)?;
    host.world_mut().0.tick = 101;
    host.advance(Duration::from_secs(201))
        .map_err(error)?
        .encode_scene()
        .map_err(error)?
        .submit(None)
        .map_err(error)?;
    let reset = fields(device, queue, handle).await?;
    if reset.iter().flatten().any(|value| *value != 0) {
        return Err("effect-history quality change did not seed every history texel".into());
    }
    drop(host.poll_completed().map_err(error)?);
    host.set_quality_knob(&knobs[2].name, 4)?;
    host.world_mut().0.tick = 102;
    host.advance(Duration::from_secs(202))
        .map_err(error)?
        .encode_scene()
        .map_err(error)?
        .submit(None)
        .map_err(error)?;
    let reset = fields(device, queue, handle).await?;
    if reset.iter().flatten().any(|value| *value != 0)
        || !handle.lock().map_err(error)?.history_reset
    {
        return Err("view-history quality change did not reach effect reset".into());
    }
    if host.pipeline_creation_count() != count {
        return Err("quality created pipelines".into());
    }
    drop(host.poll_completed().map_err(error)?);
    Ok(format!(
        "quality knobs={knobs:?}: staged boundary, none/effect/view invalidations, skipped producer versus measured consumer, unchanged pipeline count={count:?}"
    ))
}
