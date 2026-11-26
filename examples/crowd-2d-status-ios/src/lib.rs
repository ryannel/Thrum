//! Application entry point for the Crowd status display.

#[cfg(target_os = "ios")]
#[expect(
    unsafe_code,
    reason = "the app's Swift bootstrap lends its store path through C"
)]
#[unsafe(no_mangle)]
extern "C" fn thrum_mobile_start(store: *const std::ffi::c_char, fixed_steps: i32, witness: i32) {
    use std::{ffi::CStr, path::PathBuf};

    use thrum_app::ios::{MobileOptions, run};

    if store.is_null() {
        return;
    }
    // SAFETY: the Swift entry passes a NUL-terminated path within withCString.
    let Ok(store) = unsafe { CStr::from_ptr(store) }.to_str() else {
        return;
    };
    let game = crowd_2d_status::MobileCrowd::new(PathBuf::from(store));
    let game = if fixed_steps == 0 {
        game
    } else {
        game.with_fixed_steps()
    };
    if let Err(error) = run(
        game,
        MobileOptions {
            title: "Crowd",
            render_thread: "thrum-crowd-ios-render",
            witness: (witness != 0).then_some("thrum-crowd-ios-state.txt"),
        },
    ) {
        #[expect(clippy::print_stderr, reason = "report a fatal mobile launch error")]
        {
            eprintln!("{error}");
        }
    }
}
