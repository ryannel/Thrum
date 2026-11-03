# First R4 StateGroup payload codec — 2026-11-03

`thrum-network::r4` is the first running N1 component. It encodes one fixed
R4 StateGroup payload into caller-owned bytes and decodes a borrowed view of
those bytes. The crate has no third-party dependencies. It owns no W1 state,
socket, session, baseline, ACK or replica.

The fixed payload has a 32-byte header, 56-byte actor, 16-byte lifecycle,
32-byte action result and 40-byte event. Counts are checked before length
arithmetic or record access. The declared maxima of 32 actors, 32 lifecycle
records, 64 results and 64 events yield **6,944 payload bytes**, or **7,008
bytes with the separate 64-byte envelope**, within the 8 KiB group limit.
The encoder validates destination and result shape before writing; malformed
counts, lengths, disposition/event ranges and truncated input refuse. The
decoder returns iterators over borrowed fixed records without a decoded
array allocation.

Four tests pass, including a literal 176-byte independent golden vector that
checks every field's offset, signed value and little-endian representation in
both directions. The maximum-count fixture uses distinct actor, lifecycle,
result and event IDs. The focused command is
`RUSTC_WRAPPER= rustup run 1.99.1 cargo test -p thrum-network --locked --offline`.
The full `RUSTC_WRAPPER= ./dev check` passed after the crate was registered.
An independent review found no actionable codec defect after the golden
vector was added.
With the pinned Rust compiler selected explicitly, `cargo check --locked
--offline -p thrum-network --target TARGET` also passed for
`wasm32-unknown-unknown`, `aarch64-apple-ios-sim` and
`aarch64-linux-android`. These are compile checks, not browser, phone or
transport runs. The local Homebrew `rustc` has no rustup cross-target
libraries, so the direct checks set `RUSTC` to
`rustup which --toolchain 1.99.1 rustc`, matching `./dev`'s pinned compiler
selection.

This is **structural framing**, not semantic admission. Envelope identity,
manifest ordinals, actor range/quaternion validity, event origin, lifecycle
dependency closure, controller-state/result linkage, group partition
generation and cross-group replica ordering remain unimplemented. An
[isolated release CPU probe](N1-R4-PAYLOAD-COST-2026-11-03.md) measures only
this codec; allocation requests, RSS and whole-path CPU remain unqualified. The next
slice must connect validated source projection to a bounded N1-owned journal
and one multi-group consumer before a network-performance claim.
