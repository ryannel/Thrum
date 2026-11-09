//! Bounded writer isolated from world/render lanes.

use std::{
    io::Write,
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use thrum_render::runtime::SubmittedImage;

/// Writer-thread observations, separate from render/GPU work and storage flush.
#[derive(Clone, Copy, Debug, Default)]
pub struct WriterReport {
    /// Raw payload bytes successfully handed to the output.
    pub bytes: u64,
    /// Complete image records written.
    pub frames: u64,
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

impl<I: AsRef<[u8]> + Send + 'static> ImageWriter<I> {
    /// Start a sequential raw RGBA sink. Returned image metadata retains record
    /// order/identity; no headers misalign full-frame disk writes. Compression is downstream
    /// and must be deferred until after the measured interval.
    /// # Errors
    /// Returns a writer-thread creation error.
    pub fn start(
        mut output: impl Write + Send + 'static,
        capacity: usize,
        byte_allowance: u64,
    ) -> Result<Self, String> {
        if !(1..=256).contains(&capacity) {
            return Err("writer capacity must be in 1..=256".to_owned());
        }
        let (send, receive) = mpsc::sync_channel::<I>(capacity);
        let (recycle, returned) = mpsc::sync_channel(capacity);
        let (finish, outcome) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("thrum-raw-spool".to_owned())
            .spawn(move || {
                let result = (|| {
                    let mut remaining = byte_allowance;
                    let mut report = WriterReport::default();
                    while let Ok(image) = receive.recv() {
                        let length =
                            u64::try_from(image.as_ref().len()).map_err(|e| e.to_string())?;
                        let begin = Instant::now();
                        let result = if length > remaining {
                            Err(std::io::Error::other(
                                "raw spool output-byte allowance exhausted",
                            ))
                        } else {
                            remaining -= length;
                            output.write_all(image.as_ref())
                        };
                        report.write_time += begin.elapsed();
                        drop(recycle.try_send(image));
                        result.map_err(|e| e.to_string())?;
                        report.bytes += length;
                        report.frames += 1;
                    }
                    let begin = Instant::now();
                    output.flush().map_err(|e| e.to_string())?;
                    report.flush_time = begin.elapsed();
                    Ok(report)
                })();
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
