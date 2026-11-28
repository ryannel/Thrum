//! Bounded, synchronous dispatch for borrowed query rows. Persistent worker
//! mailboxes avoid allocating queue blocks while submitting warmed query work.

use std::{
    any::Any,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    ptr::NonNull,
    sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError},
    thread,
};

const MAX_WORKERS: usize = 7;
const MIN_ROWS_PER_CHUNK: usize = 64;
type Panic = Box<dyn Any + Send>;

struct Job {
    data: NonNull<()>,
    run: fn(NonNull<()>),
}

// SAFETY: Job::borrow only erases a Send task. Pool::run exclusively lends
// each task to one worker and waits for completion before dropping its stack.
#[expect(unsafe_code, reason = "transfer a scoped Send task")]
unsafe impl Send for Job {}

impl Job {
    // SAFETY: the task must stay in place and unborrowed until its worker
    // acknowledges completion. Only the worker may dereference the job.
    #[expect(unsafe_code, reason = "lend a query chunk to a persistent worker")]
    unsafe fn borrow<T: Send, F: Fn(&mut T) + Sync>(task: &mut (&mut [T], &F)) -> Self {
        fn run<T, F: Fn(&mut T)>(data: NonNull<()>) {
            // SAFETY: Job::borrow erased this exact stack task, and the
            // caller waits for the worker before reusing or dropping it.
            let (slice, run) = unsafe { data.cast::<(&mut [T], &F)>().as_mut() };
            slice.iter_mut().for_each(*run);
        }
        Self {
            data: NonNull::from(task).cast(),
            run: run::<T, F>,
        }
    }
}

#[derive(Default)]
struct Mailbox {
    state: Mutex<WorkerState>,
    ready: Condvar,
}

#[derive(Default)]
struct WorkerState {
    pending: Option<Job>,
    done: bool,
    panic: Option<Panic>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Mailbox {
    fn worker(&self) -> ! {
        loop {
            let mut state = self
                .ready
                .wait_while(lock(&self.state), |state| state.pending.is_none())
                .unwrap_or_else(PoisonError::into_inner);
            let job = state.pending.take().expect("notified job");
            drop(state);
            let panic = catch_unwind(AssertUnwindSafe(|| (job.run)(job.data))).err();
            let mut state = lock(&self.state);
            state.panic = panic;
            state.done = true;
            self.ready.notify_all();
        }
    }

    fn submit(&self, job: Job) {
        let mut state = lock(&self.state);
        state.pending = Some(job);
        state.done = false;
        self.ready.notify_all();
    }

    fn finish(&self) -> Option<Panic> {
        let mut state = self
            .ready
            .wait_while(lock(&self.state), |state| !state.done)
            .unwrap_or_else(PoisonError::into_inner);
        state.panic.take()
    }
}

struct Pool {
    workers: Vec<Arc<Mailbox>>,
    busy: Mutex<()>,
}

impl Pool {
    fn new() -> Self {
        let count = thread::available_parallelism().map_or(0, |count| count.get() - 1);
        let workers = (0..count.min(MAX_WORKERS))
            .filter_map(|index| {
                let mailbox = Arc::new(Mailbox::default());
                let worker = Arc::clone(&mailbox);
                thread::Builder::new()
                    .name(format!("thrum query {index}"))
                    .spawn(move || worker.worker())
                    .ok()?;
                Some(mailbox)
            })
            .collect();
        Self {
            workers,
            busy: Mutex::new(()),
        }
    }

    fn run<T: Send, F: Fn(&mut T) + Sync>(&self, jobs: &mut [T], run: &F) {
        // Contended or nested calls run on their caller, so there is no
        // deadlock when a scope's callback itself submits another scope.
        let Ok(busy) = self.busy.try_lock() else {
            jobs.iter_mut().for_each(run);
            return;
        };
        let chunks = (jobs.len() / MIN_ROWS_PER_CHUNK).min(self.workers.len() + 1);
        if chunks < 2 {
            jobs.iter_mut().for_each(run);
            return;
        }
        let mut slices = jobs.chunks_mut(jobs.len().div_ceil(chunks));
        let local = slices.next().expect("nonempty query");
        let mut tasks: [Option<_>; MAX_WORKERS] =
            std::array::from_fn(|_| slices.next().map(|slice| (slice, run)));
        let mut submitted = 0;
        for (mailbox, task) in self.workers.iter().zip(tasks.iter_mut().flatten()) {
            // SAFETY: tasks stay on this stack, one per worker, until
            // every submitted task is acknowledged below.
            #[expect(unsafe_code, reason = "one exclusive loan per awaited worker")]
            mailbox.submit(unsafe { Job::borrow(task) });
            submitted += 1;
        }

        // Catch even the caller's panic before waiting. Collect all worker
        // outcomes before any panic payload can unwind or be dropped.
        let local_panic = catch_unwind(AssertUnwindSafe(|| local.iter_mut().for_each(run))).err();
        let mut workers = self.workers[..submitted].iter();
        let panics: [Option<Panic>; MAX_WORKERS] =
            std::array::from_fn(|_| workers.next().and_then(|worker| worker.finish()));
        drop(busy);
        if let Some(panic) = local_panic.or_else(|| panics.into_iter().flatten().next()) {
            resume_unwind(panic);
        }
    }
}

pub(super) fn run<T: Send>(jobs: &mut [T], run: impl Fn(&mut T) + Send + Sync) {
    if jobs.len() < 2 * MIN_ROWS_PER_CHUNK {
        jobs.iter_mut().for_each(run);
    } else {
        static POOL: OnceLock<Pool> = OnceLock::new();
        POOL.get_or_init(Pool::new).run(jobs, &run);
    }
}
