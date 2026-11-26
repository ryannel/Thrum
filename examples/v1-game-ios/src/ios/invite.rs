//! V1 invite payload handed off by the app's URL handler.

use std::{
    ffi::{CStr, c_char},
    path::PathBuf,
};

use thrum_app::ios::{SceneRequest, request_scene};

#[expect(
    unsafe_code,
    reason = "Swift lends the UTF-8 payload and font store path for this call"
)]
#[unsafe(no_mangle)]
extern "C" fn thrum_v1_open_invite(bytes: *const u8, len: usize, font_store: *const c_char) {
    if bytes.is_null() || len == 0 || font_store.is_null() {
        return;
    }
    // SAFETY: The app's Swift handler keeps both buffers live during this synchronous call.
    let (payload, content) = unsafe {
        (
            std::str::from_utf8(std::slice::from_raw_parts(bytes, len)),
            CStr::from_ptr(font_store).to_str(),
        )
    };
    if let (Ok(payload), Ok(content)) = (payload, content) {
        request_scene(SceneRequest {
            payload: payload.to_owned(),
            content: PathBuf::from(content),
        });
    }
}
