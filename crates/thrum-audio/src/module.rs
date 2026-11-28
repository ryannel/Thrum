//! The import-free WebAssembly entry points. Samples are interleaved in module memory.
#![expect(
    unsafe_code,
    reason = "the documented WebAssembly exports have fixed symbol names"
)]

use std::cell::RefCell;

use crate::diagnostics::{FullPool, ReferenceRun};

struct Module {
    reference: Option<ReferenceRun>,
    full: Option<FullPool>,
    output: [f32; 8192],
}

thread_local! {
    static MODULE: RefCell<Module> = const { RefCell::new(Module {
        reference: None, full: None, output: [0.0; 8192],
    }) };
}

// SAFETY: Each export has an audio-specific name unique to this module.
#[unsafe(no_mangle)]
extern "C" fn audio_reference_prepare() {
    MODULE.with_borrow_mut(|m| m.reference = Some(ReferenceRun::default()));
}

// SAFETY: Each export has an audio-specific name unique to this module.
#[unsafe(no_mangle)]
extern "C" fn audio_reference_pull(requested: usize) -> usize {
    MODULE.with_borrow_mut(|m| {
        let samples = requested.saturating_mul(2).min(m.output.len());
        m.reference
            .as_mut()
            .map_or(0, |reference| reference.pull(&mut m.output[..samples]))
    })
}

// SAFETY: Each export has an audio-specific name unique to this module.
#[unsafe(no_mangle)]
extern "C" fn audio_output_samples() -> *const f32 {
    MODULE.with_borrow(|m| m.output.as_ptr())
}

// SAFETY: Each export has an audio-specific name unique to this module.
#[unsafe(no_mangle)]
extern "C" fn audio_cost_prepare() {
    MODULE.with_borrow_mut(|m| m.full = Some(FullPool::new().expect("valid cost scene")));
}

// SAFETY: Each export has an audio-specific name unique to this module.
#[unsafe(no_mangle)]
extern "C" fn audio_cost_render() -> u32 {
    MODULE.with_borrow_mut(|m| m.full.as_mut().expect("cost scene was prepared").render())
}
