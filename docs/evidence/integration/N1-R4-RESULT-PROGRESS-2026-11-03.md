# Event-free R4 result progress — 2026-11-03

The registered `thrum-network::progress` owner reserves a 256-entry client
input inventory from an externally admitted same-epoch baseline. It keeps the
contiguous `resolvedThrough` cursor and individually resolved higher inputs
across a gap. `reserve_input` checks capacity and sequence exhaustion before
consuming a new sequence. A live result must name one retained, unresolved
input with the declared action type, and its resolution tick must be strictly
after the controlled partition's preceding admitted sample and no later than
the new group's sample. A controlled result group must carry the complete
controller actor record. The group watermark must equal the exact contiguous
prefix after applying all newly named results; a pending gap cannot be skipped.

`Replica::admit_with_progress` preflights progress, validates and publishes the
state/lifecycle candidate, then commits the infallible progress changes. A
state refusal leaves the progress inventory unchanged; a progress refusal
leaves confirmed state unchanged. Six focused tests cover higher result 8
waiting for 7, exact watermark closure, baseline-seeded higher resolution,
prior-sample temporal refusal after a live group and after a tick-10
baseline, schema refusal after progress preflight, 257th input reservation
and event refusal. Command: `RUSTC_WRAPPER= rustup run 1.99.1 cargo test -p
thrum-network --test r4_progress`. Independent read-only review found and
corrected the temporal closure gap and found no residual issue in that fix.
The full `RUSTC_WRAPPER= ./dev check` gate passes for the checkpoint. Its
first run hit an unrelated `thrum-source` fixture's `Budget(Busy)` while
other repository work was active; that exact test passed alone, and the
complete gate passed on one rerun.

This is an **event-free** result slice. Any event record or result with an
immediate-event range is refused before state mutation. The baseline's
pending input inventory, control assignment and epoch must be verified by
the future session/baseline owner; this code does not authenticate an epoch,
validate envelope sequence or promise reconnect durability. An application
consumer receives the borrowed result records only after successful paired
admission. No event deduplication, outbound ACK/journal, W1 capture, transport,
allocator-call/RSS or whole-callback performance claim is made. The broader
N1 build-plan item stays open.
