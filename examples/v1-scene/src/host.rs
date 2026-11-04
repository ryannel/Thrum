//! Bounded offscreen frame host for the first assembled V1 workload.

use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[cfg(not(target_arch = "wasm32"))]
use thrum_cook::EnvironmentProduct;
use thrum_core::{
    ClockConfig, ClockState, TickClock,
    sim::{Simulation, VisualSnapshotPool},
};
#[cfg(not(target_arch = "wasm32"))]
use thrum_render::synthetic::PhotometricEnvironment;
use thrum_render::{
    extract::RenderExtractor,
    present::{PresentationBinding, SurfacePresenter, SurfaceProfile},
    synthetic::{
        OffscreenTarget, PreparedSynthetic, SyntheticLight, SyntheticPointLight, SyntheticScene,
        SyntheticView,
    },
    timing::GpuTimings,
};

use crate::scene::{self, ApplyReport, ENTITY_COUNT};

const GENERATION: u64 = 1;
const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;
const GPU_WAIT: Duration = Duration::from_secs(10);
const IN_FLIGHT: usize = 3;

/// One completed simulation-to-GPU frame. GPU pass values are absent when the
/// adapter lacks timestamp queries or a timing readback fails. Callback time
/// excludes deferred GPU completion and must not be added to GPU pass times.
#[derive(Clone, Copy, Debug)]
pub struct FrameSample {
    /// Last complete world tick used by the draw.
    pub tick: u64,
    /// Committed updates served in this host call, including catch-up work.
    pub completed_ticks: u32,
    /// Due updates still queued after this call.
    pub queued_ticks: u32,
    /// CPU time spent inside authoritative simulation callbacks.
    pub tick_cpu: Duration,
    /// CPU time for committed-world visit and render-column extraction.
    pub extract_cpu: Duration,
    /// CPU time for scene mapping, ABI packing and queue writes.
    pub upload_cpu: Duration,
    /// CPU time for render command encoding and timestamp registration.
    pub encode_cpu: Duration,
    /// CPU wall time for this drive call, including nonblocking GPU polling.
    /// Separate completed-report draining is outside this value.
    pub wall: Duration,
    /// Actual GPU pass durations for near/far Sun shadows, forward and display.
    /// Disabled shadow passes report zero in their slots.
    pub gpu_pass_ns: Option<[f64; 4]>,
    /// GPU time for the optional surface blit, when it was submitted and timed.
    pub gpu_present_ns: Option<f64>,
    /// This frame also wrote an acquired platform surface.
    pub surface_presented: bool,
    /// The GPU frame completed but its optional timing batch was unusable.
    pub timing_fault: bool,
    /// Records whose changed ABI bytes were enqueued for upload.
    pub uploaded_rows: usize,
    /// Queue payload bytes for this update.
    pub uploaded_bytes: usize,
    /// Accepted movers with valid adjacent-tick previous positions.
    pub moving: usize,
    /// Presentation samples/updates dropped with explicit telemetry.
    pub dropped_presentation: usize,
    /// Local-light references omitted from this prepared view after a cluster
    /// exceeds its capacity. The fixed fixture prepares them once.
    pub dropped_light_references: usize,
    /// Authored presentation light channels clamped during preparation.
    pub clamped_light_channels: usize,
    /// Cooked visual product used by this submitted frame, if selected.
    pub visual_revision: Option<[u8; 32]>,
}

/// One clock callback. A busy presentation ring drops only its optional view;
/// committed simulation updates are never dropped or applied twice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DriveReport {
    /// Last complete authoritative tick after this callback.
    pub tick: u64,
    /// World updates committed in this callback.
    pub completed_ticks: u32,
    /// Due updates left queued by the fixed clock.
    pub queued_ticks: u32,
    /// A GPU view was submitted for `tick`.
    pub submitted: bool,
    /// An acquired platform surface received this frame in the same submission.
    pub surface_presented: bool,
    /// No frame slot was free, so this optional view was skipped.
    pub dropped_view: bool,
    /// CPU wall time for the entire callback, including nonblocking GPU polling.
    pub wall: Duration,
}

