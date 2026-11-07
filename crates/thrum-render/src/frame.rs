//! Bounded native frame handoff. The shell keeps OS events on its thread while
//! a worker owns the frame operation; neither request nor reply grows a queue.

use std::time::Duration;

/// How a platform display target related to the worker's consumption time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetLeadStatus {
    /// The target is still within the bounded lookahead.
    Future,
    /// The display deadline has already passed; render from current time.
    Late,
    /// The platform supplied an invalid timestamp; render from current time.
    Invalid,
    /// The target is farther ahead than the supported presentation lookahead.
    Clamped,
}

/// A bounded future sample for interpolation, plus a nonfatal timing status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetLead {
    /// Duration from the worker's current media-clock sample to the target.
    pub duration: Duration,
    /// Whether the target was future, expired, invalid or clamped.
    pub status: TargetLeadStatus,
}

/// Convert a platform media-clock target at the point the worker consumes it.
/// Expired and invalid targets use the current sample. A 50 ms cap prevents a
/// bad platform forecast from interpolating far past the next world tick.
#[must_use]
#[inline]
pub fn display_target_lead(target_seconds: f64, observed_seconds: f64) -> TargetLead {
    const MAX_LEAD_SECONDS: f64 = 0.05;
    let zero = |status| TargetLead {
        duration: Duration::ZERO,
        status,
    };
    if !target_seconds.is_finite()
        || !observed_seconds.is_finite()
        || target_seconds < 0.0
        || observed_seconds < 0.0
    {
        return zero(TargetLeadStatus::Invalid);
    }
    let remaining = target_seconds - observed_seconds;
    if !remaining.is_finite() {
        return zero(TargetLeadStatus::Invalid);
    }
    if remaining <= 0.0 {
        return zero(TargetLeadStatus::Late);
    }
    if remaining > MAX_LEAD_SECONDS {
        return TargetLead {
            duration: Duration::from_millis(50),
            status: TargetLeadStatus::Clamped,
        };
    }
    TargetLead {
        duration: Duration::from_secs_f64(remaining),
        status: TargetLeadStatus::Future,
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::{
            Arc, Condvar, Mutex, PoisonError,
            atomic::{AtomicBool, Ordering},
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    const SHUTDOWN_WAIT: Duration = Duration::from_millis(100);
    const SHUTDOWN_POLL: Duration = Duration::from_millis(1);

    struct Slot<T> {
        value: Mutex<Option<T>>,
        ready: Condvar,
    }

    impl<T> Slot<T> {
        fn empty() -> Self {
            Self {
                value: Mutex::new(None),
                ready: Condvar::new(),
            }
        }
    }

    struct Shared<Request, Reply> {
        request: Slot<Request>,
        reply: Slot<Reply>,
        closed: AtomicBool,
    }

    impl<Request, Reply> Shared<Request, Reply> {
        fn close(&self) -> bool {
            // Hold both predicate locks while closing so a waiter cannot miss
            // the notification between checking the state and sleeping.
            let request = self
                .request
                .value
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let reply = self
                .reply
                .value
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let was_closed = self.closed.swap(true, Ordering::AcqRel);
            self.request.ready.notify_all();
            self.reply.ready.notify_all();
            drop(reply);
            drop(request);
            was_closed
        }
    }

    struct WorkerOwner<Request, Reply, W: FnMut()> {
        shared: Arc<Shared<Request, Reply>>,
        wake: W,
        wake_in_progress: bool,
    }

    impl<Request, Reply, W: FnMut()> WorkerOwner<Request, Reply, W> {
        fn notify(&mut self) {
            self.wake_in_progress = true;
            (self.wake)();
            self.wake_in_progress = false;
        }
    }

    impl<Request, Reply, W: FnMut()> Drop for WorkerOwner<Request, Reply, W> {
        fn drop(&mut self) {
            if !self.shared.close() && !self.wake_in_progress {
                // A panicking observer cannot turn a worker fault into a process abort.
                drop(catch_unwind(AssertUnwindSafe(|| (self.wake)())));
            }
        }
    }

    /// One in-flight request and one completed reply. The worker does not hold
    /// a game, window or GPU type in this interface.
    pub struct FrameWorker<Request, Reply> {
        shared: Arc<Shared<Request, Reply>>,
        join: Option<JoinHandle<()>>,
    }

    /// A nonblocking request was refused; the caller retains its payload.
    pub enum SendFault<Request> {
        /// The previous request has not been taken by the worker.
        Busy(Request),
        /// The worker was shut down.
        Closed(Request),
    }

    impl<Request: Send + 'static, Reply: Send + 'static> FrameWorker<Request, Reply> {
        /// Start one resident worker. `wake` signals the shell after a reply is
        /// published; it must not process the reply inside the callback.
        /// # Errors
        /// Returns an OS thread creation failure.
        pub fn start(
            name: &str,
            mut work: impl FnMut(Request) -> Reply + Send + 'static,
            wake: impl FnMut() + Send + 'static,
        ) -> std::io::Result<Self> {
            let shared = Arc::new(Shared {
                request: Slot::empty(),
                reply: Slot::empty(),
                closed: AtomicBool::new(false),
            });
            let worker = WorkerOwner {
                shared: Arc::clone(&shared),
                wake,
                wake_in_progress: false,
            };
            let join = thread::Builder::new().name(name.into()).spawn(move || {
                let mut worker = worker;
                loop {
                    let request = {
                        let mut slot = worker
                            .shared
                            .request
                            .value
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner);
                        while slot.is_none() && !worker.shared.closed.load(Ordering::Acquire) {
                            slot = worker
                                .shared
                                .request
                                .ready
                                .wait(slot)
                                .unwrap_or_else(PoisonError::into_inner);
                        }
                        if worker.shared.closed.load(Ordering::Acquire) {
                            return;
                        }
                        match slot.take() {
                            Some(request) => request,
                            None => continue,
                        }
                    };
                    let reply = work(request);
                    let mut slot = worker
                        .shared
                        .reply
                        .value
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner);
                    while slot.is_some() && !worker.shared.closed.load(Ordering::Acquire) {
                        slot = worker
                            .shared
                            .reply
                            .ready
                            .wait(slot)
                            .unwrap_or_else(PoisonError::into_inner);
                    }
                    if worker.shared.closed.load(Ordering::Acquire) {
                        return;
                    }
                    *slot = Some(reply);
                    worker.shared.reply.ready.notify_one();
                    drop(slot);
                    worker.notify();
                }
            })?;
            Ok(Self {
                shared,
                join: Some(join),
            })
        }

        /// Admit a request without waiting for the previous one. A busy caller
        /// decides whether to retain or discard an obsolete display target.
        /// # Errors
        /// Returns the payload when the slot is busy or the worker has closed.
        pub fn try_send(&self, request: Request) -> Result<(), SendFault<Request>> {
            let mut slot = self
                .shared
                .request
                .value
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if self.shared.closed.load(Ordering::Acquire) {
                return Err(SendFault::Closed(request));
            }
            if slot.is_some() {
                return Err(SendFault::Busy(request));
            }
            *slot = Some(request);
            self.shared.request.ready.notify_one();
            Ok(())
        }

        /// Take a completed reply, if any, without blocking the OS loop.
        pub fn try_recv(&self) -> Option<Reply> {
            let mut slot = self
                .shared
                .reply
                .value
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let reply = slot.take();
            if reply.is_some() {
                self.shared.reply.ready.notify_one();
            }
            reply
        }

        /// Whether the render owner has stopped or was explicitly closed.
        pub fn is_closed(&self) -> bool {
            self.shared.closed.load(Ordering::Acquire)
        }

        /// Wait for a reply during explicit shutdown or qualification only.
        pub fn recv_timeout(&self, timeout: Duration) -> Option<Reply> {
            let mut slot = self
                .shared
                .reply
                .value
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let (waited, _) = self
                .shared
                .reply
                .ready
                .wait_timeout_while(slot, timeout, |value| {
                    value.is_none() && !self.shared.closed.load(Ordering::Acquire)
                })
                .unwrap_or_else(PoisonError::into_inner);
            slot = waited;
            let reply = slot.take();
            if reply.is_some() {
                self.shared.reply.ready.notify_one();
            }
            reply
        }

        /// Close both slots and join the worker if it exits within 100 ms.
        /// A worker stuck in a platform call retains its resources until it
        /// returns. Shutdown never waits indefinitely for that call.
        pub fn shutdown(mut self) {
            self.close();
            let Some(join) = self.join.take() else {
                return;
            };
            let deadline = Instant::now() + SHUTDOWN_WAIT;
            // The join handle also covers resource and thread-local teardown.
            // Only explicit shutdown polls it, leaving the frame path unchanged.
            while !join.is_finished() {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return;
                }
                thread::sleep(remaining.min(SHUTDOWN_POLL));
            }
            drop(join.join());
        }
    }

    impl<Request, Reply> FrameWorker<Request, Reply> {
        fn close(&self) {
            self.shared.close();
        }
    }

    impl<Request, Reply> Drop for FrameWorker<Request, Reply> {
        fn drop(&mut self) {
            self.close();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{FrameWorker, SendFault};
        use crate::frame::{TargetLeadStatus, display_target_lead};
        use std::{
            sync::{
                Arc,
                atomic::{AtomicBool, AtomicU32, Ordering},
                mpsc,
            },
            thread,
            time::{Duration, Instant},
        };

        struct ExitSignal(Arc<AtomicBool>);

        #[test]
        fn delayed_worker_classifies_a_consumed_display_target_as_late() {
            let origin = Instant::now();
            let (entered, observed) = mpsc::sync_channel(1);
            let (release, paused) = mpsc::sync_channel(1);
            let worker = FrameWorker::start(
                "target-deadline-test",
                move |target_seconds| {
                    entered.send(()).expect("worker entered");
                    paused.recv().expect("worker released");
                    display_target_lead(target_seconds, origin.elapsed().as_secs_f64()).status
                },
                || {},
            )
            .expect("worker");
            worker
                .try_send(0.008)
                .unwrap_or_else(|_| panic!("target queued"));
            observed
                .recv_timeout(Duration::from_secs(1))
                .expect("entered");
            thread::sleep(Duration::from_millis(20));
            release.send(()).expect("release");
            assert_eq!(
                worker.recv_timeout(Duration::from_secs(1)),
                Some(TargetLeadStatus::Late)
            );
            worker.shutdown();
        }

        impl Drop for ExitSignal {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }

        #[test]
        fn close_wakes_an_idle_worker_even_when_start_and_shutdown_race() {
            for _ in 0..64 {
                let exited = Arc::new(AtomicBool::new(false));
                let signal = ExitSignal(Arc::clone(&exited));
                let worker = FrameWorker::start(
                    "frame-close-test",
                    move |()| {
                        let _keep_signal = &signal;
                    },
                    || {},
                )
                .expect("worker starts");
                worker.shutdown();
                assert!(
                    exited.load(Ordering::Acquire),
                    "shutdown must wait for idle worker resources to drop"
                );
            }
        }

        #[test]
        fn shutdown_does_not_wait_forever_for_an_in_flight_job() {
            let (started, entering) = mpsc::channel();
            let (release, blocked) = mpsc::channel();
            let worker = FrameWorker::start(
                "frame-stuck-test",
                move |()| {
                    started.send(()).expect("job starts");
                    blocked.recv().expect("test releases job");
                },
                || {},
            )
            .expect("worker starts");
            worker.try_send(()).unwrap_or_else(|_| panic!("job queued"));
            entering
                .recv_timeout(Duration::from_secs(1))
                .expect("worker entered");
            let (returned, done) = mpsc::channel();
            let caller = thread::spawn(move || {
                worker.shutdown();
                returned.send(()).expect("test receives shutdown result");
            });
            let detached = done.recv_timeout(Duration::from_secs(1));
            release.send(()).expect("release test job");
            caller.join().expect("shutdown caller exits");
            detached.expect("shutdown returns before the job releases");
        }

        #[test]
        fn busy_slot_retains_request_and_reply_order() {
            let (started, entering) = mpsc::channel();
            let (resume, paused) = mpsc::channel();
            let worker = FrameWorker::start(
                "frame-test",
                move |value| {
                    started.send(()).expect("test signal");
                    paused.recv().expect("test release");
                    value + 1
                },
                || {},
            )
            .expect("worker");
            worker
                .try_send(4)
                .unwrap_or_else(|_| panic!("first request"));
            entering.recv().expect("worker entered");
            worker
                .try_send(7)
                .unwrap_or_else(|_| panic!("second request"));
            assert!(matches!(worker.try_send(9), Err(SendFault::Busy(9))));
            resume.send(()).expect("resume first");
            assert_eq!(worker.recv_timeout(Duration::from_secs(1)), Some(5));
            entering.recv().expect("worker entered again");
            resume.send(()).expect("resume second");
            assert_eq!(worker.recv_timeout(Duration::from_secs(1)), Some(8));
            worker.shutdown();
        }

        #[test]
        fn panicking_job_closes_worker_and_wakes_the_window() {
            let (wake, observed) = mpsc::sync_channel(1);
            let worker = FrameWorker::start(
                "frame-panic-test",
                |()| -> () { panic!("render job failed") },
                move || {
                    let _ = wake.try_send(());
                },
            )
            .expect("worker");
            worker
                .try_send(())
                .unwrap_or_else(|_| panic!("first request"));
            observed
                .recv_timeout(Duration::from_secs(1))
                .expect("terminal wake");
            assert!(worker.is_closed());
            assert!(matches!(worker.try_send(()), Err(SendFault::Closed(()))));
            assert!(worker.recv_timeout(Duration::from_millis(10)).is_none());
            worker.shutdown();
        }

        #[test]
        fn panicking_wake_does_not_retry_or_leave_a_live_worker() {
            let (entered, observed) = mpsc::sync_channel(1);
            let calls = Arc::new(AtomicU32::new(0));
            let count = Arc::clone(&calls);
            let worker = FrameWorker::start(
                "frame-wake-panic-test",
                |value| value + 1,
                move || {
                    count.fetch_add(1, Ordering::AcqRel);
                    let _ = entered.try_send(());
                    panic!("observer failed");
                },
            )
            .expect("worker");
            worker
                .try_send(1)
                .unwrap_or_else(|_| panic!("first request"));
            observed
                .recv_timeout(Duration::from_secs(1))
                .expect("callback entered");
            let deadline = Instant::now() + Duration::from_secs(1);
            while !worker.is_closed() {
                assert!(
                    Instant::now() < deadline,
                    "worker did not report callback failure"
                );
                thread::yield_now();
            }
            assert_eq!(calls.load(Ordering::Acquire), 1);
            assert_eq!(
                worker.try_recv(),
                Some(2),
                "published reply remains consumable"
            );
            assert!(matches!(worker.try_send(3), Err(SendFault::Closed(3))));
            worker.shutdown();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::{FrameWorker, SendFault};

#[cfg(test)]
mod target_tests {
    use super::{TargetLeadStatus, display_target_lead};
    use std::time::Duration;

    #[test]
    fn worker_consumption_uses_current_target_slack() {
        let target = 100.008;
        let queued = display_target_lead(target, 100.0);
        assert_eq!(queued.status, TargetLeadStatus::Future);
        assert!((queued.duration.as_secs_f64() - 0.008).abs() < 1e-9);
        let consumed = display_target_lead(target, 100.012);
        assert_eq!(consumed.status, TargetLeadStatus::Late);
        assert_eq!(consumed.duration, Duration::ZERO);

        let future = display_target_lead(100.020, 100.012);
        assert_eq!(future.status, TargetLeadStatus::Future);
        assert!((100.012 + future.duration.as_secs_f64() - 100.020).abs() < 1e-9);
    }

    #[test]
    fn bad_or_far_targets_degrade_without_panicking() {
        for (target, now) in [
            (f64::NAN, 1.0),
            (1.0, f64::INFINITY),
            (-1.0, 1.0),
            (1.0, -1.0),
        ] {
            assert_eq!(
                display_target_lead(target, now).status,
                TargetLeadStatus::Invalid
            );
        }
        assert_eq!(display_target_lead(1.0, 1.0).status, TargetLeadStatus::Late);
        let capped = display_target_lead(f64::MAX, 0.0);
        assert_eq!(capped.status, TargetLeadStatus::Clamped);
        assert_eq!(capped.duration, Duration::from_millis(50));
        assert_eq!(
            display_target_lead(0.05, 0.0).status,
            TargetLeadStatus::Future
        );
        assert_eq!(
            display_target_lead(1.051, 1.0).status,
            TargetLeadStatus::Clamped
        );
    }
}
