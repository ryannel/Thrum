//! Correlate private qualification boundaries to Instruments in this process.

#[expect(
    unsafe_code,
    reason = "the private C bridge takes one u32 and uses SDK signpost macros with literal names"
)]
unsafe extern "C" {
    fn thrum_mac_qualification_signpost(stage: u32);
}

#[expect(
    unsafe_code,
    reason = "the linked bridge consumes a scalar and retains no Rust pointers or callbacks"
)]
pub(super) fn mark(stage: u32) {
    // SAFETY: the C function accepts the same 32-bit unsigned scalar. Its SDK
    // log is initialized once. Unknown stages are harmless and no memory crosses.
    unsafe { thrum_mac_qualification_signpost(stage) }
}
