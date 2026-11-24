//! Finite encoded-byte handoff; disk waits never serialize the next image hash.
use super::WriterError;
use std::{
    io::{self, Write},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Debug)]
struct Stopped;

impl std::fmt::Display for Stopped {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("encoded I/O worker stopped")
    }
}

impl std::error::Error for Stopped {}

pub(super) fn is_stopped(error: &io::Error) -> bool {
    error.get_ref().is_some_and(|cause| cause.is::<Stopped>())
}

const CHUNK: usize = 4 * 1024 * 1024;
// A 580 MB moving burst over two seconds needs ~180 MB reserve at 200 MB/s.
// 256 MiB provides explicit encoded headroom, independently of pixel leases.
const BUFFERS: usize = 64;
pub(super) struct IoReport {
    pub bytes: u64,
    pub time: Duration,
    pub peak: usize,
}
pub(super) struct Worker {
    thread: JoinHandle<io::Result<IoReport>>,
}
pub(super) struct Output {
    send: SyncSender<Vec<u8>>,
    returned: Receiver<Vec<u8>>,
    free: Vec<Vec<u8>>,
    current: Vec<u8>,
    counts: Arc<(AtomicUsize, AtomicUsize)>,
}
impl Output {
    pub(super) fn start(mut output: impl Write + Send + 'static) -> io::Result<(Self, Worker)> {
        let (send, receive) = mpsc::sync_channel::<Vec<u8>>(BUFFERS);
        let (recycle, returned) = mpsc::sync_channel(BUFFERS);
        let counts = Arc::new((AtomicUsize::new(0), AtomicUsize::new(0)));
        let observed = Arc::clone(&counts);
        let thread = thread::Builder::new()
            .name("thrum-spool-io".to_owned())
            .spawn(move || {
                let mut report = IoReport {
                    bytes: 0,
                    time: Duration::ZERO,
                    peak: 0,
                };
                while let Ok(mut chunk) = receive.recv() {
                    let begin = Instant::now();
                    output.write_all(&chunk)?;
                    report.time += begin.elapsed();
                    report.bytes += chunk.len() as u64;
                    observed.0.fetch_sub(1, Ordering::Relaxed);
                    chunk.clear();
                    drop(recycle.try_send(chunk));
                }
                let begin = Instant::now();
                output.flush()?;
                report.time += begin.elapsed();
                report.peak = observed.1.load(Ordering::Relaxed);
                Ok(report)
            })?;
        let mut buffers: Vec<_> = (0..BUFFERS).map(|_| Vec::with_capacity(CHUNK)).collect();
        let current = buffers.pop().expect("nonzero encoded buffer budget");
        Ok((
            Self {
                send,
                returned,
                free: buffers,
                current,
                counts,
            },
            Worker { thread },
        ))
    }
    fn dispatch(&mut self) -> io::Result<()> {
        if self.current.is_empty() {
            return Ok(());
        }
        let count = self.counts.0.fetch_add(1, Ordering::Relaxed) + 1;
        self.counts.1.fetch_max(count, Ordering::Relaxed);
        self.send
            .send(std::mem::take(&mut self.current))
            .map_err(|_| io::Error::other(Stopped))?;
        self.free.extend(self.returned.try_iter());
        self.current = if let Some(buffer) = self.free.pop() {
            buffer
        } else {
            self.returned
                .recv()
                .map_err(|_| io::Error::other(Stopped))?
        };
        Ok(())
    }
}
impl Write for Output {
    fn write(&mut self, mut data: &[u8]) -> io::Result<usize> {
        let length = data.len();
        while !data.is_empty() {
            let n = data.len().min(CHUNK - self.current.len());
            self.current.extend_from_slice(&data[..n]);
            data = &data[n..];
            if self.current.len() == CHUNK {
                self.dispatch()?;
            }
        }
        Ok(length)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.dispatch()
    }
}
impl Worker {
    pub(super) fn finish(self) -> Result<IoReport, WriterError> {
        self.thread
            .join()
            .map_err(|_| WriterError::WorkerStopped)?
            .map_err(WriterError::Io)
    }
}
