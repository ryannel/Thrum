//! Scripted target qualification shared by the native runner and browser case.
use super::{DiagnosticEffect, EffectHandle, SIZE};
use std::{
    sync::{Arc, Mutex},
    task::Poll,
    time::Duration,
};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{FrameConfig, FrameHost, FrameWorld, SceneFrame, SceneSource},
    synthetic::{FrameLighting, SyntheticLight, SyntheticScene, SyntheticView},
};

struct World(SourceTick);
impl FrameWorld for World {
    type Page = SourceTick;
    fn source(&self) -> SourceTick {
        self.0
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, _: Duration) -> Result<WorldAdvance, String> {
        Ok(WorldAdvance {
            tick: self.0.tick,
            overload: None,
            completed_ticks: 0,
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: false,
            tick_cpu: Duration::ZERO,
        })
    }
    fn publication(&mut self) -> Result<WorldPublication<SourceTick>, String> {
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(self.0.epoch),
                tick: self.0.tick,
            },
            timeline: self.0.timeline,
            page: self.0,
        })
    }
    fn presentation_alpha_at(&self, _: Duration) -> f32 {
        1.0
    }
}
struct Scene;
impl SceneSource<SourceTick> for Scene {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.; 3], |sink| {
            sink.push(0, 1, 1, [0., 0., 0.], 0);
        })
    }
    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, String> {
        Ok(SceneFrame {
            view: SyntheticView::default(),
            lighting: FrameLighting::Diagnostic(SyntheticLight::default()),
        })
    }
}
fn error(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}

/// Read every texel of one texture and mip, stripping GPU row padding.
/// # Panics
/// Panics if the private readback callback mutex is poisoned.
/// # Errors
/// Returns GPU mapping or device progress errors.
pub async fn read_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    mip: u32,
) -> Result<Vec<u32>, String> {
    let width = (texture.width() >> mip).max(1);
    let height = (texture.height() >> mip).max(1);
    let depth = if texture.dimension() == wgpu::TextureDimension::D3 {
        (texture.depth_or_array_layers() >> mip).max(1)
    } else {
        texture.depth_or_array_layers()
    };
    let row = (width * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("effect exact readback"),
        size: u64::from(row * height * depth),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: mip,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: depth,
        },
    );
    queue.submit([encoder.finish()]);
    let shared = Arc::new(Mutex::new((None, None::<std::task::Waker>)));
    let callback = Arc::clone(&shared);
    buffer.map_async(wgpu::MapMode::Read, .., move |result| {
        let mut slot = callback.lock().expect("readback callback");
        slot.0 = Some(result);
        if let Some(waker) = slot.1.take() {
            waker.wake();
        }
    });
    #[cfg(not(target_arch = "wasm32"))]
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(30)),
        })
        .map_err(error)?;
    std::future::poll_fn(|cx| {
        let mut slot = shared.lock().expect("readback future");
        if let Some(result) = slot.0.take() {
            Poll::Ready(result)
        } else {
            slot.1 = Some(cx.waker().clone());
            Poll::Pending
        }
    })
    .await
    .map_err(error)?;
    let mapped = buffer.get_mapped_range(..).map_err(error)?;
    let mut values = Vec::new();
    for bytes in mapped.chunks_exact(row as usize) {
        for value in bytes[..(width * 4) as usize].as_chunks::<4>().0 {
            values.push(u32::from_le_bytes(*value));
        }
    }
    drop(mapped);
    buffer.unmap();
    Ok(values)
}

// This reference is a literal recurrence; the shader uses a bounded spatial maximum.
fn advance(previous: &[u32], from: u64, to: u64, source: bool) -> Vec<u32> {
    let mut field = previous.to_vec();
    for tick in (to - (to - from).min(8) + 1)..=to {
        let old = field;
        field = vec![0; 1024];
        for y in 0..32usize {
            for x in 0..32usize {
                field[y * 32 + x] = old[y * 32 + x].saturating_sub(2).max(
                    [
                        ((x + 31) % 32, y),
                        ((x + 1) % 32, y),
                        (x, (y + 31) % 32),
                        (x, (y + 1) % 32),
                    ]
                    .iter()
                    .map(|&(a, b)| old[b * 32 + a].saturating_sub(32))
                    .max()
                    .unwrap_or(0),
                );
            }
        }
        if source {
            field[16 * 32 + ((tick / 3) % 32) as usize] = 255;
        }
    }
    field
}

