//! Bounded native frame handoff. The shell keeps OS events on its thread while
//! a worker owns the frame operation; neither request nor reply grows a queue.

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        sync::{
            Arc, Condvar, Mutex, PoisonError,
            atomic::{AtomicBool, Ordering},
        },
        thread::{self, JoinHandle},
        time::Duration,
    };

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
            mut wake: impl FnMut() + Send + 'static,
        ) -> std::io::Result<Self> {
            let shared = Arc::new(Shared {
                request: Slot::empty(),
                reply: Slot::empty(),
                closed: AtomicBool::new(false),
            });
            let worker = Arc::clone(&shared);
            let join = thread::Builder::new().name(name.into()).spawn(move || {
                loop {
                    let request = {
                        let mut slot = worker
                            .request
                            .value
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner);
                        while slot.is_none() && !worker.closed.load(Ordering::Acquire) {
                            slot = worker
                                .request
                                .ready
                                .wait(slot)
                                .unwrap_or_else(PoisonError::into_inner);
                        }
                        if worker.closed.load(Ordering::Acquire) {
                            return;
                        }
                        match slot.take() {
                            Some(request) => request,
                            None => continue,
                        }
                    };
                    let reply = work(request);
                    let mut slot = worker
                        .reply
                        .value
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner);
                    while slot.is_some() && !worker.closed.load(Ordering::Acquire) {
                        slot = worker
                            .reply
                            .ready
                            .wait(slot)
                            .unwrap_or_else(PoisonError::into_inner);
                    }
                    if worker.closed.load(Ordering::Acquire) {
                        return;
                    }
                    *slot = Some(reply);
                    worker.reply.ready.notify_one();
                    drop(slot);
                    wake();
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

        /// Close both slots. A worker stuck in a platform call is detached so
        /// OS teardown cannot block forever; its owned resources live until it
        /// returns.
        pub fn shutdown(mut self) {
            self.close();
            if let Some(join) = self.join.take() {
                if join.is_finished() {
                    drop(join.join());
                } else {
                    drop(join);
                }
            }
        }
    }

    impl<Request, Reply> FrameWorker<Request, Reply> {
        fn close(&self) {
            // A notification must be paired with the mutex that protects the
            // wait predicate. Otherwise close can race between a worker's
            // predicate check and its actual wait.
            let request = self
                .shared
                .request
                .value
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let reply = self
                .shared
                .reply
                .value
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            self.shared.closed.store(true, Ordering::Release);
            self.shared.request.ready.notify_all();
            self.shared.reply.ready.notify_all();
            drop(reply);
            drop(request);
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
        use std::{
            sync::{
                Arc,
                atomic::{AtomicBool, Ordering},
                mpsc,
            },
            thread,
            time::{Duration, Instant},
        };

        struct ExitSignal(Arc<AtomicBool>);

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
                let deadline = Instant::now() + Duration::from_secs(1);
                while !exited.load(Ordering::Acquire) && Instant::now() < deadline {
                    thread::yield_now();
                }
                assert!(exited.load(Ordering::Acquire), "closed worker must exit");
            }
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
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::{FrameWorker, SendFault};
