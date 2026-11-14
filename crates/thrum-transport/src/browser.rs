//! Browser WebTransport, with mandatory certificate hash pinning and bounded game queues.
use crate::{CloseReason, HELLO, Lane, Link, LinkError, Profile, RELIABLE_LIMIT, Ticket};
use js_sys::{Object, Promise, Reflect, Uint8Array};
use wasm_bindgen::{JsCast, prelude::*};
use wasm_bindgen_futures::{JsFuture, spawn_local};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = WebTransport)]
    type Transport;
    #[wasm_bindgen(constructor, js_class = WebTransport, catch)]
    fn new(url: &str, options: &JsValue) -> Result<Transport, JsValue>;
    #[wasm_bindgen(method, getter)]
    fn ready(this: &Transport) -> Promise;
    #[wasm_bindgen(method, getter)]
    fn closed(this: &Transport) -> Promise;
    #[wasm_bindgen(method, getter)]
    fn datagrams(this: &Transport) -> JsValue;
    #[wasm_bindgen(method, js_name = createBidirectionalStream)]
    fn create_stream(this: &Transport) -> Promise;
    #[wasm_bindgen(method, js_name = close)]
    fn close_transport(this: &Transport, info: &JsValue);
    #[wasm_bindgen(js_name = WritableStream)]
    type Writable;
    #[wasm_bindgen(method, js_name = getWriter)]
    fn writer(this: &Writable) -> Writer;
    #[wasm_bindgen(js_name = WritableStreamDefaultWriter)]
    type Writer;
    #[wasm_bindgen(method, getter, js_name = ready)]
    fn writer_ready(this: &Writer) -> Promise;
    #[wasm_bindgen(method)]
    fn write(this: &Writer, bytes: &JsValue) -> Promise;
    #[wasm_bindgen(js_name = ReadableStream)]
    type Readable;
    #[wasm_bindgen(method, js_name = getReader)]
    fn reader(this: &Readable) -> Reader;
    #[wasm_bindgen(js_name = ReadableStreamDefaultReader)]
    type Reader;
    #[wasm_bindgen(method)]
    fn read(this: &Reader) -> Promise;
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn performance_now() -> f64;
    #[wasm_bindgen(js_name = setTimeout)]
    fn set_timeout(callback: &js_sys::Function, ms: i32);
}
fn field(value: &JsValue, name: &str) -> Result<JsValue, JsValue> {
    Reflect::get(value, &name.into())
}
fn set(value: &JsValue, name: &str, data: &JsValue) -> Result<(), JsValue> {
    Reflect::set(value, &name.into(), data).map(|_| ())
}
/// Browser monotonic milliseconds. Pass this to sends and counter snapshots.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "performance.now is nonnegative milliseconds within u64 range"
)]
pub fn now() -> u64 {
    performance_now() as u64
}
async fn pause() {
    let promise = Promise::new(&mut |resolve, _| {
        set_timeout(&resolve, 1);
    });
    drop(JsFuture::from(promise).await);
}
/// Open a pinned WebTransport link. Startup and all I/O are detached from the game.
/// # Errors
/// Returns an invalid profile or local browser constructor failure; network errors are events.
/// # Panics
/// Detached tasks panic if link state has been poisoned by an earlier panic.
pub fn connect(ticket: &Ticket, profile: Profile) -> Result<Link, LinkError> {
    profile.validate()?;
    let options = Object::new();
    let hash = Object::new();
    let hashes = js_sys::Array::new();
    let configure = || -> Result<Transport, JsValue> {
        set(&hash, "algorithm", &"sha-256".into())?;
        set(
            &hash,
            "value",
            &Uint8Array::from(ticket.hash.as_slice()).into(),
        )?;
        hashes.push(&hash);
        set(&options, "serverCertificateHashes", &hashes)?;
        set(&options, "requireUnreliable", &true.into())?;
        set(&options, "allowPooling", &false.into())?;
        set(&options, "congestionControl", &"low-latency".into())?;
        Transport::new(&format!("https://{}/thrum/1", ticket.address), &options)
    };
    let transport = configure().map_err(|_| LinkError::StartFailed)?;
    let startup_ms = profile.startup_ms;
    let link = Link::new(profile);
    let io = link.clone();
    let transport = std::rc::Rc::new(transport);
    let control = std::rc::Rc::clone(&transport);
    let timer = link.clone();
    let started = now();
    spawn_local(async move {
        loop {
            {
                let mut state = timer.state.lock().expect("link state poisoned");
                state.expire(now());
                if !state.open
                    && state.closed.is_none()
                    && now().saturating_sub(started) >= startup_ms
                {
                    state.close(CloseReason::TimedOut);
                }
                if let Some(reason) = state.closed.clone() {
                    let info = Object::new();
                    let code = if let CloseReason::Local(code) = reason {
                        code
                    } else {
                        0
                    };
                    drop(set(&info, "closeCode", &code.into()));
                    control.close_transport(&info);
                    break;
                }
            }
            pause().await;
        }
    });
    let closed = link.clone();
    let closing = std::rc::Rc::clone(&transport);
    spawn_local(async move {
        let reason = match JsFuture::from(closing.closed()).await {
            Ok(info) => CloseReason::Peer(
                field(&info, "closeCode")
                    .ok()
                    .and_then(|v| v.as_f64())
                    .map_or(0, close_code),
            ),
            Err(_) => CloseReason::Transport,
        };
        closed
            .state
            .lock()
            .expect("link state poisoned")
            .close(reason);
    });
    spawn_local(async move {
        if setup(io.clone(), &transport).await.is_err() {
            io.state
                .lock()
                .expect("link state poisoned")
                .close(CloseReason::Refused);
        }
    });
    Ok(link)
}
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "WebTransport closeCode is a WebIDL unsigned long"
)]
fn close_code(value: f64) -> u32 {
    value as u32
}
async fn read_chunk(reader: &Reader) -> Result<Vec<u8>, JsValue> {
    let result = JsFuture::from(reader.read()).await?;
    if field(&result, "done")?.as_bool() == Some(true) {
        return Err("stream ended".into());
    }
    Ok(Uint8Array::new(&field(&result, "value")?).to_vec())
}
async fn setup(link: Link, transport: &Transport) -> Result<(), JsValue> {
    JsFuture::from(transport.ready()).await?;
    let datagrams = transport.datagrams();
    let profile = link
        .state
        .lock()
        .expect("link state poisoned")
        .profile
        .clone();
    set(
        &datagrams,
        "outgoingMaxAge",
        &(profile.age_ms as f64).into(),
    )?;
    set(
        &datagrams,
        "outgoingHighWaterMark",
        &((profile.stack_bytes / crate::DATAGRAM_LIMIT) as f64).into(),
    )?;
    set(
        &datagrams,
        "incomingHighWaterMark",
        &((profile.datagram_bytes / crate::DATAGRAM_LIMIT) as f64).into(),
    )?;
    let stream = JsFuture::from(transport.create_stream()).await?;
    let writer: Writer = field(&stream, "writable")?
        .unchecked_into::<Writable>()
        .writer();
    let reader: Reader = field(&stream, "readable")?
        .unchecked_into::<Readable>()
        .reader();
    JsFuture::from(writer.write(&Uint8Array::from(HELLO.as_slice()))).await?;
    let mut buffer = Vec::new();
    while buffer.len() < HELLO.len() {
        buffer.extend(read_chunk(&reader).await?);
    }
    if &buffer[..4] != HELLO {
        return Err("invalid hello".into());
    }
    buffer.drain(..4);
    link.state.lock().expect("link state poisoned").opened();
    let outbound = link.clone();
    spawn_local(async move {
        if reliable_out(&outbound, &writer).await.is_err() {
            outbound
                .state
                .lock()
                .expect("link state poisoned")
                .close(CloseReason::Transport);
        }
    });
    let inbound = link.clone();
    spawn_local(async move {
        if reliable_in(&inbound, &reader, buffer).await.is_err() {
            inbound
                .state
                .lock()
                .expect("link state poisoned")
                .close(CloseReason::Protocol);
        }
    });
    let writer = field(&datagrams, "writable")?
        .unchecked_into::<Writable>()
        .writer();
    let reader = field(&datagrams, "readable")?
        .unchecked_into::<Readable>()
        .reader();
    let outbound = link.clone();
    spawn_local(async move {
        let result = async {
            while outbound.is_open() {
                JsFuture::from(writer.writer_ready()).await?;
                let bytes = {
                    let mut state = outbound.state.lock().expect("link state poisoned");
                    state.expire(now());
                    state.pop(Lane::Datagram)
                };
                if let Some(bytes) = bytes {
                    JsFuture::from(writer.write(&Uint8Array::from(bytes.as_slice()))).await?;
                } else {
                    pause().await;
                }
            }
            Ok::<(), JsValue>(())
        }
        .await;
        if result.is_err() {
            outbound
                .state
                .lock()
                .expect("link state poisoned")
                .close(CloseReason::Transport);
        }
    });
    spawn_local(async move {
        let result = async {
            while link.is_open() {
                let bytes = read_chunk(&reader).await?;
                if bytes.is_empty() || bytes.len() > crate::DATAGRAM_LIMIT {
                    return Err("invalid datagram".into());
                }
                link.state
                    .lock()
                    .expect("link state poisoned")
                    .receive(Lane::Datagram, bytes);
            }
            Ok::<(), JsValue>(())
        }
        .await;
        if result.is_err() {
            link.state
                .lock()
                .expect("link state poisoned")
                .close(CloseReason::Transport);
        }
    });
    Ok(())
}
async fn reliable_out(link: &Link, writer: &Writer) -> Result<(), JsValue> {
    while link.is_open() {
        JsFuture::from(writer.writer_ready()).await?;
        let bytes = link
            .state
            .lock()
            .expect("link state poisoned")
            .pop(Lane::Reliable);
        if let Some(bytes) = bytes {
            let length = u32::try_from(bytes.len()).expect("bounded message");
            let mut frame = Vec::with_capacity(4 + bytes.len());
            frame.extend_from_slice(&length.to_le_bytes());
            frame.extend(bytes);
            JsFuture::from(writer.write(&Uint8Array::from(frame.as_slice()))).await?;
        } else {
            pause().await;
        }
    }
    Ok(())
}
async fn reliable_in(link: &Link, reader: &Reader, mut buffer: Vec<u8>) -> Result<(), JsValue> {
    while link.is_open() {
        if buffer.len() >= 4 {
            let length = u32::from_le_bytes(buffer[..4].try_into().expect("four bytes")) as usize;
            if length == 0 || length > RELIABLE_LIMIT {
                return Err("invalid frame length".into());
            }
            if buffer.len() >= length + 4 {
                let room = {
                    let state = link.state.lock().expect("link state poisoned");
                    state.inbound_bytes[0] + length <= state.profile.reliable_bytes
                };
                if !room {
                    pause().await;
                    continue;
                }
                link.state
                    .lock()
                    .expect("link state poisoned")
                    .receive(Lane::Reliable, buffer[4..4 + length].to_vec());
                buffer.drain(..4 + length);
                continue;
            }
        }
        buffer.extend(read_chunk(reader).await?);
    }
    Ok(())
}
