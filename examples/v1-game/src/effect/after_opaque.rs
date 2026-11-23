//! Shared Metal/WebGPU slab qualification, using submitted scene inputs.
use super::qualification::World;
use super::{DiagnosticEffect, qualification::read_texture_bytes};
use std::time::Duration;
use thrum_render::{
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameConfig, FrameError, FrameHost, SceneFrame, SceneSource},
    synthetic::{
        DisplayOverlay, FrameLighting, Reconstruction, SyntheticLight, SyntheticScene,
        SyntheticView,
    },
};

/// A failure in the submitted-input diagnostic, including its GPU preparation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QualificationError {
    /// A check could not complete or its submitted result differed from the expectation.
    CheckFailed(String),
}

impl std::fmt::Display for QualificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CheckFailed(detail) => write!(f, "after-opaque qualification failed: {detail}"),
        }
    }
}

impl std::error::Error for QualificationError {}

pub(super) struct Scene {
    temporal: bool,
    reconstruction: bool,
}
impl SceneSource<SourceTick> for Scene {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        let x = if page.tick < 12 { -1.0 } else { 1.0 };
        into.extract(*page, [0.; 3], |sink| {
            sink.push(0, 1, 1, [x, 2., 2.], 0);
            sink.push(1, 2, 1, [0., 2., -2.], 0);
        })
    }
    fn frame(
        &self,
        source: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        Ok(SceneFrame {
            view: SyntheticView::perspective(
                [0., 2., 10.],
                [0., 2., 0.],
                [0., 1., 0.],
                std::f32::consts::FRAC_PI_3,
                1.,
                0.1,
                100.,
            )?
            .with_temporal_inputs(self.temporal)
            .with_reconstruction(self.reconstruction)
            .with_display_overlay(if self.temporal {
                DisplayOverlay::TemporalInputs
            } else {
                DisplayOverlay::Off
            })
            .with_jitter_phase(0, [64, 64])
            .with_history_reset(source.tick == 14),
            lighting: FrameLighting::Diagnostic(SyntheticLight::default()),
        })
    }
}
fn error(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}
pub(super) fn half(bytes: &[u8]) -> f32 {
    let n = u16::from_le_bytes([bytes[0], bytes[1]]);
    let sign = if n & 0x8000 == 0 { 1. } else { -1. };
    let exponent = i32::from((n >> 10) & 31);
    let mantissa = f32::from(n & 1023);
    if exponent == 0 {
        sign * mantissa * 2_f32.powi(-24)
    } else {
        sign * (1. + mantissa / 1024.) * 2_f32.powi(exponent - 15)
    }
}
pub(super) async fn host(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    temporal: bool,
    reconstruction: bool,
) -> Result<FrameHost<World, Scene>, String> {
    FrameHost::new(
        caps,
        queue,
        FrameConfig {
            render_extent: [64, 64],
            surface: Some(thrum_render::present::SurfaceProfile {
                format: wgpu::TextureFormat::Bgra8Unorm,
                extent: [64, 64],
            }),
        },
        World(SourceTick {
            epoch: 7,
            timeline: 3,
            tick: 10,
        }),
        Scene {
            temporal,
            reconstruction,
        },
        SyntheticScene::grid(2).map_err(error)?,
        1.,
    )
    .await
    .map_err(error)
}

/// Prove graph requests, moving depth, submitted unjittered motion, cuts and effect resets.
/// # Errors
/// Returns the first target mismatch or device error.
pub async fn run(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    instance: &wgpu::Instance,
) -> Result<String, QualificationError> {
    run_checks(device, queue, caps, instance)
        .await
        .map_err(QualificationError::CheckFailed)
}

async fn run_checks(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    instance: &wgpu::Instance,
) -> Result<String, String> {
    let mut record = vec![format!(
        "adapter={:?}; generation={}; recipe={:?}",
        caps.adapter(),
        caps.generation(),
        caps.recipe()
    )];
    record.extend(graph_cases(caps, queue).await?);
    record.push(shared_copy(caps, queue).await?);
    record.push(Box::pin(super::after_compute::run(device, queue, caps)).await?);
    record.extend(Box::pin(slab_pixels(device, queue, caps, instance)).await?);
    Ok(record.join("\n"))
}