async fn fields(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    handle: &EffectHandle,
) -> Result<Vec<Vec<u32>>, String> {
    let textures = {
        let witness = handle.lock().map_err(error)?;
        let a = witness
            .state
            .side(0, thrum_render::contribution::HistoryBinding::Previous);
        let b = witness
            .state
            .side(1, thrum_render::contribution::HistoryBinding::Previous);
        vec![
            witness.textures[0][a].clone(),
            witness.textures[1][b].clone(),
            witness.textures[2][0].clone(),
        ]
    };
    let mut result = Vec::new();
    for texture in &textures {
        result.push(read_texture(device, queue, texture, 0).await?);
    }
    Ok(result)
}

async fn reset_kinds(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    handle: &EffectHandle,
) -> Result<(), String> {
    let textures = handle.lock().map_err(error)?.textures.clone();
    for (resource, sides) in textures.iter().enumerate().skip(3) {
        for mip in 0..sides[0].mip_level_count() {
            let texture = &sides[0];
            let width = (texture.width() >> mip).max(1);
            let height = (texture.height() >> mip).max(1);
            let got = read_texture(device, queue, texture, mip).await?;
            for (at, value) in got.into_iter().enumerate() {
                let at = u32::try_from(at).map_err(error)?;
                let expected =
                    at % width + 16 * ((at / width) % height) + 256 * (at / (width * height));
                if value != expected {
                    return Err(format!(
                        "kind resource {resource} mip {mip} texel {at}: got {value} expected {expected}"
                    ));
                }
            }
        }
    }
    Ok(())
}

