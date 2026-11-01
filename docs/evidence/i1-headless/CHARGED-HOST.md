# Sealed charged host callback

1 November 2026. This is a bounded host handoff proof, not the complete process byte gate. `HeadlessHost::with_charged_replica` builds a finite charged replica from a sealed charged baseline. `on_callback_charged` runs the same validated authority-then-replica stage order as the prototype callback, publishes shared charged outcomes, and retains the committed snapshot until its caller releases the callback result. The ordinary callback rejects a charged host before advancing its clock.

## Object headroom and liveness

The local replica table has a fixed maximum live count. The host now checks `replica.len + (all already queued inputs + this new input) × max_spawns_per_action` before a **new** input sequence enters the authority. Exact pending or resolved duplicates return their existing answer before this capacity check. A host constructed around pre-existing pending inputs also checks their conservative credits. This is an object-slot headroom guarantee, not a byte credit for future group/output/history allocations. It is conservative: a profile at its object limit can refuse an update, rejection or despawn if its game-wide declaration says that action could spawn. A useful profile needs enough spare object slots for its maximum accepted input envelope, or a later per-action bound; refusal occurs before sequence consumption and cannot strand an accepted action.

After the authority's single due-input scan, a defensive step check compares this tick's maximum declared spawns with the already charged replica table. If it fails despite host input admission, that is a terminal contract fault rather than an impossible retry loop. Ordinary snapshot/group pool backpressure remains retryable before edits. The host's callback result vector, replica progress maps/validation scratch, completed authority history and future byte credits are not yet charged.

The tests prove a two-peer callback with one committed event, one rejection, both results and watermarks, confirmed state and last-reader charge release. A second two-peer case reserves the last free slot for one accepted action, returns its pending duplicate, and refuses a future-timed peer's new sequence. When the first action is rejected, its credit is released; the future-timed action then enters, holds its credit across an idle tick and spawns at its due tick. Another new action is refused before sequence consumption, while resolved duplicate lookup and later idle ticks continue. A two-tick catch-up callback delivers the first charged group once before a later injected rule fault, then stays terminal. `RUSTC_WRAPPER= ./dev check` passes.

## Local callback cost

The [release fixture](../../../crates/thrum-core/examples/i1_charged_replica_cost.rs) runs 1,000 warmups and 10,000 measured callbacks, each with one closed-value replacement, one accepted action, immediate acknowledgement and two registered peers. Synthetic monotonic callbacks arrive at approximately 60 Hz; the wall time measured is submit + callback + acknowledgement. The intentionally loose one-second CPU budget avoids turning OS scheduling spikes into benchmark overload; overload behavior has separate [host evidence](HOST.md). Two macOS/aarch64 runs gave these p50 nanoseconds:

| Records | Prototype, runs 1 / 2 | Charged, runs 1 / 2 |
| ---: | ---: | ---: |
| 64 | 708 / 792 | 833 / 875 |
| 4,096 | 1,083 / 1,166 | 1,166 / 1,333 |

The charged path costs about 8–17% more at these local medians; run-to-run and tail noise is material, and the two modes run sequentially. This cost buys explicit shared-owner and finite-object admission on this slice. The fixture does not measure a device, real callback scheduling, transport, host delivery byte charges, or 1%/25% multi-object churn. The parent memory gate and whole-engine performance qualification remain open.
