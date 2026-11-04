//! Bounded offscreen frame host for the first assembled V1 workload.

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use thrum_cook::EnvironmentProduct;
use thrum_core::{
    ClockConfig, ClockState, TickClock,
    sim::{Simulation, VisualSnapshotPool},
};
#[cfg(not(target_arch = "wasm32"))]
use thrum_render::extract::SourceTick;
#[cfg(not(target_arch = "wasm32"))]
use thrum_render::synthetic::PhotometricEnvironment;
use thrum_render::{
    extract::{ExtractReport, RenderExtractor},
    present::{PresentationBinding, SurfacePresenter, SurfaceProfile},
    synthetic::{
        OffscreenTarget, PreparedSynthetic, SyntheticLight, SyntheticPointLight, SyntheticScene,
        SyntheticView,
    },
    timing::GpuTimings,
};

use crate::scene::{self, ApplyReport, ENTITY_COUNT};

#[cfg(not(target_arch = "wasm32"))]
mod native_worker;
#[cfg(not(target_arch = "wasm32"))]
use native_worker::{CompletedExtract, ExtractRequest, NativeWorker};

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
    /// GPU submission order; independent of the visual source tick.
    pub submission_order: u64,
    /// Committed updates served in this host call, including catch-up work.
    pub completed_ticks: u32,
    /// Due updates still queued after this call.
    pub queued_ticks: u32,
    /// CPU time spent inside authoritative simulation callbacks.
    pub tick_cpu: Duration,
    /// CPU time for committed-world visit and render-column extraction.
    pub extract_cpu: Duration,
    /// Native worker queue delay; zero for serial extraction.
    pub worker_queue_ns: u32,
    /// Time a completed native CPU view waited before submission.
    pub worker_ready_ns: u32,
    /// Committed ticks newer than this view at submission.
    pub visual_age_ticks: u32,
    /// A completed scene target was presented again while native extraction was busy.
    pub reused_visual: bool,
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
    /// Refused elapsed ticks when the bounded clock overloaded. `Some(0)`
    /// means it remains overloaded and requires explicit pause/drain/resume.
    pub overload: Option<u64>,
    /// World updates committed in this callback.
    pub completed_ticks: u32,
    /// Due updates left queued by the fixed clock.
    pub queued_ticks: u32,
    /// A GPU view was submitted; `tick` remains the latest authoritative tick.
    /// The submitted source tick is reported by the eventual `FrameSample`.
    pub submitted: bool,
    /// An acquired platform surface received this frame in the same submission.
    pub surface_presented: bool,
    /// A fresh optional view was skipped; a stale scene may still be submitted.
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