/// An acquired texture view supplied by the platform's single surface owner.
/// The platform calls `present` only when `DriveReport::surface_presented` is true.
pub struct SurfaceOutput<'a> {
    /// View of the exact acquired surface texture.
    pub view: &'a wgpu::TextureView,
    /// Its configured physical pixel extent.
    pub extent: [u32; 2],
}

enum SlotState {
    Free,
    Rendering(FrameSample),
    Resolving(FrameSample),
}

struct FrameSlot {
    target: OffscreenTarget,
    presentation: Option<PresentationBinding>,
    timings: GpuTimings,
    render_done: Arc<AtomicBool>,
    state: SlotState,
}

/// A detached copy of the scene and executor handles for preparing one cooked
/// visual generation without borrowing the ticking host.
#[cfg(not(target_arch = "wasm32"))]
pub struct VisualPrepareInputs {
    device: wgpu::Device,
    queue: wgpu::Queue,
    scene: SyntheticScene,
    surface: bool,
    owner: Arc<()>,
}

/// Prepared product and GPU targets awaiting an extraction-barrier selection.
/// Keep this value across a busy frame ring and retry publication later.
#[cfg(not(target_arch = "wasm32"))]
pub struct PreparedEnvironment {
    product: Option<EnvironmentProduct>,
    prepared: Option<PreparedSynthetic>,
    slots: Option<Vec<FrameSlot>>,
    owner: Arc<()>,
}

#[cfg(not(target_arch = "wasm32"))]
impl VisualPrepareInputs {
    /// Build a complete presentation generation off the authoritative tick path.
    ///
    /// # Errors
    ///
    /// Returns a GPU preparation fault without affecting the running host.
    pub async fn prepare(self, product: EnvironmentProduct) -> Result<PreparedEnvironment, String> {
        let values = product.values();
        let light = PhotometricEnvironment {
            toward_sun: values.toward_sun(),
            sun_lux: values.sun_lux(),
            sun_colour: values.sun_colour(),
            ambient_lux: values.ambient_lux(),
            background_radiance: values.background_radiance(),
        };
        let prepared = PreparedSynthetic::prepare_with_photometric_environment(
            &self.device,
            &self.queue,
            GENERATION,
            &self.scene,
            light,
            1.0,
        )
        .await
        .map_err(|error| format!("cooked visual prepare: {error:?}"))?;
        let mut slots = Vec::with_capacity(IN_FLIGHT);
        for _ in 0..IN_FLIGHT {
            slots.push(FrameSlot {
                target: OffscreenTarget::new(&self.device, &prepared, WIDTH, HEIGHT)
                    .map_err(|error| format!("cooked target: {error:?}"))?,
                presentation: None,
                timings: GpuTimings::new(
                    &self.device,
                    &self.queue,
                    GENERATION,
                    prepared.scene_pass_count() + u32::from(self.surface),
                )
                .map_err(|error| format!("cooked timings: {error:?}"))?,
                render_done: Arc::new(AtomicBool::new(false)),
                state: SlotState::Free,
            });
        }
        Ok(PreparedEnvironment {
            product: Some(product),
            prepared: Some(prepared),
            slots: Some(slots),
            owner: self.owner,
        })
    }
}

/// One fixed-clock simulation and prepared GPU scene. The device and queue
/// belong to the caller's executor; this host creates no independent GPU owner.
/// Three prepared targets bound in-flight work and defer GPU timing readback.
/// A platform adapter may also supply an acquired surface view for the
/// prepared presentation pass; the host never owns the surface lifecycle.
pub struct FrameHost {
    device: wgpu::Device,
    queue: wgpu::Queue,
    clock: TickClock,
    sim: Simulation,
    snapshots: VisualSnapshotPool,
    extractor: RenderExtractor,
    scene: SyntheticScene,
    changed_slots: Vec<u32>,
    prepared: PreparedSynthetic,
    presenter: Option<SurfacePresenter>,
    slots: Vec<FrameSlot>,
    completed: VecDeque<FrameSample>,
    dropped_views: u64,
    dropped_reports: u64,
    dropped_capture_rows: u64,
    dropped_surface: u64,
    #[cfg(not(target_arch = "wasm32"))]
    visual_product: Option<EnvironmentProduct>,
    #[cfg(not(target_arch = "wasm32"))]
    visual_owner: Arc<()>,
}

