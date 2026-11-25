//! Admission for every timestamp query set created by the frame runtime.

use super::TimingError;

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod metal;
#[cfg(all(
    feature = "creation-counters",
    any(target_os = "macos", target_os = "ios")
))]
pub use metal::{TimestampQueryRequestStage, with_timestamp_query_request_probe};

/// Allocate a timestamp query set on a device with timestamp queries enabled.
/// Use this for all timestamp allocations that share a process with the frame
/// runtime. On Metal it serializes admission and creation with other callers.
///
/// # Errors
///
/// Returns an error for an invalid count, a device without timestamp queries,
/// or insufficient Metal counter sample buffers in the process.
pub fn create_timestamp_query_set(
    device: &wgpu::Device,
    label: &str,
    count: u32,
) -> Result<wgpu::QuerySet, TimingError> {
    let [query] = allocate(device, label, [count])?;
    Ok(query)
}

pub(super) fn allocate<const N: usize>(
    device: &wgpu::Device,
    label: &str,
    counts: [u32; N],
) -> Result<[wgpu::QuerySet; N], TimingError> {
    if counts
        .iter()
        .any(|count| !(1..=wgpu::QUERY_SET_MAX_QUERIES).contains(count))
    {
        return Err(TimingError::InvalidCapacity);
    }
    if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
        return Err(TimingError::TimestampQueriesUnavailable);
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    let _admission = metal::admit(device, &counts)?;

    Ok(counts.map(|count| {
        device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some(label),
            ty: wgpu::QueryType::Timestamp,
            count,
        })
    }))
}