async fn replace_device(
    host: &mut FrameHost<World, Scene>,
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    generation: u64,
    handle: &EffectHandle,
) -> Result<(), String> {
    let inputs = host.device_prepare_inputs().map_err(error)?;
    let (replacement, replacement_queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(error)?;
    let next_caps = DeviceCapabilities::capture(adapter, &replacement, generation + 1);
    let mut prepared = inputs
        .prepare(
            &next_caps,
            &replacement_queue,
            Some(SurfaceProfile {
                format: wgpu::TextureFormat::Bgra8Unorm,
                extent: [80, 64],
            }),
        )
        .await
        .map_err(error)?;
    device.destroy();
    host.publish_device(&mut prepared).map_err(error)?;
    host.world_mut().0.tick = 101;
    let report = host
        .advance(Duration::from_secs(20))
        .map_err(error)?
        .encode_scene()
        .map_err(error)?
        .submit(None)
        .map_err(error)?;
    if !report.submitted {
        return Err("replacement frame was not submitted".into());
    }
    let reset = fields(&replacement, &replacement_queue, handle).await?;
    if reset
        .iter()
        .any(|field| field.iter().any(|value| *value != 0))
    {
        return Err("replacement did not seed the exact declared reset".into());
    }
    if handle.lock().map_err(error)?.generation != next_caps.generation() {
        return Err("old generation remained attached".into());
    }
    reset_kinds(&replacement, &replacement_queue, handle).await?;
    Ok(())
}

/// Run the explicit state-table script through the real frame host and GPU.
/// # Errors
/// Returns the first exact mismatch, with the script frame and resource named.
pub async fn run(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    adapter: &wgpu::Adapter,
) -> Result<String, String> {
    let source = SourceTick {
        epoch: 7,
        timeline: 3,
        tick: 10,
    };
    let mut host = FrameHost::new(
        caps,
        queue,
        FrameConfig {
            render_extent: [64, 64],
            surface: Some(SurfaceProfile {
                format: wgpu::TextureFormat::Bgra8Unorm,
                extent: [64, 64],
            }),
        },
        World(source),
        Scene,
        SyntheticScene::grid(1).map_err(error)?,
        1.0,
    )
    .await
    .map_err(error)?;
    let (effect, handle) = DiagnosticEffect::new();
    host.set_scene_contribution(Box::new(effect))?;
    let mut a = vec![0; 1024];
    let mut b = a.clone();
    let mut committed = 10;
    // Literal table rows: seed, later, equal, pause, discard, retry, capture,
    // bounded stall, backward, later, new timeline, resize with an equal tick.
    let script = [
        (10, 3, "seed"),
        (11, 3, "later"),
        (11, 3, "equal"),
        (11, 3, "pause"),
        (12, 3, "discard"),
        (11, 3, "after-discard-skip"),
        (12, 3, "retry"),
        (13, 3, "cadence-skip"),
        (14, 3, "cadence-run"),
        (15, 3, "cadence-skip"),
        (16, 3, "cadence-run"),
        (100, 9, "capture"),
        (100, 3, "stall"),
        (99, 3, "backward"),
        (100, 3, "later"),
        (100, 4, "timeline"),
        (100, 4, "resize"),
    ];
    let mut records = vec![format!(
        "adapter={:?}; generation={}",
        caps.adapter(),
        caps.generation()
    )];
    let pipeline_count = host.pipeline_creation_count();
    for (index, (tick, timeline, kind)) in script.into_iter().enumerate() {
        host.world_mut().0.tick = tick;
        host.world_mut().0.timeline = timeline;
        if kind == "after-discard-skip" {
            handle.lock().map_err(error)?.controls.anchor = [0, 0];
        }
        if kind == "later" && tick == 100 {
            handle.lock().map_err(error)?.controls.source = true;
        }
        if kind == "discard" {
            handle.lock().map_err(error)?.controls.anchor = [64, 0];
        }
        if kind == "retry" {
            handle.lock().map_err(error)?.controls.anchor = [32, 0];
        }
        if kind == "capture" {
            handle.lock().map_err(error)?.controls.anchor = [128, 0];
        }
        if kind == "stall" {
            handle.lock().map_err(error)?.controls.anchor = [32, 0];
        }
        if kind.starts_with("cadence-") {
            handle.lock().map_err(error)?.controls.cadence = 2;
        }
        if kind != "resize" && host.pipeline_creation_count() != pipeline_count {
            return Err("runtime cadence created a pipeline".into());
        }
        if kind == "resize" {
            host.resize_scene_surface([80, 64]).map_err(error)?;
        }
        if kind == "pause" {
            handle.lock().map_err(error)?.controls.source = false;
        }
        if kind == "discard" {
            let token = host
                .advance(Duration::from_secs(index as u64))
                .map_err(error)?
                .encode_scene()
                .map_err(error)?;
            if !token.can_acquire_surface() {
                return Err("discard fixture encoded no scene".into());
            }
            drop(token);
        } else if kind == "capture" {
            let _capture = host.begin_capture().map_err(error)?;
        } else {
            let report = host
                .advance(Duration::from_secs(index as u64))
                .map_err(error)?
                .encode_scene()
                .map_err(error)?
                .submit(None)
                .map_err(error)?;
            if !report.submitted {
                return Err(format!("{kind}: no submission"));
            }
            // Readback below is queue ordered after this scene, and gives it time to retire.
        }
        if matches!(kind, "seed" | "backward" | "timeline") {
            a.fill(0);
            b.fill(0);
            committed = tick;
        } else if !matches!(
            kind,
            "equal" | "pause" | "discard" | "after-discard-skip" | "capture" | "resize"
        ) {
            let source_on = handle.lock().map_err(error)?.controls.source;
            a = advance(&a, committed, tick, source_on);
            committed = tick;
            if tick.is_multiple_of(handle.lock().map_err(error)?.controls.cadence) {
                b = a.iter().map(|value| value * 3).collect();
            }
        }
        let actual = fields(device, queue, &handle).await?;
        if matches!(kind, "seed" | "backward" | "timeline") {
            reset_kinds(device, queue, &handle).await?;
        }
        if matches!(kind, "discard" | "after-discard-skip")
            && handle.lock().map_err(error)?.state.anchor() != [0, 0]
        {
            return Err("discard committed its anchor".into());
        }
        if matches!(kind, "retry" | "capture")
            && handle.lock().map_err(error)?.state.anchor() != [32, 0]
        {
            return Err("anchor differs from the last submitted one".into());
        }
        for (resource, (got, expected)) in actual
            .iter()
            .zip([
                a.clone(),
                b.clone(),
                a.iter().zip(&b).map(|(a, b)| a + 1024 * b).collect(),
            ])
            .enumerate()
        {
            if *got != expected {
                let at = got
                    .iter()
                    .zip(&expected)
                    .position(|(a, b)| a != b)
                    .unwrap_or(0);
                return Err(format!(
                    "frame {index} {kind} resource {resource} texel {at}: got {} expected {}",
                    got[at], expected[at]
                ));
            }
        }
        if kind == "later" && matches!(tick, 11 | 100) && !a.iter().any(|v| *v > 0) {
            return Err("repeated-tick witness was empty".into());
        }
        drop(host.poll_completed().map_err(error)?);
        records.push(format!("{kind}:{tick}:exact"));
    }
    if handle.lock().map_err(error)?.state.dropped_ticks() != 76 {
        return Err("stall dropped-tick count is not 76".into());
    }
    // The last frame reset at tick 100. Every texel of all kinds has a literal expected value.
    reset_kinds(device, queue, &handle).await?;
    Box::pin(replace_device(
        &mut host,
        adapter,
        device,
        caps.generation(),
        &handle,
    ))
    .await?;
    records.push("destroyed-device:recovered:exact".into());
    records.push(format!(
        "cadence pipeline count={pipeline_count:?}; texture bytes=23712; uniforms=128"
    ));
    Ok(format!(
        "{}; all cube faces, volume texels and array mips exact; width={SIZE}",
        records.join("; ")
    ))
}