impl FrameHost {
    /// Prepare one 10k-entity scene, all four scene passes and an optional
    /// surface presentation pass before the first draw.
    /// # Errors
    /// Returns setup, device or scene preparation errors without a partial host.
    /// # Panics
    /// Panics only if the compiled prepared-frame ring size is zero.
    pub async fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface: Option<SurfaceProfile>,
    ) -> Result<Self, String> {
        let sim = scene::simulation()?;
        let mut snapshots = VisualSnapshotPool::new(
            NonZeroUsize::new(IN_FLIGHT).expect("nonzero prepared frame ring"),
            ENTITY_COUNT,
        );
        let mut extractor = RenderExtractor::new(ENTITY_COUNT);
        let initial = snapshots
            .publish(&sim)
            .ok_or("initial visual page unavailable")?;
        scene::extract(&mut extractor, &initial);
        drop(initial);
        let count = u32::try_from(ENTITY_COUNT).map_err(|_| "fixture exceeds u32 slots")?;
        let mut scene =
            SyntheticScene::grid(count).map_err(|error| format!("synthetic scene: {error:?}"))?;
        scene.view = SyntheticView::perspective(
            [0.0, 0.0, 2.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            1.0,
            WIDTH as f32 / HEIGHT as f32,
            0.1,
            20.0,
        )
        .map_err(|error| format!("camera: {error:?}"))?;
        scene.sun_shadows = true;
        scene.local_lights = [
            ([-0.55, -0.55, 1.0], [5.0, 0.8, 0.4]),
            ([0.55, -0.55, 1.0], [0.4, 4.0, 0.8]),
            ([-0.55, 0.55, 1.0], [0.7, 1.0, 5.0]),
            ([0.55, 0.55, 1.0], [4.0, 3.0, 0.4]),
        ]
        .map(|(position, candela)| SyntheticPointLight {
            position,
            radius: 0.6,
            candela,
        })
        .to_vec();
        let mut changed_slots = Vec::with_capacity(ENTITY_COUNT);
        scene::apply_frame_to_scene(&mut scene, extractor.frame(), &mut changed_slots);
        let prepared = PreparedSynthetic::prepare_with_light(
            device,
            queue,
            GENERATION,
            &scene,
            SyntheticLight {
                direction_to_light: [0.4, -0.6, 1.0],
                irradiance: [4.0; 3],
            },
        )
        .await
        .map_err(|error| format!("prepare: {error:?}"))?;
        let presenter = match surface {
            Some(profile) => Some(
                SurfacePresenter::prepare(device, GENERATION, [WIDTH, HEIGHT], profile)
                    .await
                    .map_err(|error| format!("surface prepare: {error:?}"))?,
            ),
            None => None,
        };
        let mut slots = Vec::with_capacity(IN_FLIGHT);
        for _ in 0..IN_FLIGHT {
            let target = OffscreenTarget::new(device, &prepared, WIDTH, HEIGHT)
                .map_err(|error| format!("target: {error:?}"))?;
            slots.push(FrameSlot {
                presentation: presenter
                    .as_ref()
                    .map(|presenter| presenter.bind_target(device, &target))
                    .transpose()
                    .map_err(|error| format!("surface binding: {error:?}"))?,
                target,
                timings: GpuTimings::new(
                    device,
                    queue,
                    GENERATION,
                    if surface.is_some() { 5 } else { 4 },
                )
                .map_err(|error| format!("GPU timings: {error:?}"))?,
                render_done: Arc::new(AtomicBool::new(false)),
                state: SlotState::Free,
            });
        }
        let clock = TickClock::new(ClockConfig::default(), Duration::ZERO)
            .map_err(|error| format!("clock: {error:?}"))?;
        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            clock,
            sim,
            snapshots,
            extractor,
            scene,
            changed_slots,
            prepared,
            presenter,
            slots,
            completed: VecDeque::with_capacity(IN_FLIGHT),
            dropped_views: 0,
            dropped_reports: 0,
            dropped_capture_rows: 0,
            dropped_surface: 0,
            #[cfg(not(target_arch = "wasm32"))]
            visual_product: None,
            #[cfg(not(target_arch = "wasm32"))]
            visual_owner: Arc::new(()),
        })
    }

    /// Copy presentation inputs once for an asynchronous visual preparation.
    /// This copies the finite scene only on accepted edits, never on ticks.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn visual_prepare_inputs(&self) -> VisualPrepareInputs {
        VisualPrepareInputs {
            device: self.device.clone(),
            queue: self.queue.clone(),
            scene: self.scene.clone(),
            surface: self.presenter.is_some(),
            owner: Arc::clone(&self.visual_owner),
        }
    }

    /// Select a fully prepared cooked view at a GPU-safe extraction barrier.
    /// `false` retains the candidate for retry while older packets are live.
    ///
    /// # Errors
    ///
    /// Returns a binding or full-table upload fault without replacing the active view.
    ///
    /// # Panics
    ///
    /// Panics only if a checked candidate loses an owned resource during the
    /// final single-threaded move into the active host.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn publish_environment(
        &mut self,
        candidate: &mut PreparedEnvironment,
    ) -> Result<bool, String> {
        if !Arc::ptr_eq(&candidate.owner, &self.visual_owner) {
            return Err("visual candidate belongs to another host".to_owned());
        }
        self.poll_gpu()?;
        if self.pending_frames() != 0 {
            return Ok(false);
        }
        let slots = candidate
            .slots
            .as_mut()
            .ok_or_else(|| "visual candidate already published".to_owned())?;
        for slot in slots.iter_mut() {
            slot.presentation = self
                .presenter
                .as_ref()
                .map(|presenter| presenter.bind_target(&self.device, &slot.target))
                .transpose()
                .map_err(|error| format!("cooked surface binding: {error:?}"))?;
        }
        self.changed_slots.clear();
        for index in 0..self.scene.instances.len() {
            self.changed_slots
                .push(u32::try_from(index).map_err(|_| "visual instance slot exceeds u32")?);
        }
        candidate
            .prepared
            .as_mut()
            .ok_or_else(|| "visual candidate already published".to_owned())?
            .upload_instances(&self.queue, &self.scene.instances, &self.changed_slots)
            .map_err(|error| format!("cooked full upload: {error:?}"))?;
        self.prepared = candidate.prepared.take().expect("checked candidate");
        self.slots = candidate.slots.take().expect("checked candidate");
        self.visual_product = candidate.product.take();
        self.extractor.invalidate_history();
        Ok(true)
    }

    /// Delay until the next world update is due; the platform owns actual sleep.
    pub fn next_due_in(&self) -> Option<Duration> {
        self.clock.next_due_in()
    }

    /// Last committed world tick.
    pub fn tick(&self) -> u64 {
        self.sim.tick()
    }

    /// Number of optional views skipped because all GPU packet slots were held.
    pub fn dropped_views(&self) -> u64 {
        self.dropped_views
    }

    /// Completed profiler samples discarded when the caller did not drain the
    /// bounded result queue. This never affects authoritative ticks or GPU work.
    pub fn dropped_reports(&self) -> u64 {
        self.dropped_reports
    }

    /// Presentation rows omitted while refreshing an explicit capture.
    pub fn dropped_capture_rows(&self) -> u64 {
        self.dropped_capture_rows
    }

    /// Optional surface views omitted because acquisition or encode was unavailable.
    pub fn dropped_surface(&self) -> u64 {
        self.dropped_surface
    }

    /// Recompile the surface extent declaration after the platform configures
    /// a nonzero replacement; in-flight old textures remain GPU-owned.
    /// # Errors
    /// Refuses an invalid extent or a host without a surface presenter.
    pub fn resize_surface(&mut self, extent: [u32; 2]) -> Result<(), String> {
        self.presenter
            .as_mut()
            .ok_or_else(|| "host has no surface presenter".to_owned())?
            .resize(&self.device, extent)
            .map_err(|error| format!("surface resize: {error:?}"))
    }

    /// Number of submitted render packets awaiting GPU completion or readback.
    pub fn pending_frames(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| !matches!(slot.state, SlotState::Free))
            .count()
    }

    /// Progress native callbacks without blocking and return the oldest completed
    /// frame, if any. Browser WebGPU delivers callbacks through its event loop.
    /// # Errors
    /// Returns a device polling fault; optional timestamp failures are reported
    /// on the completed frame and do not stop the simulation.
    pub fn poll_completed(&mut self) -> Result<Option<FrameSample>, String> {
        self.poll_gpu()?;
        let oldest_pending = self
            .slots
            .iter()
            .filter_map(|slot| match slot.state {
                SlotState::Free => None,
                SlotState::Rendering(sample) | SlotState::Resolving(sample) => Some(sample.tick),
            })
            .min();
        if self
            .completed
            .front()
            .is_some_and(|sample| oldest_pending.is_none_or(|pending| sample.tick < pending))
        {
            Ok(self.completed.pop_front())
        } else {
            Ok(None)
        }
    }

    /// Admit monotonic host time through the shared fixed-step clock. Every due
    /// update commits once before the latest committed view is extracted. A full
    /// presentation ring skips this view with a counter, never the world update.
    /// If no update is due, no GPU frame is encoded. Overload is explicit.
    /// # Errors
    /// Returns a clock, simulation or GPU fault; never silently skips an update.
    pub fn drive(
        &mut self,
        now: Duration,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, String> {
        if surface.is_some() && self.presenter.is_none() {
            return Err("host has no surface presenter".to_owned());
        }
        let wall_start = Instant::now();
        self.poll_gpu()?;
        let mut tick_cpu = Duration::ZERO;
        let clock = self
            .clock
            .advance(now, 4, || {
                let start = Instant::now();
                let result = self.sim.step(&[]).map(|_| ());
                tick_cpu += start.elapsed();
                result
            })
            .map_err(|error| format!("clock advance: {error:?}"))?;
        if let Some(error) = clock.step_error {
            return Err(format!("simulation tick failed: {error:?}"));
        }
        if clock.state == ClockState::Overloaded {
            return Err(format!(
                "fixed-clock overload: {} ticks refused, {} still queued",
                clock.refused_on_overload, clock.queued
            ));
        }
        let mut report = DriveReport {
            tick: self.sim.tick(),
            completed_ticks: clock.completed,
            queued_ticks: clock.queued,
            submitted: false,
            surface_presented: false,
            dropped_view: false,
            wall: Duration::ZERO,
        };
        if clock.completed == 0 {
            report.wall = wall_start.elapsed();
            return Ok(report);
        }
        if self.presenter.is_some()
            && surface.is_none_or(|surface| {
                self.presenter
                    .as_ref()
                    .is_none_or(|presenter| presenter.extent() != surface.extent)
            })
        {
            self.dropped_views += 1;
            self.dropped_surface += 1;
            report.dropped_view = true;
            report.wall = wall_start.elapsed();
            return Ok(report);
        }
        let Some(slot_index) = self
            .slots
            .iter()
            .position(|slot| matches!(slot.state, SlotState::Free))
        else {
            self.dropped_views += 1;
            report.dropped_view = true;
            report.wall = wall_start.elapsed();
            return Ok(report);
        };

        let extract_start = Instant::now();
        let Some(snapshot) = self.snapshots.publish(&self.sim) else {
            self.dropped_views += 1;
            report.dropped_view = true;
            report.wall = wall_start.elapsed();
            return Ok(report);
        };
        let extracted = scene::extract(&mut self.extractor, &snapshot);
        drop(snapshot);
        let extract_cpu = extract_start.elapsed();
        let upload_start = Instant::now();
        let ApplyReport {
            moving,
            dropped_invalid,
        } = scene::apply_frame_to_scene(
            &mut self.scene,
            self.extractor.frame(),
            &mut self.changed_slots,
        );
        let uploaded = self
            .prepared
            .upload_instances(&self.queue, &self.scene.instances, &self.changed_slots)
            .map_err(|error| format!("instance update: {error:?}"))?;
        let upload_cpu = upload_start.elapsed();

        let encode_start = Instant::now();
        let visual_revision = self.current_visual_revision();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let slot = &mut self.slots[slot_index];
        self.prepared
            .encode(
                &mut encoder,
                GENERATION,
                &slot.target,
                Some(&mut slot.timings),
            )
            .map_err(|error| format!("render encode: {error:?}"))?;
        let surface_presented = if let (Some(presenter), Some(binding), Some(surface)) =
            (self.presenter.as_ref(), slot.presentation.as_ref(), surface)
        {
            if presenter
                .encode(
                    &mut encoder,
                    binding,
                    surface.view,
                    surface.extent,
                    Some(&mut slot.timings),
                )
                .is_ok()
            {
                true
            } else {
                self.dropped_surface += 1;
                false
            }
        } else {
            false
        };
        slot.timings
            .finish_render(GENERATION, &encoder)
            .map_err(|error| format!("finish timestamps: {error:?}"))?;
        let encode_cpu = encode_start.elapsed();
        slot.render_done.store(false, Ordering::Release);
        self.queue.submit([encoder.finish()]);
        let done = Arc::clone(&slot.render_done);
        self.queue
            .on_submitted_work_done(move || done.store(true, Ordering::Release));
        slot.state = SlotState::Rendering(FrameSample {
            tick: self.sim.tick(),
            completed_ticks: clock.completed,
            queued_ticks: clock.queued,
            tick_cpu,
            extract_cpu,
            upload_cpu,
            encode_cpu,
            wall: wall_start.elapsed(),
            gpu_pass_ns: None,
            gpu_present_ns: None,
            surface_presented,
            timing_fault: false,
            uploaded_rows: uploaded.changed_rows,
            uploaded_bytes: uploaded.bytes,
            moving,
            dropped_presentation: extracted.dropped_capacity
                + extracted.dropped_duplicate
                + extracted.dropped_invalid
                + dropped_invalid
                + uploaded.dropped_invalid
                + uploaded.dropped_shadow_envelope,
            dropped_light_references: self.prepared.dropped_light_references(),
            clamped_light_channels: self.prepared.presentation_light_clamps(),
            visual_revision,
        });
        report.submitted = true;
        report.surface_presented = surface_presented;
        report.wall = wall_start.elapsed();
        Ok(report)
    }

    fn current_visual_revision(&self) -> Option<[u8; 32]> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.visual_product
                .as_ref()
                .map(|product| *product.revision().as_bytes())
        }
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    }

    /// Render and read back the current committed scene once for visual evidence.
    /// This capture is outside the timed frame path.
    /// # Errors
    /// Returns a GPU encode, completion or readback error.
    pub fn capture_pixels(&mut self) -> Result<Vec<u8>, String> {
        if self
            .extractor
            .frame()
            .source()
            .is_none_or(|source| source.tick != self.sim.tick())
        {
            let snapshot = self
                .snapshots
                .publish(&self.sim)
                .ok_or("visual snapshot pages held during capture")?;
            let extracted = scene::extract(&mut self.extractor, &snapshot);
            drop(snapshot);
            let applied = scene::apply_frame_to_scene(
                &mut self.scene,
                self.extractor.frame(),
                &mut self.changed_slots,
            );
            let uploaded = self
                .prepared
                .upload_instances(&self.queue, &self.scene.instances, &self.changed_slots)
                .map_err(|error| format!("capture instance update: {error:?}"))?;
            let dropped = u64::try_from(
                extracted.dropped_capacity
                    + extracted.dropped_duplicate
                    + extracted.dropped_invalid
                    + applied.dropped_invalid
                    + uploaded.dropped_invalid
                    + uploaded.dropped_shadow_envelope,
            )
            .unwrap_or(u64::MAX);
            self.dropped_capture_rows = self.dropped_capture_rows.saturating_add(dropped);
        }
        let target = OffscreenTarget::new(&self.device, &self.prepared, WIDTH, HEIGHT)
            .map_err(|error| format!("capture target: {error:?}"))?;
        let mut pixels = target.readback(&self.device);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.prepared
            .encode(&mut encoder, GENERATION, &target, None)
            .map_err(|error| format!("capture render: {error:?}"))?;
        pixels
            .encode(&mut encoder, &target)
            .map_err(|error| format!("capture readback: {error:?}"))?;
        let submitted = self.queue.submit([encoder.finish()]);
        self.wait(submitted, "capture")?;
        pixels
            .try_collect(GENERATION)
            .map_err(|error| format!("capture collect: {error:?}"))?
            .ok_or_else(|| "capture readback incomplete".to_owned())
    }

    fn wait(&self, submitted: wgpu::SubmissionIndex, operation: &str) -> Result<(), String> {
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(GPU_WAIT),
            })
            .map_err(|error| format!("{operation} poll: {error:?}"))?;
        Ok(())
    }

    fn poll_gpu(&mut self) -> Result<(), String> {
        self.device
            .poll(wgpu::PollType::Poll)
            .map_err(|error| format!("GPU poll: {error:?}"))?;
        let max_passes = self.prepared.scene_pass_count() + u32::from(self.presenter.is_some());
        for slot in &mut self.slots {
            let state = std::mem::replace(&mut slot.state, SlotState::Free);
            match state {
                SlotState::Free => {}
                SlotState::Rendering(mut sample) => {
                    if !slot.render_done.load(Ordering::Acquire) {
                        slot.state = SlotState::Rendering(sample);
                        continue;
                    }
                    if !slot.timings.is_enabled() {
                        retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                        continue;
                    }
                    let mut encoder = self
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                    match slot.timings.resolve_if_ready(GENERATION, &mut encoder) {
                        Ok(true) => {
                            self.queue.submit([encoder.finish()]);
                            slot.state = SlotState::Resolving(sample);
                        }
                        Ok(false) => slot.state = SlotState::Rendering(sample),
                        Err(_) => {
                            sample.timing_fault = true;
                            slot.timings = new_timings(&self.device, &self.queue, max_passes)?;
                            retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                        }
                    }
                }
                SlotState::Resolving(mut sample) => match slot.timings.try_collect(GENERATION) {
                    Ok(Some(passes)) => {
                        if let Some((scene, present)) = pass_durations(
                            &passes,
                            sample.surface_presented,
                            self.prepared.scene_pass_count() == 4,
                        ) {
                            sample.gpu_pass_ns = Some(scene);
                            sample.gpu_present_ns = present;
                        } else {
                            sample.timing_fault = true;
                        }
                        retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                    }
                    Ok(None) => slot.state = SlotState::Resolving(sample),
                    Err(_) => {
                        sample.timing_fault = true;
                        slot.timings = new_timings(&self.device, &self.queue, max_passes)?;
                        retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                    }
                },
            }
        }
        Ok(())
    }
}

