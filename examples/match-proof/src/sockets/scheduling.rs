//! Darwin scheduling for the proof's latency-sensitive simulation threads.

use std::io;

#[cfg(target_vendor = "apple")]
#[expect(unsafe_code, reason = "Darwin SDK's value-only thread-local QoS ABI")]
unsafe extern "C" {
    fn qos_class_self() -> u32;
    fn pthread_set_qos_class_self_np(class: u32, relative_priority: i32) -> i32;
}

#[cfg(target_vendor = "apple")]
#[expect(
    unsafe_code,
    reason = "apply and observe QoS only on the calling proof thread"
)]
pub(super) fn interactive() -> io::Result<Option<[u32; 2]>> {
    const USER_INTERACTIVE: u32 = 0x21;
    // SAFETY: The getter obtains the calling thread's QoS with no borrowed state.
    let before = unsafe { qos_class_self() };
    // SAFETY: The class and relative priority are documented Darwin SDK values.
    let result = unsafe { pthread_set_qos_class_self_np(USER_INTERACTIVE, 0) };
    if result != 0 {
        return Err(io::Error::from_raw_os_error(result));
    }
    // SAFETY: Same thread-local getter as before.
    let after = unsafe { qos_class_self() };
    Ok(Some([before, after]))
}

#[cfg(not(target_vendor = "apple"))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "preserve the same scheduling interface for the portable native build"
)]
pub(super) fn interactive() -> io::Result<Option<[u32; 2]>> {
    Ok(None)
}
