//! The Wasm dev-server client's end-to-end check in a browser page, which sends an `Origin` and
//! so exercises the server's loopback CORS path that Node never reaches.
//!
//! Driven by `scripts/check_g5_browser_dev_client.sh`: `wasm-bindgen-test-runner` serves the page
//! interactively (`NO_HEADLESS=1`) at a loopback origin, a browser opens it, and the driver edits
//! one source after the page logs `E2E_READY`. The test body is the Node check's.

#![cfg(target_arch = "wasm32")]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[path = "dev_server_wasm.rs"]
mod page;