#[derive(Clone, Copy)]
struct SubmitFrame {
    extracted: ExtractReport,
    extract_cpu: Duration,
    worker_queue_cpu: Duration,
    worker_ready_cpu: Duration,
    tick_cpu: Duration,
    completed_ticks: u32,
    queued_ticks: u32,
    frame_tick: u64,
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
    desired_axes: [f32; 3],
    admitted_axes: [f32; 3],
    input_sequence: u64,
    snapshots: VisualSnapshotPool,
    extractor: Option<RenderExtractor>,
    #[cfg(not(target_arch = "wasm32"))]
    worker: Option<NativeWorker>,
    #[cfg(not(target_arch = "wasm32"))]
    worker_ready: Option<CompletedExtract>,
    #[cfg(not(target_arch = "wasm32"))]
    visual_generation: u64,
    #[cfg(not(target_arch = "wasm32"))]
    last_view_source: Option<SourceTick>,
    #[cfg(not(target_arch = "wasm32"))]
    last_render_slot: Option<usize>,
    scene: SyntheticScene,
    changed_slots: Vec<u32>,
    prepared: PreparedSynthetic,
    presenter: Option<SurfacePresenter>,
    slots: Vec<FrameSlot>,
    completed: VecDeque<FrameSample>,
    next_submission_order: u64,
    dropped_views: u64,
    dropped_stale_views: u64,
    #[cfg(not(target_arch = "wasm32"))]
    reused_views: u64,
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
        #[cfg(not(target_arch = "wasm32"))]
        let initial_source = SourceTick {
            epoch: sim.epoch().0,
            tick: sim.tick(),
        };
        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            clock,
            sim,
            desired_axes: [0.0; 3],
            admitted_axes: [0.0; 3],
            input_sequence: 1,
            snapshots,
            extractor: Some(extractor),
            #[cfg(not(target_arch = "wasm32"))]
            worker: None,
            #[cfg(not(target_arch = "wasm32"))]
            worker_ready: None,
            #[cfg(not(target_arch = "wasm32"))]
            visual_generation: 0,
            #[cfg(not(target_arch = "wasm32"))]
            last_view_source: Some(initial_source),
            #[cfg(not(target_arch = "wasm32"))]
            last_render_slot: None,
            scene,
            changed_slots,
            prepared,
            presenter,
            slots,
            completed: VecDeque::with_capacity(IN_FLIGHT),
            next_submission_order: 0,
            dropped_views: 0,
            dropped_stale_views: 0,
            #[cfg(not(target_arch = "wasm32"))]
            reused_views: 0,
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

    /// Move native CPU extraction to one long-lived worker. World ticks remain
    /// on the caller and never wait for this optional presentation task.
    /// `wake` must notify the platform loop after a result becomes available.
    ///
    /// # Errors
    ///
    /// Returns a native worker startup fault, leaving serial extraction active.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_native_worker(
        &mut self,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<(), String> {
        if self.worker.is_some() {
            return Err("frame worker already enabled".to_owned());
        }
        self.worker = Some(NativeWorker::new(wake)?);
        Ok(())
    }