async fn graph_cases(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
) -> Result<Vec<String>, String> {
    let mut record = Vec::new();
    for (reconstruction, effect, refraction, count) in [
        (false, false, false, 0),
        (true, false, false, 0),
        (false, true, false, 0),
        (true, true, true, 1),
    ] {
        let mut host = host(caps, queue, false, reconstruction).await?;
        let expected = if reconstruction {
            Reconstruction::Taau
        } else {
            Reconstruction::Off
        };
        if host.reconstruction() != expected {
            return Err("graph case did not prepare the selected reconstruction provider".into());
        }
        if effect {
            let (effect, _) = DiagnosticEffect::new();
            host.set_scene_contribution(Box::new(effect.with_slab(refraction)))?;
        }
        let graph = host.scene_graph().map_err(error)?;
        let names = graph.pass_names().collect::<Vec<_>>();
        if names
            .iter()
            .filter(|&&name| name == "effect refraction copy")
            .count()
            != count
        {
            return Err("refraction request count".into());
        }
        if names.iter().any(|name| name.ends_with("diagnostic slab")) != effect {
            return Err("absent slab adds a pass".into());
        }
        if names.contains(&"portable TAAU") != reconstruction {
            return Err("reconstruction graph does not match its selection".into());
        }
        if reconstruction && effect {
            let slab = names
                .iter()
                .position(|name| name.ends_with("diagnostic slab"));
            let resolve = names.iter().position(|name| *name == "portable TAAU");
            if !matches!((slab, resolve), (Some(slab), Some(resolve)) if slab < resolve) {
                return Err("reconstruction did not follow the slab".into());
            }
        }
        let storage = host.storage_report();
        let depth = storage
            .lines()
            .find(|line| line.contains("role=depth"))
            .ok_or("missing depth storage row")?;
        let sampled = depth.contains("TEXTURE_BINDING");
        if sampled != (reconstruction || effect) {
            return Err(format!("depth request union: {depth}"));
        }
        record.push(format!(
            "reconstruction={expected:?} effect={effect} refraction={refraction}: passes={names:?}; {depth}"
        ));
    }
    Ok(record)
}

