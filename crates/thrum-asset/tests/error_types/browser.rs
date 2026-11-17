//! Error-source checks for the actual browser fetch adapter, executable in Node on the Mac.

use std::error::Error;
use thrum_asset::{BrowserFetch, BrowserTransportError, Fetch};
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen(inline_js = "
export function install_rejection(cause, body) {
    const previous = globalThis.fetch;
    globalThis.fetch = () => body
        ? Promise.resolve({status: 200, arrayBuffer: () => Promise.reject(cause)})
        : Promise.reject(cause);
    return previous;
}
export function restore_fetch(previous) { globalThis.fetch = previous; }
")]
extern "C" {
    fn install_rejection(cause: &JsValue, body: bool) -> JsValue;
    fn restore_fetch(previous: JsValue);
}

#[wasm_bindgen_test]
async fn fetch_and_body_failures_preserve_the_original_javascript_value() {
    for body in [false, true] {
        let cause: JsValue = js_sys::Object::new().into();
        let previous = install_rejection(&cause, body);
        let result = BrowserFetch.get("https://source-test.invalid/asset").await;
        restore_fetch(previous);
        let Err(error) = result else {
            panic!("injected promise rejection must fail");
        };
        let source = error
            .source()
            .expect("browser rejection source")
            .downcast_ref::<BrowserTransportError>()
            .expect("typed browser adapter");
        let BrowserTransportError::Rejected(original) = source;
        assert_eq!(
            original, &cause,
            "the original JavaScript object survives both rejection paths"
        );
    }
}
