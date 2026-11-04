//! Bounded offscreen frame host for the first assembled V1 workload.

use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use thrum_core::{ClockConfig, ClockState, TickClock, sim::Simulation};
use thrum_render::{
    extract::RenderExtractor,
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
    /// Actual GPU timestamps for near/far Sun shadows, forward and display.
    pub gpu_pass_ns: Option<[f64; 4]>,
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
    /// No frame slot was free, so this optional view was skipped.
    pub dropped_view: bool,
    /// CPU wall time for the entire callback, including nonblocking GPU polling.
    pub wall: Duration,
}

enum SlotState {
    Free,
    Rendering(FrameSample),
    Resolving(FrameSample),
}

struct FrameSlot {
    target: OffscreenTarget,
    timings: GpuTimings,
    render_done: Arc<AtomicBool>,
    state: SlotState,
}

/// One fixed-clock simulation and prepared GPU scene. The device and queue
/// belong to the caller's executor; this host creates no independent GPU owner.
/// Three prepared targets bound in-flight work and defer GPU timing readback.
/// This is still an offscreen diagnostic, not a platform surface.
pub struct FrameHost<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    clock: TickClock,
    sim: Simulation,
    extractor: RenderExtractor,
    scene: SyntheticScene,
    changed_slots: Vec<u32>,
    prepared: PreparedSynthetic,
    slots: Vec<FrameSlot>,
    completed: VecDeque<FrameSample>,
    dropped_views: u64,
    dropped_reports: u64,
    dropped_capture_rows: u64,
}

impl<'a> FrameHost<'a> {
    /// Prepare one 10k-entity scene and all four GPU passes before the first draw.
    /// # Errors
    /// Returns setup, device or scene preparation errors without a partial host.
    pub async fn new(device: &'a wgpu::Device, queue: &'a wgpu::Queue) -> Result<Self, String> {
        let sim = scene::simulation()?;
        let mut extractor = RenderExtractor::new(ENTITY_COUNT);
        scene::extract(&mut extractor, &sim);
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
        let mut slots = Vec::with_capacity(IN_FLIGHT);
        for _ in 0..IN_FLIGHT {
            slots.push(FrameSlot {
                target: OffscreenTarget::new(device, &prepared, WIDTH, HEIGHT)
                    .map_err(|error| format!("target: {error:?}"))?,
                timings: GpuTimings::new(device, queue, GENERATION, 4)
                    .map_err(|error| format!("GPU timings: {error:?}"))?,
                render_done: Arc::new(AtomicBool::new(false)),
                state: SlotState::Free,
            });
        }
        let clock = TickClock::new(ClockConfig::default(), Duration::ZERO)
            .map_err(|error| format!("clock: {error:?}"))?;
        Ok(Self {
            device,
            queue,
            clock,
            sim,
            extractor,
            scene,
            changed_slots,
            prepared,
            slots,
            completed: VecDeque::with_capacity(IN_FLIGHT),
            dropped_views: 0,
            dropped_reports: 0,
            dropped_capture_rows: 0,
        })
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
    pub fn drive(&mut self, now: Duration) -> Result<DriveReport, String> {
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
            dropped_view: false,
            wall: Duration::ZERO,
        };
        if clock.completed == 0 {
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
        let extracted = scene::extract(&mut self.extractor, &self.sim);
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
            .upload_instances(self.queue, &self.scene.instances, &self.changed_slots)
            .map_err(|error| format!("instance update: {error:?}"))?;
        let upload_cpu = upload_start.elapsed();

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
        });
        report.submitted = true;
        report.wall = wall_start.elapsed();
        Ok(report)
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
            let extracted = scene::extract(&mut self.extractor, &self.sim);
            let applied = scene::apply_frame_to_scene(
                &mut self.scene,
                self.extractor.frame(),
                &mut self.changed_slots,
            );
            let uploaded = self
                .prepared
                .upload_instances(self.queue, &self.scene.instances, &self.changed_slots)
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
        let target = OffscreenTarget::new(self.device, &self.prepared, WIDTH, HEIGHT)
            .map_err(|error| format!("capture target: {error:?}"))?;
        let mut pixels = target.readback(self.device);
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
                            slot.timings = new_timings(self.device, self.queue)?;
                            retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                        }
                    }
                }
                SlotState::Resolving(mut sample) => match slot.timings.try_collect(GENERATION) {
                    Ok(Some(passes)) => {
                        sample.gpu_pass_ns = pass_durations(&passes);
                        sample.timing_fault = sample.gpu_pass_ns.is_none();
                        retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                    }
                    Ok(None) => slot.state = SlotState::Resolving(sample),
                    Err(_) => {
                        sample.timing_fault = true;
                        slot.timings = new_timings(self.device, self.queue)?;
                        retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                    }
                },
            }
        }
        Ok(())
    }
}

fn new_timings(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<GpuTimings, String> {
    GpuTimings::new(device, queue, GENERATION, 4)
        .map_err(|error| format!("GPU timing recovery: {error:?}"))
}

fn pass_durations(passes: &[thrum_render::timing::PassTiming]) -> Option<[f64; 4]> {
    let names = [
        "synthetic Sun shadow near",
        "synthetic Sun shadow far",
        "synthetic forward opaque",
        "synthetic tone encode",
    ];
    (passes.len() == names.len()
        && passes
            .iter()
            .zip(names)
            .all(|(pass, name)| pass.name.as_ref() == name))
    .then(|| std::array::from_fn(|index| passes[index].nanoseconds))
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
