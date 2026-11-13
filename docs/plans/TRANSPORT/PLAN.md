# TRANSPORT plan

Two processes exchange messages over real internet protocols, and one of them may be a browser.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

Read [Mechanisms](DESIGN.md#mechanisms) and [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) before
starting either milestone, and [Exact baselines](../ARCHITECTURE.md#exact-baselines) before landing.
[The planning probe](../../evidence/TRANSPORT/planning-probe.md) says what was measured before the design was
written; its code was thrown away on purpose, so build from the design.

## Open for the human

- Assumption: TRANSPORT waits for the V1 exit (`SHOWCASE` M8), because the owner's page puts the game after V1 ("Once V1 lands, the next goal is an original game"). It shares no file with the renderer, so it could start sooner if the owner says; each landing would still pay one exact-baseline verifier run, because it adds a crate and lockfile entries.
- Assumption: "over the network" is proved between separate processes on this Mac, over real QUIC and WebTransport on loopback, through a relay that adds 100 ms of round trip, ±20 ms of jitter and 2% loss. No second machine, Wi-Fi, home router or internet path is run, and a public host stays the owner's to order.
- Assumption: Chrome is the only browser run, and A0's WebSocket fallback is not built. Safari and Firefox now have WebTransport and are designed for, not tried.
- Assumption: a player joins with a join ticket, which is the host's address and its certificate's fingerprint, about a hundred characters to put in a link or paste. A ticket stops working when its host stops. A short code a player could type needs a directory service, which is not built.
- Assumption: a host holds up to four players, and "hold a session" is proved for ten minutes with four clients (two on the Mac, one in Chrome, one in the iPhone simulator).
- Assumption: the iPhone simulator proves that the link works there, never its speed. A physical iPhone, where the local-network permission applies, is not run.
- Assumption: builders and reviewers ask to run the socket proofs outside the Codex sandbox, one request for each run, as they do today for the GPU. Every socket proof is behind one command to keep that to one kind of request. The other choices, full access for the sandbox or a Claude builder for this module, are owner decision 11 in the module list.

## M1 A Mac program, a Chrome page and an iPhone-simulator program stay connected to one host through delay, jitter and 2% loss, and reconnect after a drop

Criteria: M1.1–M1.12 in the done-when file.

Nothing in the repository opens a socket today. `thrum-network` holds byte codecs that belong to `SESSION`; this
milestone neither edits it nor depends on it, and the link carries opaque bytes. Its `MAX_MESSAGE_BYTES` and A5's
1,000-byte datagram budget are where the two limits come from. `thrum-asset`'s `remote/browser.rs` shows the
hand-written `wasm_bindgen` imports the browser adapter follows, and `thrum-network/examples/` shows the stdin
targets M2 follows. The tick-quantised `LinkConditioner` in `crates/thrum-core/tests/support/v1_workload.rs` stays
as it is. New packages and every direct outside dependency are registered in `.config/architecture.toml`, and the
new crates are fetched once with the network.

The order matters in one place: write the link script against the in-memory link first, then make each socket
adapter pass the same script. Record costs last, at the commit being claimed.

## M2 A peer that sends garbage, floods, stalls or freezes cannot stop the host or slow the other players, and a game without multiplayer contains no network code

Criteria: M2.1–M2.11 in the done-when file.

M1 left the adapters, the relay, the proof command and the session run, which M2.9 repeats unchanged. The
transport section of `docs/design/modules/NETWORKING.md` is the owning design M2.7 brings up to date, and the
maturity ledger is `docs/design/maturity.json`.

Stop rules, for both milestones:
- If the engine's Chrome adapter cannot hold datagrams to the local host, or the simulator cannot reach it, keep the measurements and end `BLOCKED`. Do not move a lane onto the stream, drop a target or shorten the script.
- Never relax a certificate check to make a client connect: no Chrome flag that touches certificates, QUIC or origins, and no option that skips the hash.
- Nothing that opens a socket, starts Chrome or calls the simulator runs under `cargo test`'s defaults or in `./dev check`. Those proofs go through the proof command, outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute the in-memory link, Node or another target's result.
- No time, byte count or allocation count stops the build (brief, decision 3). Record a row that trips a tripwire as measured, note what you found in the log, and keep building. The one figure that fails a run is the design's 90% floor on datagrams delivered.
- Record only runs of the commit being claimed, with the load on the machine.
- Adding a dependency must not change the version of any crate already in the lockfile. Run the exact-baseline verifier before `land`, and again when `land` refuses a rebased commit for lack of a proof.
- Leave `thrum-network` and `thrum-core` as they are. A change either needs is a proposal in the log and `BLOCKED needs_dependency`.