async fn slab_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    instance: &wgpu::Instance,
) -> Result<Vec<String>, String> {
    let mut record = Vec::new();
    let mut host = host(caps, queue, true, false).await?;
    let mut baseline = super::after_opaque::host(caps, queue, true, false).await?;
    let (effect, handle) = DiagnosticEffect::new();
    host.set_scene_contribution(Box::new(effect.with_slab(true)))?;
    for subject in [&mut host, &mut baseline] {
        subject
            .enable_submitted_capture_from(
                31,
                thrum_render::runtime::CaptureLimits::default(),
                thrum_render::synthetic::CaptureSource::SceneLinear,
            )
            .map_err(error)?;
    }
    let mut frames = Vec::new();
    let mut refracted = 0;
    for tick in 10..=16 {
        host.world_mut().0.tick = tick;
        baseline.world_mut().0.tick = tick;
        if tick == 16 {
            host.world_mut().0.timeline = 4;
            baseline.world_mut().0.timeline = 4;
        }
        if tick == 13 {
            handle.lock().map_err(error)?.controls.slab_offset = 3.;
            drop(
                host.advance(Duration::from_millis(12_500))
                    .map_err(error)?
                    .encode_scene()
                    .map_err(error)?,
            );
        }
        {
            let mut state = handle.lock().map_err(error)?;
            state.controls.slab_offset = if tick >= 13 { 0.25 } else { 0. };
            state.controls.slab_reset = tick == 15;
        }
        let report = host
            .advance(Duration::from_secs(tick))
            .map_err(error)?
            .encode_scene()
            .map_err(error)?
            .submit(None)
            .map_err(error)?;
        if !report.submitted {
            return Err("slab did not submit".into());
        }
        let texture = host
            .submitted_temporal_witness()
            .ok_or("missing submitted temporal witness")?;
        let bytes = read_texture_bytes(device, queue, texture, 0, 8).await?;
        baseline
            .advance(Duration::from_secs(tick))
            .map_err(error)?
            .encode_scene()
            .map_err(error)?
            .submit(None)
            .map_err(error)?;
        let baseline_witness = baseline
            .submitted_temporal_witness()
            .ok_or("baseline temporal witness")?
            .clone();
        let baseline_depth = read_texture_bytes(device, queue, &baseline_witness, 0, 8).await?;
        let progress = handle.lock().map_err(error)?.textures[2][0].clone();
        let pixels =
            super::qualification::submitted_colour(&mut host, device, queue, &progress, tick)
                .await?;
        let baseline_pixels =
            super::qualification::submitted_colour(&mut baseline, device, queue, &progress, tick)
                .await?;
        let values = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|pixel| {
                [
                    half(&pixel[0..2]),
                    half(&pixel[2..4]),
                    half(&pixel[4..6]),
                    half(&pixel[6..8]),
                ]
            })
            .collect::<Vec<_>>();
        let centre = values[32 * 64 + 32];
        let invalid = matches!(tick, 10 | 14 | 15 | 16);
        if invalid {
            if centre[2..] != [65504., 65504.] || (centre[0] - 1.).abs() > f32::EPSILON {
                return Err(format!("tick {tick} reset inputs: {centre:?}"));
            }
        } else {
            let expected = if tick == 13 {
                -0.25 * 32. * 3_f32.sqrt() / 10.
            } else {
                0.
            };
            if (centre[2] - expected).abs() > 0.003
                || centre[3].abs() > 0.003
                || (centre[0] - 0.35).abs() > 0.003
            {
                return Err(format!(
                    "tick {tick} displacement expected={expected}: {centre:?}"
                ));
            }
            refracted += refraction_pixels(
                device,
                queue,
                &handle,
                &values,
                &pixels,
                &baseline_pixels,
                &baseline_depth,
            )
            .await?;
        }
        record.push(format!("submitted tick={tick}: centre={centre:?}"));
        frames.push(values);
        drop(host.poll_completed().map_err(error)?);
    }
    // Same frame: the nearer object moved from left to right between ticks 11 and 12.
    // Pixels newly exposed by that move must immediately contain slab inputs, not old depth.
    let exposed = frames[1]
        .iter()
        .zip(&frames[2])
        .filter(|(a, b)| a[1] > 0.011 && b[1] == 0. && (b[0] - 0.35).abs() < 0.003)
        .count();
    let hidden = frames[1]
        .iter()
        .zip(&frames[2])
        .filter(|(a, b)| a[1] == 0. && b[1] > 0.011 && b[0] == 0.)
        .count();
    if exposed < 4 || hidden < 4 {
        return Err(format!(
            "moving occluder did not cut slab in the same frame: exposed={exposed},hidden={hidden}"
        ));
    }
    record.push(format!("current-depth cuts: exposed={exposed},hidden={hidden}; unjittered displacement, cut and effect reset pass"));
    if refracted == 0 {
        return Err("refraction proof sampled no geometry behind the slab".into());
    }
    record.push(format!("submitted refraction: shifted scene colour and single attenuation checked, {refracted} samples from geometry behind the slab; discarded displacement 3 never committed"));
    handle.lock().map_err(error)?.controls.slab_reset = false;
    replacement(&mut host, device, instance, caps).await?;
    record.push(
        "replacement device: invalid slab motion and reactive=1 on submitted generation 2".into(),
    );
    Ok(record)
}

async fn refraction_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    handle: &super::EffectHandle,
    inputs: &[[f32; 4]],
    pixels: &[u8],
    baseline: &[u8],
    depth: &[u8],
) -> Result<usize, String> {
    let textures = {
        let witness = handle.lock().map_err(error)?;
        [2, 3, 4, 5].map(|index| witness.textures[index][0].clone())
    };
    let mut fields = Vec::new();
    for (i, texture) in textures.iter().enumerate() {
        fields.push(
            super::qualification::read_texture(device, queue, texture, if i == 2 { 2 } else { 0 })
                .await?,
        );
    }
    let mut refracted = 0;
    for y in 0..64usize {
        for x in 0..64usize {
            if (inputs[y * 64 + x][0] - 0.35).abs() > 0.003 {
                continue;
            }
            let shifted = (y * 64 + (x + 6).min(63)) * 8;
            let slab_depth = 0.1 * (100. - 10.) / (10. * (100. - 0.1));
            let source = if half(&depth[shifted + 2..shifted + 4]) > slab_depth {
                (y * 64 + x) * 8
            } else {
                shifted
            };
            let value = fields[0][y % 32 * 32 + x % 32]
                + fields[1][y % 8 * 8 + x % 8]
                + fields[2][y % 2 * 2 + x % 2]
                + fields[3][y % 8 * 8 + x % 8];
            let weight = f32::from(u16::try_from(value % 17).map_err(error)?) / 17.;
            for (channel, tint) in [0.002, 0.008, 0.01].into_iter().enumerate() {
                let expected = half(&baseline[source + channel * 2..source + channel * 2 + 2])
                    * (0.65 + tint * weight);
                let actual = half(
                    &pixels[(y * 64 + x) * 8 + channel * 2..(y * 64 + x) * 8 + channel * 2 + 2],
                );
                if (actual - expected).abs() > expected.abs() * 0.001 + 0.000_000_06 {
                    return Err(format!(
                        "refraction at ({x},{y}) channel {channel}: expected {expected}, got {actual}"
                    ));
                }
            }
            let source_depth = half(&depth[source + 2..source + 4]);
            refracted += usize::from(source_depth > 0. && source_depth < 0.01);
        }
    }
    Ok(refracted)
}

