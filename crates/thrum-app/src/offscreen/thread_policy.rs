//! Explicit scheduling class for an unattended render owner.
#[expect(
    unsafe_code,
    reason = "Darwin thread-local QoS ABI from sys/qos.h and pthread/qos.h; only a local size_t output pointer, no ownership transfer"
)]
unsafe extern "C" {
    fn qos_class_self() -> u32;
    fn pthread_cpu_number_np(number: *mut usize) -> i32;
    fn pthread_set_qos_class_self_np(class: u32, relative_priority: i32) -> i32;
}

/// Named scheduling policy; inherited mode is retained for controlled diagnosis.
#[derive(Clone, Copy, Debug)]
pub enum RenderThreadPolicy {
    /// Leave the caller's inherited class unchanged.
    Inherited,
    /// Treat render opportunities as latency-sensitive interactive work.
    Interactive,
}

/// The Darwin requested QoS class before and after applying a policy.
#[derive(Debug)]
pub struct ThreadPolicyReport {
    /// Inherited requested class (0x15 is default; 0x21 interactive).
    pub before: u32,
    /// Declared class used for every measured opportunity.
    pub after: u32,
}

/// Configure on the render-owner thread before preparation/warmup. Both ON and
/// OFF use the same class; capture I/O is never used to boost the baseline.
/// # Errors
/// Refuses a QoS request the operating system rejects.
#[expect(
    unsafe_code,
    reason = "read/set only the calling thread QoS via documented Darwin value-only functions"
)]
pub fn configure(policy: RenderThreadPolicy) -> std::io::Result<ThreadPolicyReport> {
    // SAFETY: Value-only getter for the calling thread; available on this macOS target.
    let before = unsafe { qos_class_self() };
    if matches!(policy, RenderThreadPolicy::Interactive) {
        // SAFETY: 0x21 is QOS_CLASS_USER_INTERACTIVE, priority 0 is in the documented range.
        let error = unsafe { pthread_set_qos_class_self_np(0x21, 0) };
        if error != 0 {
            return Err(std::io::Error::from_raw_os_error(error));
        }
    }
    // SAFETY: Same thread-local value-only getter, no borrowed state.
    let after = unsafe { qos_class_self() };
    Ok(ThreadPolicyReport { before, after })
}

#[expect(
    unsafe_code,
    reason = "Darwin writes only the valid local size_t output; CPU query is diagnostic, never an affinity or synchronization guarantee"
)]
fn current_cpu() -> Option<usize> {
    let mut number = 0;
    // SAFETY: The SDK signature takes a live writable size_t pointer; it cannot escape this call.
    let error = unsafe { pthread_cpu_number_np(&raw mut number) };
    (error == 0).then_some(number)
}

/// CPU consumption and wall time for one calling-thread region.
pub struct WorkCost {
    /// Time scheduled on a CPU, excluding sleeps/preemption/driver waits.
    pub cpu: std::time::Duration,
    /// CPU observed at region start, without an affinity guarantee.
    pub cpu_start: Option<usize>,
    /// CPU observed at region end; a difference exposes migration.
    pub cpu_end: Option<usize>,
    /// Elapsed monotonic wall time including waits.
    pub wall: std::time::Duration,
}
/// Paired thread-CPU and wall clocks, independent of the world clock.
pub struct WorkClock {
    cpu_start: Option<usize>,
    cpu: cpu_time::ThreadTime,
    wall: std::time::Instant,
}
impl WorkClock {
    /// Begin a diagnostic region.
    /// # Errors
    /// Refuses an unavailable thread CPU clock.
    pub fn start() -> std::io::Result<Self> {
        Ok(Self {
            cpu_start: current_cpu(),
            cpu: cpu_time::ThreadTime::try_now()?,
            wall: std::time::Instant::now(),
        })
    }
    /// Finish on the same thread.
    /// # Errors
    /// Refuses an unavailable thread CPU clock.
    pub fn finish(self) -> std::io::Result<WorkCost> {
        Ok(WorkCost {
            cpu: self.cpu.try_elapsed()?,
            cpu_start: self.cpu_start,
            cpu_end: current_cpu(),
            wall: self.wall.elapsed(),
        })
    }
}
