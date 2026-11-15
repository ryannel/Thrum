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
    host.set_quality_knob(&knobs[0].name, 7).map_err(error)?;
    if host.quality_knobs()[0].value != old {
        return Err("quality changed outside a frame boundary".into());
    }
    if host.set_quality_knob(&knobs[0].name, 0).is_ok()
        || host.set_quality_knob("unknown", 1).is_ok()
    {
        return Err("quality accepted an invalid request".into());
    }
    host.advance(Duration::from_secs(17))
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
    let mut timing_record = "timestamps unsupported by enabled device".to_owned();
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
        timing_record = format!(
            "effect 0: trail={:?}; effect 2: consumer={:?}",
            timing.execution("effect 0: trail"),
            timing.execution("effect 2: consumer")
        );
    }
    host.set_quality_knob(&knobs[1].name, 2).map_err(error)?;
    host.world_mut().0.tick = 101;
    host.advance(Duration::from_secs(18))
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
    host.set_quality_knob(&knobs[2].name, 4).map_err(error)?;
    host.world_mut().0.tick = 102;
    host.advance(Duration::from_secs(19))
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
    let dropped = handle.lock().map_err(error)?.state.dropped_ticks();
    drop(host.poll_completed().map_err(error)?);
    host.world_mut().0.tick = 200;
    host.advance(Duration::from_secs(20))
        .map_err(error)?
        .encode_scene()
        .map_err(error)?
        .submit(None)
        .map_err(error)?;
    let volume = handle.lock().map_err(error)?.textures[3][0].clone();
    let texels = super::read_texture(device, queue, &volume, 0).await?;
    for z in 0..4usize {
        for y in 0..8usize {
            for x in 0..8usize {
                let base = if z < 2 { 200 } else { 0 };
                let expected = u32::try_from(base + x + 16 * y + 256 * z).map_err(error)?;
                if texels[z * 64 + y * 8 + x] != expected {
                    return Err(
                        "volume-slices quality value did not select exactly two slices".into(),
                    );
                }
            }
        }
    }
    if handle.lock().map_err(error)?.state.dropped_ticks() - dropped != 94 {
        return Err("catch-up quality value did not bound the 98-tick stall to four steps".into());
    }
    if host.pipeline_creation_count() != count {
        return Err("quality created pipelines".into());
    }
    drop(host.poll_completed().map_err(error)?);
    Ok(format!(
        "quality knobs={knobs:?}: staged boundary, none/effect/view invalidations, two volume slices and four-step catch-up verified, {timing_record}, unchanged pipeline count={count:?}"
    ))
}