    /// A completed CPU view is waiting for a surface and free GPU slot.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_ready_view(&self) -> bool {
        self.worker_ready.is_some()
    }

    /// A previously prepared scene can be shown while the native worker is late.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_reusable_view(&self) -> bool {
        self.worker.as_ref().is_some_and(NativeWorker::is_busy)
            && self
                .last_view_source
                .is_some_and(|source| source.epoch == self.sim.epoch().0)
            && self.last_render_slot.is_some_and(|index| {
                self.slots
                    .get(index)
                    .is_some_and(|slot| matches!(slot.state, SlotState::Free))
            })
    }

    /// Surface frames drawn from an older prepared scene while extraction was busy.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reused_views(&self) -> u64 {
        self.reused_views
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
        self.visual_generation = self.visual_generation.wrapping_add(1);
        self.last_view_source = None;
        self.last_render_slot = None;
        if let Some(extractor) = self.extractor.as_mut() {
            extractor.invalidate_history();
        }
        Ok(true)
    }

    /// Delay until the next world update is due; the platform owns actual sleep.
    pub fn next_due_in(&self) -> Option<Duration> {
        self.clock.next_due_in()
    }

    /// Pause wall-time admission while retaining already queued world updates.
    pub fn pause(&mut self) {
        self.clock.pause();
    }

    /// Resume from a new monotonic time origin after queued work has drained.
    ///
    /// # Errors
    /// Returns a clock error for an active clock, pending updates or backwards time.
    pub fn resume(&mut self, now: Duration) -> Result<(), String> {
        self.clock
            .resume(now)
            .map_err(|error| format!("clock resume: {error:?}"))
    }

    /// Finish previously admitted work before starting a fresh platform time
    /// origin. Hidden time and explicitly refused overload time stay outside
    /// the authoritative tick history.
    /// # Errors
    /// Returns a host fault if admitted ticks cannot drain or resume is invalid.
    pub fn drain_and_resume(&mut self, now: Duration) -> Result<(), String> {
        loop {
            let report = self.drive(now, None)?;
            if report.queued_ticks == 0 {
                break;
            }
            if report.completed_ticks == 0 {
                return Err("paused world updates could not drain".to_owned());
            }
        }
        self.resume(now)
    }

    /// Last committed world tick.
    pub fn tick(&self) -> u64 {
        self.sim.tick()
    }

    /// Sample continuous local controls for the next authoritative tick.
    /// Multiple UI changes before that tick use the latest normalized value.
    /// Local and remote inputs use the same `Simulation::admit_frame` gate.
    ///
    /// # Errors
    /// Returns a normalized-axis validation error without changing the held input.
    pub fn set_controls(&mut self, axes: [f32; 3]) -> Result<(), String> {
        scene::control_frame(axes, 0, 0)
            .validate_axes()
            .map_err(|error| format!("local controls: {error:?}"))?;
        self.desired_axes = axes;
        Ok(())
    }

    /// Committed world position of the fixture's controlled actor.
    pub fn controlled_position(&self) -> Option<[f64; 3]> {
        self.sim
            .handle(scene::CONTROLLED_ID)
            .and_then(|handle| self.sim.get(handle).ok())
            .map(|entity| entity.position.0)
    }

    /// Number of optional views skipped because all GPU packet slots were held.
    pub fn dropped_views(&self) -> u64 {
        self.dropped_views
    }

    /// Completed native views rejected after their visual age or generation expired.
    pub fn dropped_stale_views(&self) -> u64 {
        self.dropped_stale_views
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
                SlotState::Rendering(sample) | SlotState::Resolving(sample) => {
                    Some(sample.submission_order)
                }
            })
            .min();
        Ok(pop_report_before_pending(
            &mut self.completed,
            oldest_pending,
        ))
    }

    /// Admit monotonic host time through the shared fixed-step clock. Every due
    /// update commits once before a visual page can be published. A full
    /// presentation ring skips an optional view with a counter, never the world
    /// update. Native worker completion can submit on a zero-update callback;
    /// serial extraction submits only after a due update. Overload is explicit.
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
        let desired_axes = self.desired_axes;
        let admitted_axes = &mut self.admitted_axes;
        let input_sequence = &mut self.input_sequence;
        let sim = &mut self.sim;
        let clock = self
            .clock
            .advance(now, 4, || {
                let start = Instant::now();
                let result = (|| {
                    if desired_axes
                        .iter()
                        .zip(admitted_axes.iter())
                        .any(|(desired, admitted)| desired.to_bits() != admitted.to_bits())
                    {
                        let tick = sim.tick().checked_add(1).ok_or("input tick exhausted")?;
                        let frame = scene::control_frame(desired_axes, tick, *input_sequence);
                        sim.admit_frame(frame)
                            .map_err(|error| format!("local input admission: {error:?}"))?;
                        *admitted_axes = desired_axes;
                        *input_sequence = input_sequence
                            .checked_add(1)
                            .ok_or("input sequence exhausted")?;
                    }
                    sim.step(&[])
                        .map(|_| ())
                        .map_err(|error| format!("simulation step: {error:?}"))
                })();
                tick_cpu += start.elapsed();
                result
            })
            .map_err(|error| format!("clock advance: {error:?}"))?;
        if let Some(error) = clock.step_error {
            return Err(format!("simulation tick failed: {error:?}"));
        }
        let mut report = DriveReport {
            tick: self.sim.tick(),
            overload: (clock.state == ClockState::Overloaded).then_some(clock.refused_on_overload),
            completed_ticks: clock.completed,
            queued_ticks: clock.queued,
            submitted: false,
            surface_presented: false,
            dropped_view: false,
            wall: Duration::ZERO,
        };
        if report.overload.is_some() {
            if report.completed_ticks > 0 {
                self.dropped_views = self.dropped_views.saturating_add(1);
                report.dropped_view = true;
            }
            report.wall = wall_start.elapsed();
            return Ok(report);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.worker.is_some() {
            self.drive_native_view(&mut report, tick_cpu, wall_start, surface)?;
            report.wall = wall_start.elapsed();
            return Ok(report);
        }
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
        let extracted = scene::extract(
            self.extractor
                .as_mut()
                .ok_or("serial extractor unavailable")?,
            &snapshot,
        );
        drop(snapshot);
        let extract_cpu = extract_start.elapsed();
        self.submit_extracted(
            slot_index,
            &mut report,
            surface,
            SubmitFrame {
                extracted,
                extract_cpu,
                worker_queue_cpu: Duration::ZERO,
                worker_ready_cpu: Duration::ZERO,
                tick_cpu,
                completed_ticks: clock.completed,
                queued_ticks: clock.queued,
                frame_tick: self.sim.tick(),
            },
            wall_start,
        )?;
        Ok(report)
    }

    fn submit_extracted(
        &mut self,
        slot_index: usize,
        report: &mut DriveReport,
        surface: Option<&SurfaceOutput<'_>>,
        frame: SubmitFrame,
        wall_start: Instant,
    ) -> Result<(), String> {
        let upload_start = Instant::now();
        let ApplyReport {
            moving,
            dropped_invalid,
        } = scene::apply_frame_to_scene(
            &mut self.scene,
            self.extractor
                .as_ref()
                .expect("completed extractor belongs to host")
                .frame(),
            &mut self.changed_slots,
        );
        let uploaded = self
            .prepared
            .upload_instances(&self.queue, &self.scene.instances, &self.changed_slots)
            .map_err(|error| format!("instance update: {error:?}"))?;
        let upload_cpu = upload_start.elapsed();
        #[cfg(not(target_arch = "wasm32"))]
        let visual_revision = self.current_visual_revision();
        #[cfg(target_arch = "wasm32")]
        let visual_revision = None;
        self.encode_prepared(
            slot_index,
            report,
            surface,
            FrameSample {
                tick: frame.frame_tick,
                submission_order: 0,
                completed_ticks: frame.completed_ticks,
                queued_ticks: frame.queued_ticks,
                tick_cpu: frame.tick_cpu,
                extract_cpu: frame.extract_cpu,
                worker_queue_ns: duration_ns_u32(frame.worker_queue_cpu),
                worker_ready_ns: duration_ns_u32(frame.worker_ready_cpu),
                visual_age_ticks: u32::try_from(self.sim.tick().saturating_sub(frame.frame_tick))
                    .unwrap_or(u32::MAX),
                reused_visual: false,
                upload_cpu,
                encode_cpu: Duration::ZERO,
                wall: Duration::ZERO,
                gpu_pass_ns: None,
                gpu_present_ns: None,
                surface_presented: false,
                timing_fault: false,
                uploaded_rows: uploaded.changed_rows,
                uploaded_bytes: uploaded.bytes,
                moving,
                dropped_presentation: frame.extracted.dropped_capacity
                    + frame.extracted.dropped_duplicate
                    + frame.extracted.dropped_invalid
                    + dropped_invalid
                    + uploaded.dropped_invalid
                    + uploaded.dropped_shadow_envelope,
                dropped_light_references: self.prepared.dropped_light_references(),
                clamped_light_channels: self.prepared.presentation_light_clamps(),
                visual_revision,
            },
            wall_start,
        )?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.last_view_source = Some(SourceTick {
                epoch: self.sim.epoch().0,
                tick: frame.frame_tick,
            });
            self.last_render_slot = Some(slot_index);
        }
        Ok(())
    }

    fn encode_prepared(
        &mut self,
        slot_index: usize,
        report: &mut DriveReport,
        surface: Option<&SurfaceOutput<'_>>,
        sample: FrameSample,
        wall_start: Instant,
    ) -> Result<(), String> {
        let encode_start = Instant::now();
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
        self.finish_encoded(
            slot_index,
            report,
            encoder,
            sample,
            wall_start,
            encode_start,
            surface_presented,
        )
    }

    fn finish_encoded(
        &mut self,
        slot_index: usize,
        report: &mut DriveReport,
        encoder: wgpu::CommandEncoder,
        mut sample: FrameSample,
        wall_start: Instant,
        encode_start: Instant,
        surface_presented: bool,
    ) -> Result<(), String> {
        let slot = &mut self.slots[slot_index];
        slot.timings
            .finish_render(GENERATION, &encoder)
            .map_err(|error| format!("finish timestamps: {error:?}"))?;
        let encode_cpu = encode_start.elapsed();
        slot.render_done.store(false, Ordering::Release);
        self.queue.submit([encoder.finish()]);
        let done = Arc::clone(&slot.render_done);
        self.queue
            .on_submitted_work_done(move || done.store(true, Ordering::Release));
        sample.encode_cpu = encode_cpu;
        sample.wall = wall_start.elapsed();
        sample.surface_presented = surface_presented;
        sample.submission_order = self.next_submission_order;
        self.next_submission_order = self.next_submission_order.wrapping_add(1);
        slot.state = SlotState::Rendering(sample);
        report.submitted = true;
        report.surface_presented = surface_presented;
        report.wall = wall_start.elapsed();
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn submit_reused_view(
        &mut self,
        slot_index: usize,
        report: &mut DriveReport,
        surface: &SurfaceOutput<'_>,
        source: SourceTick,
        tick_cpu: Duration,
        wall_start: Instant,
    ) -> Result<(), String> {
        let encode_start = Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let slot = &mut self.slots[slot_index];
        self.presenter
            .as_ref()
            .ok_or("reused view needs a presenter")?
            .encode(
                &mut encoder,
                slot.presentation
                    .as_ref()
                    .ok_or("reused view source binding unavailable")?,
                surface.view,
                surface.extent,
                Some(&mut slot.timings),
            )
            .map_err(|error| format!("reused surface encode: {error:?}"))?;
        self.finish_encoded(
            slot_index,
            report,
            encoder,
            FrameSample {
                tick: source.tick,
                submission_order: 0,
                completed_ticks: report.completed_ticks,
                queued_ticks: report.queued_ticks,
                tick_cpu,
                extract_cpu: Duration::ZERO,
                worker_queue_ns: 0,
                worker_ready_ns: 0,
                visual_age_ticks: u32::try_from(self.sim.tick().saturating_sub(source.tick))
                    .unwrap_or(u32::MAX),
                reused_visual: true,
                upload_cpu: Duration::ZERO,
                encode_cpu: Duration::ZERO,
                wall: Duration::ZERO,
                gpu_pass_ns: None,
                gpu_present_ns: None,
                surface_presented: false,
                timing_fault: false,
                uploaded_rows: 0,
                uploaded_bytes: 0,
                moving: 0,
                dropped_presentation: 0,
                dropped_light_references: self.prepared.dropped_light_references(),
                clamped_light_channels: self.prepared.presentation_light_clamps(),
                visual_revision: self.current_visual_revision(),
            },
            wall_start,
            encode_start,
            true,
        )?;
        self.reused_views = self.reused_views.saturating_add(1);
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn drive_native_view(
        &mut self,
        report: &mut DriveReport,
        tick_cpu: Duration,
        wall_start: Instant,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<(), String> {
        if let Some(completed) = self
            .worker
            .as_mut()
            .expect("native worker enabled")
            .try_complete()?
        {
            debug_assert!(self.worker_ready.is_none());
            self.worker_ready = Some(completed);
        }
        if self.worker_ready.is_none() {
            if report.completed_ticks > 0 {
                if self
                    .worker
                    .as_ref()
                    .expect("native worker enabled")
                    .is_busy()
                {
                    self.dropped_views = self.dropped_views.saturating_add(1);
                    report.dropped_view = true;
                    self.try_submit_reused_view(report, tick_cpu, wall_start, surface)?;
                    return Ok(());
                }
                let publication_start = Instant::now();
                let Some(snapshot) = self.snapshots.publish(&self.sim) else {
                    self.dropped_views = self.dropped_views.saturating_add(1);
                    report.dropped_view = true;
                    return Ok(());
                };
                let publication_cpu = publication_start.elapsed();
                let extractor = self
                    .extractor
                    .take()
                    .expect("idle worker returns extractor");
                self.worker
                    .as_mut()
                    .expect("native worker enabled")
                    .dispatch(
                        extractor,
                        snapshot,
                        ExtractRequest {
                            generation: self.visual_generation,
                            publication_cpu,
                            tick_cpu,
                            completed_ticks: report.completed_ticks,
                            queued_ticks: report.queued_ticks,
                        },
                    );
                return Ok(());
            }
            if self
                .worker
                .as_ref()
                .expect("native worker enabled")
                .is_busy()
            {
                self.try_submit_reused_view(report, tick_cpu, wall_start, surface)?;
            }
            return Ok(());
        }
        if self.presenter.is_some()
            && surface.is_none_or(|surface| {
                self.presenter
                    .as_ref()
                    .is_none_or(|presenter| presenter.extent() != surface.extent)
            })
        {
            if report.completed_ticks > 0 {
                self.dropped_views = self.dropped_views.saturating_add(1);
                report.dropped_view = true;
            }
            return Ok(());
        }
        let Some(slot_index) = self
            .slots
            .iter()
            .position(|slot| matches!(slot.state, SlotState::Free))
        else {
            if report.completed_ticks > 0 {
                self.dropped_views = self.dropped_views.saturating_add(1);
                report.dropped_view = true;
            }
            return Ok(());
        };
        let completed = self.worker_ready.take().expect("checked ready view");
        let stale = completed.epoch != self.sim.epoch().0
            || completed.generation != self.visual_generation
            || self.sim.tick().saturating_sub(completed.tick) > 2;
        self.extractor = Some(completed.extractor);
        let frame = if stale {
            self.dropped_stale_views = self.dropped_stale_views.saturating_add(1);
            let extractor = self.extractor.as_mut().expect("returned extractor");
            extractor.invalidate_history();
            let start = Instant::now();
            let Some(snapshot) = self.snapshots.publish(&self.sim) else {
                self.dropped_views = self.dropped_views.saturating_add(1);
                report.dropped_view = true;
                return Ok(());
            };
            let extracted = scene::extract(extractor, &snapshot);
            drop(snapshot);
            SubmitFrame {
                extracted,
                extract_cpu: start.elapsed(),
                worker_queue_cpu: completed.queue_wait,
                worker_ready_cpu: completed.completed_at.elapsed(),
                tick_cpu,
                completed_ticks: report.completed_ticks,
                queued_ticks: report.queued_ticks,
                frame_tick: self.sim.tick(),
            }
        } else {
            SubmitFrame {
                extracted: completed.report,
                extract_cpu: completed.publication_cpu + completed.cpu,
                worker_queue_cpu: completed.queue_wait,
                worker_ready_cpu: completed.completed_at.elapsed(),
                tick_cpu: completed.tick_cpu,
                completed_ticks: completed.completed_ticks,
                queued_ticks: completed.queued_ticks,
                frame_tick: completed.tick,
            }
        };
        self.submit_extracted(slot_index, report, surface, frame, wall_start)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn try_submit_reused_view(
        &mut self,
        report: &mut DriveReport,
        tick_cpu: Duration,
        wall_start: Instant,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<(), String> {
        let Some((surface, source)) = surface.zip(self.last_view_source) else {
            return Ok(());
        };
        if source.epoch != self.sim.epoch().0
            || self
                .presenter
                .as_ref()
                .is_none_or(|presenter| presenter.extent() != surface.extent)
        {
            return Ok(());
        }
        let Some(slot_index) = self.last_render_slot.filter(|&index| {
            self.slots
                .get(index)
                .is_some_and(|slot| matches!(slot.state, SlotState::Free))
        }) else {
            return Ok(());
        };
        self.submit_reused_view(slot_index, report, surface, source, tick_cpu, wall_start)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn current_visual_revision(&self) -> Option<[u8; 32]> {
        self.visual_product
            .as_ref()
            .map(|product| *product.revision().as_bytes())
    }

    /// Render and read back the current committed scene once for visual evidence.
    /// This capture is outside the timed frame path.
    /// # Errors
    /// Returns a GPU encode, completion or readback error.
    pub fn capture_pixels(&mut self) -> Result<Vec<u8>, String> {
        #[cfg(not(target_arch = "wasm32"))]
        self.reclaim_extractor_for_capture()?;
        if self
            .extractor
            .as_ref()
            .ok_or("capture extractor unavailable")?
            .frame()
            .source()
            .is_none_or(|source| source.tick != self.sim.tick())
        {
            let snapshot = self
                .snapshots
                .publish(&self.sim)
                .ok_or("visual snapshot pages held during capture")?;
            let extracted = scene::extract(
                self.extractor
                    .as_mut()
                    .ok_or("capture extractor unavailable")?,
                &snapshot,
            );
            drop(snapshot);
            let applied = scene::apply_frame_to_scene(
                &mut self.scene,
                self.extractor
                    .as_ref()
                    .ok_or("capture extractor unavailable")?
                    .frame(),
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

    #[cfg(not(target_arch = "wasm32"))]
    fn reclaim_extractor_for_capture(&mut self) -> Result<(), String> {
        if self.extractor.is_some() {
            return Ok(());
        }
        let completed = if let Some(ready) = self.worker_ready.take() {
            ready
        } else {
            self.worker
                .as_mut()
                .expect("worker owns absent extractor")
                .wait_complete(Duration::from_secs(2))?
        };
        let mut extractor = completed.extractor;
        // The returned columns have not been mapped or uploaded to the scene.
        // Force the ordinary capture refresh even when their source tick equals
        // the current world tick.
        extractor.invalidate_history();
        self.extractor = Some(extractor);
        self.dropped_views = self.dropped_views.saturating_add(1);
        Ok(())
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
                            sample.reused_visual,
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
    reused_visual: bool,
) -> Option<([f64; 4], Option<f64>)> {
    if reused_visual {
        return (passes.len() == 1
            && presented
            && passes[0].name.as_ref() == thrum_render::present::PASS_NAME)
            .then(|| ([0.0; 4], Some(passes[0].nanoseconds)));
    }
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

fn duration_ns_u32(value: Duration) -> u32 {
    u32::try_from(value.as_nanos()).unwrap_or(u32::MAX)
}

fn retain_sample(queue: &mut VecDeque<FrameSample>, dropped: &mut u64, sample: FrameSample) {
    if queue.len() == IN_FLIGHT {
        if queue
            .front()
            .is_some_and(|oldest| sample.submission_order <= oldest.submission_order)
        {
            *dropped += 1;
            return;
        }
        queue.pop_front();
        *dropped += 1;
    }
    let insert = queue
        .iter()
        .position(|prior| prior.submission_order > sample.submission_order)
        .unwrap_or(queue.len());
    queue.insert(insert, sample);
}

fn pop_report_before_pending(
    queue: &mut VecDeque<FrameSample>,
    oldest_pending: Option<u64>,
) -> Option<FrameSample> {
    queue
        .front()
        .is_some_and(|sample| {
            oldest_pending.is_none_or(|pending| sample.submission_order < pending)
        })
        .then(|| queue.pop_front())
        .flatten()
}

#[cfg(test)]
mod tests;
