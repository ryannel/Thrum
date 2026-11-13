# TRANSPORT design review

13 November 2026. Reviewed commit `f574992194499bc5f2e5f6f5e6450e90eabacff2`.

**Design changes requested: three Must Fix findings.** The design provides the right transport layer for the
human's multiplayer request, within its explicitly recorded loopback-only qualification. Its datagram expiry
mechanism and native link-opening sequence have gaps, and its session acceptance item promises delivery through
failures that the link contract correctly permits to interrupt delivery.

**Must Fix — datagram-expiry-after-handoff. Verified design gap by source inspection.**

[DESIGN.md](../../plans/TRANSPORT/DESIGN.md), lines 80, 186–188 and 261–263, promises to discard datagrams after
the profile's age limit. The native mechanism is to retain them in the adapter and hand Quinn only what it can
send immediately. M1.1–M1.4 require every adapter to pass this expiry part of the link script.

The selected Quinn 0.11.12 API does not provide that handoff. `send_datagram` enqueues bytes;
`send_datagram_wait` waits for buffer space; `datagram_send_buffer_space` reports capacity, not permission to
transmit now. These are documented in [Quinn's connection API](https://docs.rs/quinn/latest/quinn/struct.Connection.html#method.send_datagram).
I also read the installed quinn-proto 0.11.19 source: `src/connection/datagrams.rs:27–54` queues a `Datagram`
containing data without an expiry; `src/connection/mod.rs:604–640` subsequently applies congestion and pacing.
The inspected documentation identifies version 0.11.12.

Consider a datagram admitted while a reliable transfer has filled the congestion window. It leaves the adapter's
age-tracked queue but remains inside Quinn, with no deadline. The application then stops producing datagrams.
When the path recovers, nothing in the specified mechanism expires that last queued datagram. Making Quinn's
buffer small does not give its final entry an age limit. This counterexample follows from the source; I did
not run a socket reproduction. QUIC explicitly makes expiration an optional implementation facility, while
datagrams remain subject to congestion control. [RFC 9221, section 5.4](https://www.rfc-editor.org/rfc/rfc9221.html#section-5.4)

Chrome's `outgoingMaxAge` concerns time in its outgoing queue. It does not establish a deadline covering an
adapter queue plus browser buffering plus the network, and a resolved write need not mean the datagram was
sent rather than discarded. [WebTransport datagram processing](https://www.w3.org/TR/webtransport/#senddatagrams)
The design must also distinguish an adapter's observable expiry counts from browser-internal discards, especially
given its recorded absence of `getStats`.

Resolve this by defining exactly where age starts and stops and supplying an enforceable mechanism on each
adapter. State what remains SESSION's responsibility for rejecting old input. The proof must distinguish expiry
from packets simply lost at the relay and cover a queued final datagram after production stops. If the promise
is narrowed to time in an adapter-owned queue, change the shared contract and its consumer guidance explicitly;
do not keep claiming that the native and browser mechanisms enforce the stronger promise.

**Must Fix — native-link-open-needs-wire-progress. Verified design gap by source inspection.**

[DESIGN.md](../../plans/TRANSPORT/DESIGN.md), lines 96–100, 104 and 322–323, lets the host take newly opened
links and send messages, but its join sequence only says that the client opens a bidirectional stream and both
ends report open. No transport startup bytes are specified. Zero-length application messages are forbidden.

Quinn does not announce a newly opened stream until the opener writes to it. The host's `accept_bi` therefore
cannot complete merely because the client called `open_bi`.
[Quinn's stream-opening contract](https://docs.rs/quinn/latest/quinn/struct.Connection.html#method.accept_bi)
The selected web-transport-quinn 0.13.6 raw-session wrapper does not supply the missing header: in the inspected
`src/session.rs:570–576`, its `header_bi` is empty.

An ordinary consumer connects and waits for the host's welcome or baseline without first sending an application
message. If open awaits stream acceptance, the host never reports that link open. If open is reported earlier,
the host still has no accepted stream on which to deliver its first reliable message. Client-first traffic in
the planning probe conceals this dependency. This is a missing transport mechanism, not a demand to implement
SESSION's handshake inside TRANSPORT.

Specify how link establishment makes the stream usable independently of application traffic, and when each end
reports open. M1's shared script should establish an otherwise idle link and allow the first reliable application
message to come from the host. Keep any startup framing out of delivered application messages and account for
a peer that stalls during startup. No socket reproduction was run; the gap is established by the selected API's
documented behaviour and the absence of startup traffic in the design.

**Must Fix — session-proof-promises-delivery-after-death. Verified contradiction in the acceptance item.**

[DONE.json](../../plans/TRANSPORT/DONE.json), M1.5 at lines 60–63, requires every reliable message on every link
to arrive, while the same run kills a client and times another link out. It also asks for the close reason
reported by both ends for each event. M2.9 repeats this result. In contrast,
[DESIGN.md](../../plans/TRANSPORT/DESIGN.md), line 78, correctly promises delivery **or closure**.

A send can be accepted into the local queue just before its recipient is killed. That message cannot arrive at
the dead application, and that application cannot report a close event. Reconnect deliberately does not replay
old-link traffic. Even an orderly QUIC close can discard data not yet consumed by the peer application.
[Quinn's close semantics](https://docs.rs/quinn/latest/quinn/struct.Connection.html#method.close)

As written, a reviewer must either fail a correct implementation or silently reinterpret the item. Draining
everything before every destructive event could produce a pass while avoiding the interrupted-send case.
Align M1.5 and M2.9 with the delivery-or-closure contract: distinguish uninterrupted links from interrupted
ones, account for incomplete old-link traffic without replaying it, and distinguish a surviving endpoint's
reported reason from the harness's observation that a process was killed. Retain the requirement that healthy
links lose no reliable messages. This changes the proof's meaning, not its testing procedure.

**Suggestion — datagram-cap-policy. Verified omission in the interface description.**

The design says every queue has a cap, and M1.8 checks that, but the datagram send row only describes rejection
for size and emptiness plus expiry by age. A burst can fill a byte/message cap before any entry expires.
State the outbound datagram policy at that point and how it appears in counters. The bounded-queue requirement
already prevents an implementation from treating age alone as a memory cap; this clarification need not hold
up the build separately from the expiry finding.

**Suggestion — lifecycle-contract-citation. Verified attribution error.**

DESIGN.md line 193 attributes “a suspended app has lost its socket” to C11. C11 contains no such sentence.
The relevant obligation is in [NETWORKING.md](../../design/modules/NETWORKING.md#identity-connection-and-content),
which requires mobile suspend to stop input and acknowledgements and resume with a new epoch and baseline.
Correct the citation. The design already assigns background-close integration to the shell and SESSION, so
this attribution error does not itself leave the obligation unowned.

**Comparison with the human's request.**

I read [ACCEPTED.json](../../plans/TRANSPORT/ACCEPTED.json) and the project brief before the design. There is one
recorded planner task: “Plan module TRANSPORT. It needs: SHOWCASE”. The human's request carried by
[the project brief](../../plans/BRIEF.md) and [the next-goal page](../../design/NEXT-GOAL-SWARM-GAME.md) is several
players together over the network, with multiplayer additive to the reusable engine. The Mac, Chrome,
simulator and two-to-four-player range are recorded defaults. The pre-planning TRANSPORT brief at `ee9ac160`
already assigns native QUIC, browser WebTransport, an in-memory link, local host identity and counters to this
module, leaving fallback and runtime choices to its planner.

The proposed opaque message boundary, optional crate, single I/O owner, two lanes, host identity and reusable
conditioner fit that commission. SESSION retains authority, epochs, input timing, baseline and repair. No
game state or competing simulation authority is introduced. SHOWCASE:M8 remains the prerequisite. Omitting
WebSocket is a choice the prior module brief explicitly allowed.

The design does not prove play between separate machines or through an internet path. This narrowing is
explicit in PLAN.md's assumptions for the human, the architecture's owner decision 10, the design's risks and
M2.8's required maturity record. I therefore treat it as the declared qualification boundary of this module,
not an undisclosed reinterpretation or proof that the whole online-game goal is finished. No deployment or
spending is required to settle this review. Physical iPhone and other-browser qualification are also explicitly
owed rather than claimed.

**Done-when review.** I read every item in both milestones. The following assesses whether each can be judged
when built; it does not award milestone passes.

| Items | Design-stage assessment |
|---|---|
| M1.1–M1.4 | The common Rust script, virtual-time control, platform provenance, relay evidence and duplicate-check control reject useful fakes. Expiry and idle/host-first establishment need the first two fixes. |
| M1.5 | The third finding prevents literal acceptance as written. Four simultaneous clients and unaffected-link checks otherwise exercise the intended composition. |
| M1.6–M1.7 | Costs are measurable with M1 alone, compare both controllers and record absent Chrome statistics. Large transfers must use the stated message limit; the probe's oversized individual messages are not engine conformance evidence. |
| M1.8 | A public-boundary and queue-ownership read can judge nonblocking calls. Clarify the datagram cap policy as suggested. |
| M1.9–M1.11 | Dependency closure, Wasm linting and identity-path inspection can judge these without SESSION. Negative-ticket checks complement the identity inspection. |
| M1.12 | Existing exact-baseline verification is available independently of transport or later game work. |
| M2.1–M2.3 | Production-parser inspection, hostile peers beside healthy links, queue caps and one-thread sends make malformed-input and overload claims judgeable after M1. |
| M2.4–M2.6 | Process freeze/resume and page reload can be judged on the specified targets. The frozen-page observation correctly avoids equating a quiet renderer with a dead transport. |
| M2.7–M2.8 | Runnable consumer documentation and explicitly limited maturity claims are inspectable at M2. |
| M2.9 | Inherits M1.5's impossible delivery/reporting requirement; fix both together. |
| M2.10–M2.11 | Closing-commit closure and image verification can be judged without downstream modules. |

**Evidence and limits.** This is an independent design and proof-definition review, performed without delegation.
I read the request, brief, current and prior module entry, full design, plan and done-when file, applicable
contracts, networking/A0 decisions, calibration guidance, planning-probe report and module log. I inspected the
selected libraries' relevant source and primary protocol documentation for the findings above. The probe's
discarded source and raw captures were unavailable; its performance claims are treated as provisional findings
to repeat in M1. I did not build an adapter, run socket/platform proofs, inspect game imagery or claim runtime,
visual or device qualification. No earlier design review exists in this checkout. Only this review file is changed.

Validation: `./dev check-docs` passed (764 Markdown files, 4,796 local links, 30 module records, crate policy
and current design hashes). Its code-size notes concern existing files outside this review.

DESIGN CHANGES: 3 must_fix findings
