//! Browser transport: the global `fetch` (page or worker), body copied once into aligned bytes.

use std::future::Future;

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use super::{Fetch, FetchReply};
use crate::{AssetError, ProductBytes};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = fetch)]
    fn global_fetch(url: &str) -> js_sys::Promise;

    #[wasm_bindgen(js_name = Response)]
    type FetchResponse;

    #[wasm_bindgen(method, getter)]
    fn status(this: &FetchResponse) -> u16;

    #[wasm_bindgen(method, js_name = arrayBuffer)]
    fn array_buffer(this: &FetchResponse) -> js_sys::Promise;
}

/// [`Fetch`] over the JavaScript global `fetch`, available in pages, workers and Node.
#[derive(Clone, Copy, Debug, Default)]
pub struct BrowserFetch;

fn transport(error: &JsValue) -> AssetError {
    AssetError::Transport(error.as_string().unwrap_or_else(|| format!("{error:?}")))
}

impl Fetch for BrowserFetch {
    fn get(&self, url: &str) -> impl Future<Output = Result<FetchReply, AssetError>> + use<> {
        // `fetch` starts the request now; the future only waits for it.
        let started = JsFuture::from(global_fetch(url));
        async move {
            let response: FetchResponse = started
                .await
                .map_err(|error| transport(&error))?
                .unchecked_into();
            let status = response.status();
            let buffer = JsFuture::from(response.array_buffer())
                .await
                .map_err(|error| transport(&error))?;
            let view = js_sys::Uint8Array::new(&buffer);
            let body = ProductBytes::fill(view.length() as usize, |bytes| view.copy_to(bytes));
            Ok(FetchReply { status, body })
        }
    }
}
