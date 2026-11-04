//! Serial offscreen frame host for the first assembled V1 workload.

use std::time::{Duration, Instant};

use thrum_core::{ClockConfig, ClockState, TickClock, sim::Simulation};
use thrum_render::{
    extract::RenderExtractor,
    synthetic::{OffscreenTarget, PreparedSynthetic, SyntheticScene},
    timing::GpuTimings,
};

use crate::scene::{self, ApplyReport, ENTITY_COUNT};

const GENERATION: u64 = 1;
const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;
const GPU_WAIT: Duration = Duration::from_secs(10);

/// One completed simulation-to-GPU frame. GPU pass values are absent when the
/// adapter lacks timestamp queries. Wall time includes explicit GPU waits;
/// these fields must not be summed as if CPU/GPU work were independent.
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
    /// Total host-call wall time, including render and timestamp waits.
    pub wall: Duration,
    /// Actual GPU timestamps for forward and display passes, in nanoseconds.
    pub gpu_pass_ns: Option<[f64; 2]>,
    /// Records whose changed ABI bytes were enqueued for upload.
    pub uploaded_rows: usize,
    /// Queue payload bytes for this update.
    pub uploaded_bytes: usize,
    /// Accepted movers with valid adjacent-tick previous positions.
    pub moving: usize,
    /// Presentation samples/updates dropped with explicit telemetry.
    pub dropped_presentation: usize,
}

/// One fixed-clock simulation and prepared GPU scene. The device and queue
/// belong to the caller's executor; this host creates no independent GPU owner.
/// It serializes submissions and waits, so it is an assembled diagnostic rather
/// than the final asynchronous frame scheduler or platform surface.
pub struct FrameHost<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    clock: TickClock,
    sim: Simulation,
    extractor: RenderExtractor,
    scene: SyntheticScene,
    changed_slots: Vec<u32>,
    prepared: PreparedSynthetic,
    target: OffscreenTarget,
    timings: GpuTimings,
}

impl<'a> FrameHost<'a> {
    /// Prepare one 10k-entity scene and both GPU passes before the first draw.
    /// # Errors
    /// Returns setup, device or scene preparation errors without a partial host.
    pub async fn new(device: &'a wgpu::Device, queue: &'a wgpu::Queue) -> Result<Self, String> {
        let sim = scene::simulation()?;
        let mut extractor = RenderExtractor::new(ENTITY_COUNT);
        scene::extract(&mut extractor, &sim);
        let count = u32::try_from(ENTITY_COUNT).map_err(|_| "fixture exceeds u32 slots")?;
        let mut scene =
            SyntheticScene::grid(count).map_err(|error| format!("synthetic scene: {error:?}"))?;
        let mut changed_slots = Vec::with_capacity(ENTITY_COUNT);
        scene::apply_frame_to_scene(&mut scene, extractor.frame(), &mut changed_slots);
        let prepared = PreparedSynthetic::prepare(device, queue, GENERATION, &scene)
            .await
            .map_err(|error| format!("prepare: {error:?}"))?;
        let target = OffscreenTarget::new(device, &prepared, WIDTH, HEIGHT)
            .map_err(|error| format!("target: {error:?}"))?;
        let timings = GpuTimings::new(device, queue, GENERATION, 2)
            .map_err(|error| format!("GPU timings: {error:?}"))?;
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
            target,
            timings,
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

    /// Admit monotonic host time through the shared fixed-step clock. Every due
    /// update commits once before the latest committed view is extracted. If no
    /// update is due, no GPU frame is encoded. Overload is returned explicitly.
    /// # Errors
    /// Returns a clock, simulation or GPU fault; never silently skips an update.
    pub fn drive(&mut self, now: Duration) -> Result<Option<FrameSample>, String> {
        let wall_start = Instant::now();
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
        if clock.completed == 0 {
            return Ok(None);
        }

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
        self.prepared
            .encode(
                &mut encoder,
                GENERATION,
                &self.target,
                Some(&mut self.timings),
            )
            .map_err(|error| format!("render encode: {error:?}"))?;
        self.timings
            .finish_render(GENERATION, &encoder)
            .map_err(|error| format!("finish timestamps: {error:?}"))?;
        let encode_cpu = encode_start.elapsed();
        let submitted = self.queue.submit([encoder.finish()]);
        self.wait(submitted, "render")?;
        let gpu_pass_ns = self.collect_gpu_passes()?;
        Ok(Some(FrameSample {
            tick: self.sim.tick(),
            completed_ticks: clock.completed,
            queued_ticks: clock.queued,
            tick_cpu,
            extract_cpu,
            upload_cpu,
            encode_cpu,
            wall: wall_start.elapsed(),
            gpu_pass_ns,
            uploaded_rows: uploaded.changed_rows,
            uploaded_bytes: uploaded.bytes,
            moving,
            dropped_presentation: extracted.dropped_capacity
                + extracted.dropped_duplicate
                + extracted.dropped_invalid
                + dropped_invalid
                + uploaded.dropped_invalid,
        }))
    }

    /// Render and read back the current committed scene once for visual evidence.
    /// This capture is outside the timed frame path.
    /// # Errors
    /// Returns a GPU encode, completion or readback error.
    pub fn capture_pixels(&self) -> Result<Vec<u8>, String> {
        let mut pixels = self.target.readback(self.device);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.prepared
            .encode(&mut encoder, GENERATION, &self.target, None)
            .map_err(|error| format!("capture render: {error:?}"))?;
        pixels
            .encode(&mut encoder, &self.target)
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

    fn collect_gpu_passes(&mut self) -> Result<Option<[f64; 2]>, String> {
        if !self.timings.is_enabled() {
            return Ok(None);
        }
        let mut resolve = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        if !self
            .timings
            .resolve_if_ready(GENERATION, &mut resolve)
            .map_err(|error| format!("resolve timestamps: {error:?}"))?
        {
            return Err("render timestamps not ready after submission completion".to_owned());
        }
        let submitted = self.queue.submit([resolve.finish()]);
        self.wait(submitted, "timestamp")?;
        let passes = self
            .timings
            .try_collect(GENERATION)
            .map_err(|error| format!("collect timestamps: {error:?}"))?
            .ok_or_else(|| "timestamp readback incomplete".to_owned())?;
        if passes.len() != 2
            || passes[0].name.as_ref() != "synthetic forward opaque"
            || passes[1].name.as_ref() != "synthetic tone encode"
        {
            return Err("prepared GPU pass order changed".to_owned());
        }
        Ok(Some([passes[0].nanoseconds, passes[1].nanoseconds]))
    }
}
