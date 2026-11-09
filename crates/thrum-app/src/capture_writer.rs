//! Bounded writer isolated from world/render lanes.

use sha2::{Digest, Sha256};
use std::{
    io::Write,
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use thrum_render::runtime::SubmittedImage;
mod io_worker;
mod spool;

/// Live spool representation; both preserve the complete original byte stream.
#[derive(Clone, Copy, Debug)]
pub enum SpoolEncoding {
    /// Uncompressed diagnostic control.
    Raw,
    /// Two-worker Zstd level 1 over exact consecutive-frame XOR differences.
    ZstdDelta,
}

/// Writer-thread observations, separate from render/GPU work and storage flush.
#[derive(Clone, Debug, Default)]
pub struct WriterReport {
    /// Raw payload bytes successfully handed to the output.
    pub bytes: u64,
    /// Complete image records written.
    pub frames: u64,
    /// Actual encoded bytes accepted by the output, including stream termination.
    pub encoded_bytes: u64,
    /// Wall inside underlying I/O calls, separate from total codec/write time.
    pub io_time: Duration,
    /// Observed encoded buffer occupancy, bounded by 64 × 4 MiB buffers.
    pub encoded_queue_peak: usize,
    /// SHA-256 of each original RGBA record in writer order, at most 1024.
    pub frame_hashes: Vec<String>,
    /// Wall time inside writes, including downstream backpressure.
    pub write_time: Duration,
    /// Output's flush wall time; durable storage sync is the spool tool's job.
    pub flush_time: Duration,
}

/// A finite queue of CPU image leases. Writes and downstream backpressure occur
/// only on this thread; the render owner always uses nonblocking admission.
pub struct ImageWriter<I = SubmittedImage> {
    send: Option<SyncSender<I>>,
    returned: Receiver<I>,
    outcome: Receiver<Result<WriterReport, String>>,
    thread: Option<JoinHandle<()>>,
}

fn write_records<I: AsRef<[u8]>, W: Write>(
    receive: &Receiver<I>,
    recycle: &SyncSender<I>,
    output: W,
    byte_allowance: u64,
    encoding: SpoolEncoding,
) -> Result<WriterReport, String> {
    let mut output =
        spool::Spool::new(output, byte_allowance, encoding).map_err(|e| e.to_string())?;
    let mut report = WriterReport::default();
    while let Ok(image) = receive.recv() {
        let bytes = image.as_ref();
        if report.frames >= 1024 || bytes.is_empty() {
            return Err("capture record limit or empty record".to_owned());
        }
        let begin = Instant::now();
        let hash = format!("{:x}", Sha256::digest(bytes));
        let length = bytes.len() as u64;
        let result = output.record(bytes);
        report.write_time += begin.elapsed();
        drop(recycle.try_send(image));
        result.map_err(|e| e.to_string())?;
        report.bytes += length;
        report.frames += 1;
        report.frame_hashes.push(hash);
    }
    let begin = Instant::now();
    let output = output.finish().map_err(|e| e.to_string())?;
    report.flush_time = begin.elapsed();
    report.encoded_bytes = output.bytes;
    report.io_time = output.io_time;
    Ok(report)
}

impl<I: AsRef<[u8]> + Send + 'static> ImageWriter<I> {
    /// Start a bounded sequential sink. Encoding and hashing run only on this
    /// writer; each original record is independently hashed before encoding.
    /// PNG/movie packaging is deferred. Sessions refuse more than 1024 records.
    /// # Errors
    /// Returns a writer-thread creation error.
    pub fn start(
        output: impl Write + Send + 'static,
        capacity: usize,
        byte_allowance: u64,
        encoding: SpoolEncoding,
    ) -> Result<Self, String> {
        if !(1..=256).contains(&capacity) {
            return Err("writer capacity must be in 1..=256".to_owned());
        }
        let (send, receive) = mpsc::sync_channel::<I>(capacity);
        let (recycle, returned) = mpsc::sync_channel(capacity);
        let (finish, outcome) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("thrum-lossless-spool".to_owned())
            .spawn(move || {
                let result = match encoding {
                    SpoolEncoding::Raw => {
                        write_records(&receive, &recycle, output, byte_allowance, encoding)
                    }
                    SpoolEncoding::ZstdDelta => (|| {
                        let (output, worker) =
                            io_worker::Output::start(output).map_err(|e| e.to_string())?;
                        let result =
                            write_records(&receive, &recycle, output, byte_allowance, encoding);
                        let begin = Instant::now();
                        let io = worker.finish();
                        let tail = begin.elapsed();
                        let mut report = result.map_err(|error| match &io {
                            Err(disk) if error.contains("encoded I/O worker stopped") => {
                                disk.clone()
                            }
                            _ => error,
                        })?;
                        let io = io?;
                        report.flush_time += tail;
                        report.io_time = io.time;
                        report.encoded_bytes = io.bytes;
                        report.encoded_queue_peak = io.peak;
                        Ok(report)
                    })(),
                };
                drop(finish.try_send(result));
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            send: Some(send),
            returned,
            outcome,
            thread: Some(thread),
        })
    }

    /// Try to transfer an image's CPU lease. On refusal it remains the caller's.
    /// Returns the image when full, closed or failed; mark the interval incomplete.
    pub fn submit(&self, image: I) -> Option<I> {
        let Some(send) = &self.send else {
            return Some(image);
        };
        send.try_send(image).err().map(|e| match e {
            TrySendError::Full(image) | TrySendError::Disconnected(image) => image,
        })
    }

    /// Return one completed CPU lease without blocking.
    pub fn recycled(&self) -> Option<I> {
        self.returned.try_recv().ok()
    }

    /// Close admission and wait only to the explicit finalization deadline.
    /// # Errors
    /// Returns write/flush failure or timeout; no successful manifest may follow.
    pub fn finish(mut self, timeout: Duration) -> Result<WriterReport, String> {
        self.send = None;
        let deadline = Instant::now() + timeout;
        loop {
            while self.recycled().is_some() {}
            if let Ok(outcome) = self.outcome.try_recv() {
                if let Some(thread) = self.thread.take() {
                    drop(thread.join());
                }
                return outcome;
            }
            if Instant::now() >= deadline {
                return Err("raw writer finalization timeout".to_owned());
            }
            thread::sleep(Duration::from_millis(1));
        }
    }
}

#[cfg(test)]
mod tests;