async fn shared_copy(caps: &DeviceCapabilities, queue: &wgpu::Queue) -> Result<String, String> {
    let mut host = host(caps, queue, false, false).await?;
    let (a, _) = DiagnosticEffect::new();
    let (b, _) = DiagnosticEffect::new();
    host.set_scene_contribution(Box::new(
        thrum_render::contribution::SceneContributions::new(vec![
            Box::new(a.with_slab(true)),
            Box::new(b.with_slab(true)),
        ]),
    ))?;
    let graph = host.scene_graph().map_err(error)?;
    let names = graph.pass_names().collect::<Vec<_>>();
    if names
        .iter()
        .filter(|&&name| name == "effect refraction copy")
        .count()
        != 1
    {
        return Err("two effects failed to share refraction copy".into());
    }
    let late = names
        .iter()
        .filter(|name| name.starts_with("effect after "))
        .copied()
        .collect::<Vec<_>>();
    if late
        != [
            "effect after 0: effect 0: diagnostic slab",
            "effect after 1: effect 1: diagnostic slab",
        ]
    {
        return Err(format!("contribution order: {late:?}"));
    }
    let position = |name: &str| {
        names
            .iter()
            .position(|&value| value == name)
            .ok_or_else(|| format!("missing pass {name}"))
    };
    if !(position("synthetic forward opaque")? < position("effect refraction copy")?
        && position("effect refraction copy")? < position(late[0])?
        && position(late[1])? < position("synthetic tone encode")?)
    {
        return Err("late effect order".into());
    }
    Ok("two refracting effects: one shared copy, stable contribution order before pre-tone".into())
}

async fn replacement(
    host: &mut FrameHost<World, Scene>,
    old: &wgpu::Device,
    instance: &wgpu::Instance,
    caps: &DeviceCapabilities,
) -> Result<(), String> {
    let inputs = host.device_prepare_inputs().map_err(error)?;
    old.destroy();
    let (device, queue, next) = super::qualification::replacement_device(
        instance,
        caps.generation() + 1,
        caps.recipe().map_err(error)?,
    )
    .await?;
    let mut prepared = inputs
        .prepare(
            &next,
            &queue,
            Some(thrum_render::present::SurfaceProfile {
                format: wgpu::TextureFormat::Bgra8Unorm,
                extent: [64, 64],
            }),
        )
        .await
        .map_err(error)?;
    host.publish_device(&mut prepared).map_err(error)?;
    host.world_mut().0.tick = 17;
    host.advance(Duration::from_secs(17))
        .map_err(error)?
        .encode_scene()
        .map_err(error)?
        .submit(None)
        .map_err(error)?;
    let bytes = read_texture_bytes(
        &device,
        &queue,
        host.submitted_temporal_witness()
            .ok_or("replacement witness")?,
        0,
        8,
    )
    .await?;
    let centre = &bytes[(32 * 64 + 32) * 8..(32 * 64 + 33) * 8];
    if centre[4..] != [0xff, 0x7b, 0xff, 0x7b] || (half(&centre[..2]) - 1.).abs() > f32::EPSILON {
        return Err("replacement slab claimed old correspondence".into());
    }
    Ok(())
}