fn new_timings(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    max_passes: u32,
) -> Result<GpuTimings, String> {
    GpuTimings::new(device, queue, GENERATION, max_passes)
        .map_err(|error| format!("GPU timing recovery: {error:?}"))
}

fn pass_durations(
    passes: &[thrum_render::timing::PassTiming],
    presented: bool,
    shadows: bool,
) -> Option<([f64; 4], Option<f64>)> {
    let shadow_names = [
        "synthetic Sun shadow near",
        "synthetic Sun shadow far",
        "synthetic forward opaque",
        "synthetic tone encode",
    ];
    let names: &[&str] = if shadows {
        &shadow_names
    } else {
        &shadow_names[2..]
    };
    (passes.len() == names.len() + usize::from(presented)
        && passes
            .iter()
            .zip(names)
            .all(|(pass, name)| pass.name.as_ref() == *name)
        && (!presented || passes[names.len()].name.as_ref() == thrum_render::present::PASS_NAME))
        .then(|| {
            let mut scene = [0.0; 4];
            for (index, pass) in passes.iter().take(names.len()).enumerate() {
                scene[index + if shadows { 0 } else { 2 }] = pass.nanoseconds;
            }
            (scene, presented.then(|| passes[names.len()].nanoseconds))
        })
}

fn retain_sample(queue: &mut VecDeque<FrameSample>, dropped: &mut u64, sample: FrameSample) {
    if queue.len() == IN_FLIGHT {
        if queue
            .front()
            .is_some_and(|oldest| sample.tick <= oldest.tick)
        {
            *dropped += 1;
            return;
        }
        queue.pop_front();
        *dropped += 1;
    }
    let insert = queue
        .iter()
        .position(|prior| prior.tick > sample.tick)
        .unwrap_or(queue.len());
    queue.insert(insert, sample);
}

#[cfg(test)]
mod tests;
