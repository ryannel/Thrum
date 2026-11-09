//! Opt-in observer of the ordinary foreground renderer. No GPU work is submitted here.
use super::render::{FrameReply, FrameRequest, Lease, Signals};
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use thrum_app::capture_writer::{ImageWriter, SpoolEncoding};
use thrum_render::runtime::CaptureLimits;
use v1_game::ThreadedFrameHost;

const MAX_IMAGES: usize = 1024;
const MAX_CALLBACKS: usize = 4096;
const LIMITS: CaptureLimits = CaptureLimits {
    staging_slots: 6,
    cpu_buffers: 32,
    byte_allowance: 1024 * 1024 * 1024,
};

/// UIKit appends only during a session and never waits on the observer.
#[derive(Default)]
pub(super) struct Callbacks {
    rows: Mutex<Option<Vec<(Instant, f64)>>>,
    lost: AtomicU64,
    enabled: AtomicBool,
}
impl Callbacks {
    pub(super) fn observe(&self, target: f64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        match self.rows.try_lock() {
            Ok(mut rows) => {
                if let Some(rows) = &mut *rows {
                    if rows.len() < MAX_CALLBACKS {
                        rows.push((Instant::now(), target));
                    } else {
                        self.lost.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
            Err(_) => {
                self.lost.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    fn begin(&self) {
        *self
            .rows
            .lock()
            .expect("capture callback lock is never held across fallible work") =
            Some(Vec::with_capacity(MAX_CALLBACKS));
        self.lost.store(0, Ordering::Relaxed);
        self.enabled.store(true, Ordering::Relaxed);
    }
    fn end(&self, origin: Instant, opportunities: &[Value]) -> Value {
        self.enabled.store(false, Ordering::Relaxed);
        let rows = self
            .rows
            .lock()
            .expect("capture callback lock is never held across fallible work")
            .take()
            .unwrap_or_default();
        json!({"lost":self.lost.load(Ordering::Relaxed),"rows":rows.into_iter().map(|(time,target)| {
            let used=opportunities.iter().any(|row|row["target_media_seconds"].as_f64()==Some(target));
            json!({"ns":time.saturating_duration_since(origin).as_nanos(),"target_media_seconds":target,
                "reason":if used {"dispatched-to-render-owner"} else {"pending-target-coalesced-or-session-ended"}})
        }).collect::<Vec<_>>()})
    }
}

#[derive(Default)]
pub(super) struct Recorder {
    active: Option<Session>,
    next: u64,
    finalizing: Option<Arc<AtomicBool>>,
}
struct Session {
    id: u64,
    epoch: u64,
    generation: u64,
    mode: u64,
    started: Instant,
    ending: bool,
    writer: ImageWriter,
    images: Vec<Value>,
    opportunities: Vec<Value>,
    writer_omissions: u64,
    expected: u64,
    peak: (usize, usize),
    path: PathBuf,
}
struct Sink {
    file: File,
    slow: bool,
}
impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.slow {
            thread::sleep(Duration::from_millis(100));
        }
        self.file.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.sync_all()
    }
}

impl Recorder {
    pub(super) fn failed(&mut self, host: &mut ThreadedFrameHost, signals: &Signals, error: &str) {
        drop(self.cancel(host, signals, error));
        host.disable_submitted_capture();
        drop(fs::write(
            std::env::temp_dir().join("thrum-capture-error.txt"),
            error,
        ));
    }

    pub(super) fn before(
        &mut self,
        host: &mut ThreadedFrameHost,
        signals: &Signals,
        lease: &Lease,
        request: &FrameRequest,
        generation: u64,
        adapter: &wgpu::Adapter,
    ) -> Result<(), String> {
        if self.active.as_ref().is_some_and(|s| {
            s.epoch != lease.epoch()
                || s.generation != generation
                || signals.background.load(Ordering::Acquire)
        }) {
            self.cancel(host, signals, "foreground/surface/device lease retired")?;
        }
        let command = signals.capture.swap(0, Ordering::AcqRel);
        if command == 4 {
            if let Some(s) = &mut self.active {
                s.ending = true;
                host.stop_submitted_capture();
            }
        } else if command != 0 {
            if self.active.is_some()
                || self
                    .finalizing
                    .as_ref()
                    .is_some_and(|flag| flag.load(Ordering::Acquire))
            {
                return Err("capture session active or finalizing".into());
            }
            self.next += 1;
            let path = std::env::temp_dir().join(format!("thrum-capture-{}", self.next));
            fs::create_dir(&path).map_err(|e| e.to_string())?;
            let encoding = if command == 3 {
                SpoolEncoding::Raw
            } else {
                SpoolEncoding::ZstdDelta
            };
            let writer = ImageWriter::start(
                Sink {
                    file: File::create(path.join("frames.spool")).map_err(|e| e.to_string())?,
                    slow: command == 3,
                },
                LIMITS.cpu_buffers,
                8 * 1024 * 1024 * 1024,
                encoding,
            )?;
            let started = Instant::now();
            host.enable_submitted_capture(self.next, LIMITS)?;
            fs::write(path.join("identity.json"), json!({"session":self.next,"adapter":format!("{:?}",adapter.get_info()),
                "downlevel":format!("{:?}",adapter.get_downlevel_capabilities()),"backend":"Metal",
                "surface_extent":request.extent,"surface_epoch":request.epoch,"device_generation":generation,
                "mode":"realtime-windowed-tap","clock":"Instant monotonic elapsed / ordinary world lane / UIKit CADisplayLink",
                "scope":"simulator function only; final SDR engine output before surface presentation; UIKit and native UI excluded",
                "limits":{"gpu_slots":LIMITS.staging_slots,"cpu_buffers":LIMITS.cpu_buffers,"pixel_byte_allowance":LIMITS.byte_allowance,
                    "writer_queue":LIMITS.cpu_buffers,"encoded_reserve_bytes":268_435_456,"output_allowance":8_589_934_592_u64,
                    "max_records":MAX_IMAGES,"max_seconds":16},"control":command,"encoding":format!("{encoding:?}")}).to_string()).map_err(|e| e.to_string())?;
            signals.capture_callbacks.begin();
            self.active = Some(Session {
                id: self.next,
                epoch: request.epoch,
                generation,
                mode: command,
                started,
                ending: false,
                writer,
                images: Vec::new(),
                opportunities: Vec::new(),
                writer_omissions: 0,
                expected: 0,
                peak: (0, 0),
                path,
            });
        }
        if self.active.as_ref().is_some_and(|s| {
            s.started.elapsed() > Duration::from_secs(16) || s.opportunities.len() >= MAX_IMAGES
        }) {
            self.cancel(host, signals, "session duration/record allowance exhausted")?;
        }
        if let Some(s) = &mut self.active {
            s.opportunities.push(json!({"request_ns":s.started.elapsed().as_nanos(),"phase":if s.ending {"draining"} else {"capturing"},
                "target_media_seconds":request.target_media_seconds,"surface_epoch":request.epoch,"device_generation":generation}));
        }
        Ok(())
    }

    pub(super) fn after(
        &mut self,
        host: &mut ThreadedFrameHost,
        reply: &FrameReply,
        lease: &Lease,
        signals: &Signals,
    ) -> Result<(), String> {
        let Some(s) = &mut self.active else {
            return Ok(());
        };
        if s.epoch != lease.epoch() || signals.background.load(Ordering::Acquire) {
            return self.cancel(host, signals, "surface/background revoked during frame");
        }
        if let Some(row) = s.opportunities.last_mut() {
            row["status"] = json!(format!("{:?}", reply.status));
            row["report"] = json!(reply.report.map(|r| json!({"tick":r.tick,"submitted":r.submitted,
                "surface_presented":r.surface_presented,"dropped_view":r.dropped_view,"overload":r.overload})));
            row["returned_ns"] = json!(s.started.elapsed().as_nanos());
            row["cooked"] = json!(reply.cooked);
        }
        if !s.ending && reply.report.is_some_and(|r| r.submitted) {
            s.expected += 1;
        }
        if let Some((gpu, cpu)) = host.submitted_capture_usage() {
            s.peak.0 = s.peak.0.max(gpu);
            s.peak.1 = s.peak.1.max(cpu);
        }
        while let Some(image) = s.writer.recycled() {
            host.recycle_submitted_image(image);
        }
        // Ring saturation is an explicit control. Ordinary capture always collects.
        if s.mode != 2 || s.ending {
            while let Some(image) = host.try_submitted_image()? {
                if s.images.len() >= MAX_IMAGES {
                    return self.cancel(host, signals, "image record allowance exhausted");
                }
                let row = json!({"submission_id":image.sample.submission_order,"tick":image.sample.tick,
                    "source_epoch":image.source.epoch.to_string(),"source_tick":image.source.tick,"timeline":image.source.timeline,
                    "previous_endpoint_tick":image.source.tick.checked_sub(1),"alpha":image.sample.presentation_alpha,"history_epoch":image.history_epoch,
                    "device_generation":image.device_generation,"presenter_generation":image.presenter_generation,
                    "surface_epoch":s.epoch,"surface_presented":image.sample.surface_presented,"extent":image.extent,"row_stride":image.extent[0]*4,
                    "visual_revision":image.sample.visual_revision,"presentation_lead_ns":image.presentation_lead.as_nanos(),
                    "submit_ns":image.submitted_at.saturating_duration_since(s.started).as_nanos(),
                    "tap_elapsed_ns":image.submitted_elapsed.as_nanos(),"map_ns":s.started.elapsed().as_nanos()});
                if let Some(image) = s.writer.submit(image) {
                    s.writer_omissions += 1;
                    host.recycle_submitted_image(image);
                } else {
                    s.images.push(row);
                }
            }
        }
        let pending = host.submitted_capture_usage().unwrap_or_default();
        let status = json!({"pending":pending,"images":s.images.len(),"omissions":host.submitted_capture_omissions(),
            "expected":s.expected,"opportunities":s.opportunities.len(),"tick":reply.report.map(|r|r.tick)});
        // A small route witness is independently read by the driver before lifecycle actions.
        fs::write(s.path.join("pending.partial"), status.to_string())
            .and_then(|()| fs::rename(s.path.join("pending.partial"), s.path.join("pending.json")))
            .map_err(|e| e.to_string())?;
        if s.ending && pending.0 == 0 {
            self.finish(host, signals, None)?;
        }
        Ok(())
    }

    pub(super) fn cancel(
        &mut self,
        host: &mut ThreadedFrameHost,
        signals: &Signals,
        reason: &str,
    ) -> Result<(), String> {
        self.finish(host, signals, Some(reason.to_owned()))
    }
    fn finish(
        &mut self,
        host: &mut ThreadedFrameHost,
        signals: &Signals,
        reason: Option<String>,
    ) -> Result<(), String> {
        let Some(s) = self.active.take() else {
            return Ok(());
        };
        let callbacks = signals.capture_callbacks.end(s.started, &s.opportunities);
        let omissions = host.submitted_capture_omissions();
        let pending = host.submitted_capture_usage();
        host.disable_submitted_capture();
        let finalizing = Arc::new(AtomicBool::new(true));
        self.finalizing = Some(Arc::clone(&finalizing));
        thread::Builder::new().name("thrum-ios-capture-finalize".into()).spawn(move || {
            let (writer,error)=match s.writer.finish(Duration::from_secs(10)) { Ok(w)=>(Some(w),None),Err(e)=>(None,Some(e)) };
            let complete=reason.is_none() && error.is_none() && omissions==0 && s.writer_omissions==0 && !s.images.is_empty()
                && callbacks["lost"]==0 && s.expected==u64::try_from(s.images.len()).unwrap_or(u64::MAX)
                && writer.as_ref().is_some_and(|w|Some(w.frames)==u64::try_from(s.images.len()).ok());
            let report=json!({"session":s.id,"complete":complete,"cancel_reason":reason,"writer_error":error,
                "renderer_omissions":omissions,"writer_omissions":s.writer_omissions,"expected_submissions":s.expected,"pending_at_retirement":pending,
                "peak":s.peak,"images":s.images,"opportunities":s.opportunities,"callbacks":callbacks,
                "writer":writer.map(|w|json!({"frames":w.frames,"bytes":w.bytes,"encoded_bytes":w.encoded_bytes,"hashes":w.frame_hashes,
                    "io_ns":w.io_time.as_nanos(),"write_ns":w.write_time.as_nanos(),"flush_ns":w.flush_time.as_nanos(),"encoded_queue_peak":w.encoded_queue_peak}))});
            let result=fs::write(s.path.join("runtime.partial"),report.to_string()).and_then(|()|fs::rename(s.path.join("runtime.partial"),s.path.join("runtime.json")));
            drop(result); // An unfinalized directory is refused by the external validator.
            if error.as_deref() != Some("raw writer finalization timeout") { finalizing.store(false,Ordering::Release); }
        }).map_err(|e|e.to_string())?;
        Ok(())
    }
}
