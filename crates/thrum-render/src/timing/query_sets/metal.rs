use std::sync::{Mutex, MutexGuard, PoisonError};

use objc2::rc::autoreleasepool;
use objc2_metal::{MTLCounterSampleBufferDescriptor, MTLCounterSet, MTLDevice, MTLStorageMode};

use super::TimingError;

static ADMISSION: Mutex<()> = Mutex::new(());

#[expect(
    unsafe_code,
    reason = "Trial allocation on a retained Metal device using the same valid sample counts as wgpu"
)]
pub(super) fn admit<const N: usize>(
    device: &wgpu::Device,
    counts: &[u32; N],
) -> Result<Option<MutexGuard<'static, ()>>, TimingError> {
    // SAFETY: The guard keeps the device live for the trial. We only allocate
    // independent temporary buffers, with no changes to wgpu's resources.
    let Some(native) = (unsafe { device.as_hal::<wgpu::hal::api::Metal>() }) else {
        return Ok(None);
    };
    // Avoid holding a wgpu HAL guard while waiting on another requester or
    // when making the subsequent wgpu calls.
    let raw = native.raw_device().clone();
    drop(native);

    #[cfg(feature = "creation-counters")]
    notify(TimestampQueryRequestStage::Waiting);
    let admission = ADMISSION.lock().unwrap_or_else(PoisonError::into_inner);
    autoreleasepool(|_| {
        let sets = raw
            .counterSets()
            .ok_or(TimingError::TimestampQueriesUnavailable)?;
        let timestamp = sets
            .iter()
            .find(|set| &*set.name() == objc2_foundation::ns_string!("timestamp"))
            .ok_or(TimingError::TimestampQueriesUnavailable)?;
        let descriptor = MTLCounterSampleBufferDescriptor::new();
        descriptor.setCounterSet(Some(&timestamp));
        descriptor.setStorageMode(MTLStorageMode::Shared);
        let mut trial = [const { None }; N];
        for (place, &count) in trial.iter_mut().zip(counts) {
            // SAFETY: Admission has already checked the count against the
            // same limits used for the subsequent wgpu timestamp query set.
            unsafe { descriptor.setSampleCount(count as usize) };
            let buffer = raw
                .newCounterSampleBufferWithDescriptor_error(&descriptor)
                .map_err(|_| TimingError::TimestampQuerySetsExhausted)?;
            *place = Some(buffer);
        }
        Ok(())
    })?;
    #[cfg(feature = "creation-counters")]
    notify(TimestampQueryRequestStage::Admitted);
    Ok(Some(admission))
}

#[cfg(feature = "creation-counters")]
/// Stages exposed to creation-counter tests for coordinating two Metal requests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimestampQueryRequestStage {
    /// The request has selected Metal and is about to take the process lock.
    Waiting,
    /// The trial buffers were released and the process lock is still held.
    Admitted,
}

#[cfg(feature = "creation-counters")]
type Probe = Box<dyn Fn(TimestampQueryRequestStage)>;

#[cfg(feature = "creation-counters")]
std::thread_local! {
    static PROBE: std::cell::RefCell<Option<Probe>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(feature = "creation-counters")]
fn notify(stage: TimestampQueryRequestStage) {
    PROBE.with_borrow(|probe| {
        if let Some(probe) = probe {
            probe(stage);
        }
    });
}

#[cfg(feature = "creation-counters")]
/// Observe Metal timestamp admission on the current thread during a test operation.
pub fn with_timestamp_query_request_probe<T>(
    probe: impl Fn(TimestampQueryRequestStage) + 'static,
    operation: impl FnOnce() -> T,
) -> T {
    struct Restore(Option<Probe>);
    impl Drop for Restore {
        fn drop(&mut self) {
            PROBE.replace(self.0.take());
        }
    }
    let _restore = Restore(PROBE.replace(Some(Box::new(probe))));
    operation()
}
