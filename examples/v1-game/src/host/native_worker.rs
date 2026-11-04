//! One bounded native extraction lane over committed visual pages.

use std::{
    sync::{
        Arc,
        mpsc::{self, Receiver, SyncSender, TryRecvError},
    },
    time::{Duration, Instant},
};

use rayon::{ThreadPool, ThreadPoolBuilder};
use thrum_core::sim::PublishedPage;
use thrum_render::extract::{ExtractReport, RenderExtractor};

use crate::scene;

pub(super) struct CompletedExtract {
    pub(super) extractor: RenderExtractor,
    pub(super) report: ExtractReport,
    pub(super) generation: u64,
    pub(super) epoch: u128,
    pub(super) tick: u64,
    pub(super) cpu: Duration,
    pub(super) publication_cpu: Duration,
    pub(super) queue_wait: Duration,
    pub(super) completed_at: Instant,
    pub(super) tick_cpu: Duration,
    pub(super) completed_ticks: u32,
    pub(super) queued_ticks: u32,
}

#[derive(Clone, Copy)]
pub(super) struct ExtractRequest {
    pub(super) generation: u64,
    pub(super) publication_cpu: Duration,
    pub(super) tick_cpu: Duration,
    pub(super) completed_ticks: u32,
    pub(super) queued_ticks: u32,
}

pub(super) struct NativeWorker {
    pool: ThreadPool,
    send: SyncSender<CompletedExtract>,
    receive: Receiver<CompletedExtract>,
    wake: Arc<dyn Fn() + Send + Sync>,
    in_flight: bool,
}

impl NativeWorker {
    pub(super) fn new(wake: impl Fn() + Send + Sync + 'static) -> Result<Self, String> {
        let pool = ThreadPoolBuilder::new()
            .num_threads(1)
            .thread_name(|index| format!("thrum-v1-extract-{index}"))
            .build()
            .map_err(|error| format!("frame worker: {error}"))?;
        let (send, receive) = mpsc::sync_channel(1);
        Ok(Self {
            pool,
            send,
            receive,
            wake: Arc::new(wake),
            in_flight: false,
        })
    }

    pub(super) fn is_busy(&self) -> bool {
        self.in_flight
    }

    pub(super) fn dispatch(
        &mut self,
        extractor: RenderExtractor,
        snapshot: PublishedPage,
        request: ExtractRequest,
    ) {
        debug_assert!(!self.in_flight);
        self.in_flight = true;
        let send = self.send.clone();
        let wake = Arc::clone(&self.wake);
        let dispatched_at = Instant::now();
        self.pool.spawn(move || {
            let queue_wait = dispatched_at.elapsed();
            let epoch = snapshot.id().epoch.0;
            let tick = snapshot.id().tick;
            let mut extractor = extractor;
            let start = Instant::now();
            let report = scene::extract(&mut extractor, &snapshot);
            let cpu = start.elapsed();
            drop(snapshot);
            if send
                .send(CompletedExtract {
                    extractor,
                    report,
                    generation: request.generation,
                    epoch,
                    tick,
                    cpu,
                    publication_cpu: request.publication_cpu,
                    queue_wait,
                    completed_at: Instant::now(),
                    tick_cpu: request.tick_cpu,
                    completed_ticks: request.completed_ticks,
                    queued_ticks: request.queued_ticks,
                })
                .is_ok()
            {
                wake();
            }
        });
    }

    pub(super) fn try_complete(&mut self) -> Result<Option<CompletedExtract>, String> {
        match self.receive.try_recv() {
            Ok(completed) => {
                self.in_flight = false;
                Ok(Some(completed))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err("frame extraction worker stopped".to_owned()),
        }
    }

    pub(super) fn wait_complete(&mut self, timeout: Duration) -> Result<CompletedExtract, String> {
        let completed = self
            .receive
            .recv_timeout(timeout)
            .map_err(|error| format!("frame extraction wait: {error}"))?;
        self.in_flight = false;
        Ok(completed)
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, time::Duration};

    use thrum_render::extract::RenderExtractor;

    use super::{ExtractRequest, NativeWorker};
    use crate::scene::{self, ENTITY_COUNT};

    #[test]
    fn native_worker_extracts_the_committed_page_and_returns_its_owner() {
        let mut simulation = scene::simulation().expect("fixture");
        simulation.step(&[]).expect("committed tick");
        let snapshot = simulation.published_page().expect("page available");
        let mut serial = RenderExtractor::new(ENTITY_COUNT, ENTITY_COUNT);
        let serial_report = scene::extract(&mut serial, &snapshot);
        let (wake, woke) = mpsc::sync_channel(1);
        let mut worker = NativeWorker::new(move || {
            let _ = wake.try_send(());
        })
        .expect("worker starts");
        worker.dispatch(
            RenderExtractor::new(ENTITY_COUNT, ENTITY_COUNT),
            snapshot,
            ExtractRequest {
                generation: 7,
                publication_cpu: Duration::ZERO,
                tick_cpu: Duration::ZERO,
                completed_ticks: 1,
                queued_ticks: 0,
            },
        );
        let complete = worker
            .wait_complete(Duration::from_secs(2))
            .expect("bounded extraction completes");
        assert_eq!(complete.tick, 1);
        assert_eq!(complete.epoch, 1);
        assert_eq!(complete.generation, 7);
        assert_eq!(complete.report, serial_report);
        assert_eq!(complete.extractor.frame().ids(), serial.frame().ids());
        assert_eq!(
            complete.extractor.frame().current(),
            serial.frame().current()
        );
        woke.recv_timeout(Duration::from_secs(2))
            .expect("worker wake");
        assert!(!worker.is_busy());
        assert!(
            simulation.can_publish_next(),
            "lease released before completion"
        );
    }
}
