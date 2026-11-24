//! Wasm entry point driven by Chrome's page clock and WebTransport.

use wasm_bindgen::prelude::*;

use super::{Player, Trace, token_from_text};
use thrum_network::session::World;
use thrum_transport::{Profile, Ticket};

#[wasm_bindgen]
pub struct SocketPlayer(Player);

fn js_error(error: impl std::fmt::Display) -> JsError {
    JsError::new(&error.to_string())
}

#[wasm_bindgen]
impl SocketPlayer {
    #[wasm_bindgen(constructor)]
    pub fn new(ticket: &str, bodies: u32, saved: &str, trace: &str) -> Result<Self, JsError> {
        let ticket = ticket.parse::<Ticket>().map_err(js_error)?;
        let token = (!saved.is_empty())
            .then(|| token_from_text(saved))
            .transpose()
            .map_err(js_error)?;
        Player::new(
            &ticket,
            Profile::default(),
            bodies,
            token,
            Trace::parse(trace).map_err(js_error)?,
            false,
        )
        .map(Self)
        .map_err(js_error)
    }

    pub fn advance(&mut self) -> Result<String, JsError> {
        self.0
            .advance()
            .map(|rows| serde_json::Value::Array(rows).to_string())
            .map_err(js_error)
    }

    pub fn perturb(&self) -> Result<String, JsError> {
        self.0
            .perturb()
            .map(|row| row.to_string())
            .map_err(js_error)
    }

    pub fn tick(&self) -> u64 {
        self.0.client.world().tick()
    }

    pub fn token(&self) -> Option<String> {
        self.0.token_out.map(|token| crate::hex(&token.to_bytes()))
    }
}
