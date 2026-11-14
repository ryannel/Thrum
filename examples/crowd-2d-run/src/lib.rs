//! Target-independent conformance and input-wire route. Platform launch is tooling-owned.
mod costs;
pub use costs::costs;
use crowd_2d::{Simulation, run_trace};
use serde_json::{Value, json};
use thrum_core::{profile::platform_sine_control, sim::InputFrame};
use thrum_network::input_batch::{self, ActionView, ActionWire, EncodeRecord, InputCodecError};

struct NoActions;
impl ActionWire<()> for NoActions {
    fn encode(
        &self,
        _action: &(),
        _destination: &mut [u8],
    ) -> Result<(u32, usize), InputCodecError> {
        Err(InputCodecError::InvalidAction)
    }
    fn decode(&self, _action: ActionView<'_>) -> Result<(), InputCodecError> {
        Err(InputCodecError::InvalidAction)
    }
}

/// Round-trip whole frames through the production input-batch codec.
///
/// # Panics
/// The caller supplies invalid frames or actions.
pub fn wire_route(frames: &mut [InputFrame<()>; 4]) -> u64 {
    let mut bytes = [0; 4096];
    let records: Vec<_> = frames
        .iter()
        .map(|frame| EncodeRecord {
            frame,
            actor_wire_id: frame.actor.0,
        })
        .collect();
    let length = input_batch::encode(&records, &NoActions, &mut bytes).expect("valid input batch");
    let decoded = input_batch::decode(&bytes[..length]).expect("decode own batch");
    let mut changed = 0;
    for (frame, record) in frames.iter_mut().zip(decoded.records()) {
        changed += frame
            .axes
            .iter()
            .zip(record.axes)
            .filter(|(left, right)| left.to_bits() != right.to_bits())
            .count() as u64;
        let axes = record.axes;
        // Decode the complete frame, including identity, sequence and actions.
        *frame = record
            .decode_frame(
                thrum_core::sim::ControlBinding {
                    peer: frame.origin.peer,
                    connection_epoch: frame.origin.connection_epoch,
                    actor: frame.actor,
                },
                frame.actor.0,
                &NoActions,
            )
            .expect("typed frame");
        assert_eq!(frame.axes, axes);
    }
    changed
}

/// Execute one crowd trace and return JSON without regenerating any reference.
///
/// # Panics
/// A rule, history operation or profile vector violates the conformance contract.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn conformance(count: u32, ticks: u32, straight: bool, wire: bool, control: bool) -> String {
    let mut simulation = Simulation::new(count, 1);
    if control {
        simulation.game.control = Some(platform_sine_control);
    }
    let result = run_trace(&mut simulation, u64::from(ticks), straight, |frames| {
        if wire { wire_route(frames) } else { 0 }
    });
    json!({ "bodies": count, "ticks": ticks, "straight": straight, "wire": wire, "control": control,
        "fold": format!("{:016x}", result.fold),
        "checkpoints": result.checkpoints.iter().map(|&(tick, digest)| json!([tick, format!("{digest:016x}")])).collect::<Vec<_>>(),
        "timeline": result.timeline.iter().map(|digest| format!("{digest:016x}")).collect::<Vec<_>>(),
        "reused": result.reused, "hits": result.hits, "rewinds": result.rewinds, "changed_rewinds": result.changed_rewinds,
        "changed_axes": result.changed_axes, "neighbours": result.neighbours }).to_string()
}

/// Decode a JSON result produced by the shared runner.
///
/// # Errors
/// The result is malformed JSON.
pub fn read_result(value: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(value)
}

/// Return the profile's fixed arithmetic vectors for comparison by the host.
///
/// # Panics
/// If serializing a fixed-width integer vector fails.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn vectors() -> String {
    serde_json::to_string(&thrum_core::profile::conformance_vectors())
        .expect("integer vectors serialize")
}
