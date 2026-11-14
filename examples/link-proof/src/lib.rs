//! Target runners for the engine's one shared link script.
use serde_json::{Value, json};
use thrum_transport::{Link, proof::Script};
mod cost;
static IDENTITY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
fn identity() -> u64 {
    let count = IDENTITY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    #[cfg(not(target_arch = "wasm32"))]
    {
        (u64::from(std::process::id()) << 32) | count
    }
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now().to_bits() ^ count
    }
}

/// One runner state, shared by the native and Wasm entry points.
pub struct Actor {
    /// The independent content checker used by every case.
    pub script: Script,
    case: String,
    began: Option<u64>,
    exercised: bool,
    expiry: Option<Value>,
    cost: Option<cost::Cost>,
    draining: bool,
}
impl Actor {
    /// Select a proof case. Lifecycle cases send only when their case requests load.
    pub fn new(direction: u8, case: &str) -> Self {
        let mut script = Script::new(direction);
        script.identify(identity());
        if case == "session" {
            script.stop_at(u64::MAX);
        }
        Self {
            script,
            case: case.into(),
            began: None,
            exercised: false,
            expiry: None,
            cost: (case == "costs").then(|| cost::Cost::new(direction == 1)),
            draining: false,
        }
    }
    /// Run one nonblocking script step on either real or virtual time.
    /// # Errors
    /// Propagates all script failures without tolerating them.
    pub fn tick(&mut self, link: &Link, now: u64) -> Result<(), thrum_transport::CloseReason> {
        if self.draining {
            return self.script.drain(link);
        }
        if let Some(cost) = &mut self.cost {
            return cost
                .tick(link, now)
                .map_err(|_| thrum_transport::CloseReason::Protocol);
        }
        if !link.is_open() {
            return Ok(());
        }
        let start = *self.began.get_or_insert(now);
        let elapsed = now.saturating_sub(start);
        match self.case.as_str() {
            "exchange" | "session" => self.script.tick(link, now)?,
            "limits" => {
                if !self.exercised && elapsed >= 500 {
                    self.script.limits(link, now)?;
                    self.exercised = true;
                }
                self.script.drain(link)?;
            }
            "expiry" => {
                self.script.drain(link)?;
                if (1_000..1_750).contains(&elapsed) {
                    self.script
                        .expiry_burst(link, now)
                        .map_err(|_| thrum_transport::CloseReason::Protocol)?;
                }
                if elapsed >= 2_100 && self.expiry.is_none() {
                    self.expiry = Some(record(link, &self.script, now));
                }
            }
            "close-client" | "close-host" | "close-load" => {
                self.script.tick(link, now)?;
                let closer = u8::from(self.case == "close-host");
                if elapsed >= 1_500 && self.script.direction() == closer {
                    if self.case == "close-load" {
                        for _ in 0..16 {
                            match self.script.send(
                                link,
                                thrum_transport::Lane::Reliable,
                                thrum_transport::RELIABLE_LIMIT,
                                now,
                            ) {
                                Ok(()) => (),
                                Err(thrum_transport::LinkError::Full) => break,
                                Err(_) => return Err(thrum_transport::CloseReason::Protocol),
                            }
                        }
                    }
                    link.close(42);
                }
            }
            "idle" | "startup-cut" | "kill-peer" => (),
            _ => return Err(thrum_transport::CloseReason::Protocol),
        }
        Ok(())
    }
    /// Record any expiry snapshot taken before the stall heals.
    pub fn record(&self, link: &Link, now: u64) -> Value {
        let mut value = record(link, &self.script, now);
        value["expiry_while_stalled"] = self.expiry.clone().unwrap_or(Value::Null);
        if let Some(cost) = &self.cost {
            value["costs"] = cost.record();
        }
        value
    }
    /// Stop sending and keep draining, before the harness's final delivery comparison.
    pub fn drain_only(&mut self) {
        self.draining = true;
    }
}

/// Save independent application tallies alongside the adapter's accounting.
pub fn record(link: &Link, script: &Script, now: u64) -> Value {
    let counts = link.counters(now);
    let report = script.report();
    json!({
        "id": link.id(), "open": link.is_open(),
        "identity":format!("{:016x}",script.identity()),"peer_identity":script.peer_identity().map(|id|format!("{id:016x}")),
        "reliable_sent": report.reliable_sent, "reliable_received": report.reliable_received,
        "datagrams_sent": report.datagrams_sent, "datagrams_received": report.datagrams_received,
        "reordered": report.reordered,
        "lanes": counts.lanes.iter().map(|c| json!({"sent":c.sent,"received":c.received,"sent_bytes":c.sent_bytes,"received_bytes":c.received_bytes,"queued_bytes":c.queued_bytes,"oldest_ms":c.oldest_ms})).collect::<Vec<_>>(),
        "refused": counts.refused, "handed_on": counts.handed_on, "expired": counts.expired,
        "dropped_cap":counts.dropped_cap, "dropped_inbound":counts.dropped_inbound,
        "stack_bytes": counts.stack_bytes, "rtt_us": counts.rtt_us,
        "packets_sent": counts.packets_sent, "packets_lost": counts.packets_lost,
        "wire_bytes": counts.wire_bytes, "browser_discards": Value::Null,
    })
}

#[cfg(target_arch = "wasm32")]
/// Thin Wasm entry points; all message checks remain in Rust.
pub mod web {
    use super::Actor;
    use std::cell::RefCell;
    use thrum_transport::{Link, Profile, browser};
    use wasm_bindgen::prelude::*;
    thread_local! { static CLIENT: RefCell<Option<(Link, Actor)>> = const { RefCell::new(None) }; }
    /// Start the engine's Wasm client, pinning the host with the supplied ticket.
    /// # Errors
    /// Returns a malformed ticket or constructor failure.
    #[wasm_bindgen]
    pub fn start(ticket: &str, case: &str) -> Result<(), JsValue> {
        let ticket = ticket
            .parse()
            .map_err(|e: thrum_transport::LinkError| JsValue::from_str(&e.to_string()))?;
        let link = browser::connect(&ticket, Profile::default())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        CLIENT.set(Some((link, Actor::new(0, case))));
        Ok(())
    }
    /// Advance the same script used in virtual time and by native clients.
    /// # Errors
    /// Returns any content or order failure without tolerating it.
    #[wasm_bindgen]
    pub fn tick() -> Result<String, JsValue> {
        CLIENT.with_borrow_mut(|client| {
            let (link, script) = client.as_mut().ok_or("client not started")?;
            script
                .tick(link, browser::now())
                .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
            let mut report = script.record(link, browser::now());
            let mut events = Vec::new();
            while let Some(event) = link.event() {
                events.push(format!("{event:?}"));
            }
            report["events"] = serde_json::json!(events);
            Ok(report.to_string())
        })
    }
    /// Close with an application code.
    #[wasm_bindgen]
    pub fn close(code: u32) {
        CLIENT.with_borrow(|client| {
            if let Some((link, _)) = client {
                link.close(code);
            }
        });
    }
    /// Stop the shared script's sends while still receiving the accepted prefix.
    #[wasm_bindgen]
    pub fn drain_only() {
        CLIENT.with_borrow_mut(|client| {
            if let Some((_, actor)) = client {
                actor.drain_only();
            }
        });
    }
}
