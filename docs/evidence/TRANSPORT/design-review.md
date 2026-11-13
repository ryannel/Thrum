# TRANSPORT design review

13 November 2026. Reviewed commit `50be8377a702f2cee6da806e1aef1c0fc89052d6` against the previous review at
`f574992194499bc5f2e5f6f5e6450e90eabacff2`.

**Design approved: no Must Fix findings.** The revision provides TRANSPORT's part of the human's request for
several players together over the network. All three previous Must Fix findings are resolved. Approval covers
the declared module scope and proof definitions; it does not qualify play between separate machines or complete
the multiplayer game. One wording suggestion below does not hold up building.

**Previous Must Fix — datagram-expiry-after-handoff: resolved. Verified by design and source inspection.**

[DESIGN.md](../../plans/TRANSPORT/DESIGN.md), lines 86–102, 229–248 and 332–341, now bounds time only in the
link's own outbound queue. It expressly permits late delivery after handoff. A timer expires queued datagrams
even after production stops; capacity drops have a separate counter. Quinn's buffer has a small configured
allowance, and handoff waits for capacity instead of silently evicting older datagrams. Chrome's internal
discards are not misreported as observed adapter expiry.

I read installed Quinn 0.11.12 `src/connection.rs:468–483, 845–891` and quinn-proto 0.11.19
`src/connection/datagrams.rs:27–54, 97–109`. A blocked `send_datagram_wait` future retains its bytes without
enqueuing them; successful polling enqueues them, and capacity accounting includes datagram storage overhead.
The adapter can therefore keep a pending handoff subject to its queue deadline and cancel it on expiry. The
design no longer requires Quinn to withdraw an already queued datagram. Treating a pending handoff as completed
would still violate the revised queue-age and accounting requirements.

The browser design observes writer readiness and distinguishes its queue from the browser's. The
[WebTransport datagram procedure](https://www.w3.org/TR/webtransport/#senddatagrams) resolves writes for both
expiration and transmission; a resolved write does not establish delivery. The revised counters respect that
distinction. M1.1–M1.3 and mechanism 8 require sender-side accounting while the path remains stalled and sending
has stopped, with positive expiry on the in-memory and Quinn ends. Relay loss cannot substitute for expiry.
Chrome's own queue backing up remains an observation, not an assumed capability. The in-memory stack allowance
also represents late datagrams permitted by the revised contract.

The narrowing reaches the consumer: [ARCHITECTURE.md](../../plans/ARCHITECTURE.md), lines 552 and 649–653,
and design lines 441–449 tell SESSION to reject old content and not assume delivery before closure. C13 and
[NETWORKING.md](../../design/modules/NETWORKING.md#commands-clocks-and-state-publication) already place epoch,
ordering and input-age validation there. M2.7 requires these limits in the consumer guide. No numbered contract
promises the stronger datagram deadline that was removed.

**Previous Must Fix — native-link-open-needs-wire-progress: resolved. Verified by design and source inspection.**

Design lines 136–156 define startup independently of game traffic: the client writes a link hello, the host
reads it and writes its answer before reporting open, and the client reports open after reading that answer.
The hello is excluded from application messages and their counters. A startup deadline covers the handshake
and hello; incomplete startup remains charged to the host's pending-connection cap.

I confirmed the write-before-accept requirement in Quinn's `src/connection.rs:344–356` and the empty raw-session
header in web-transport-quinn 0.13.6 `src/session.rs:570–576`. The hello supplies those missing bytes; the host's
first application message follows its hello on the ordered stream. M1.1 and the common lifecycle script require
an otherwise idle link with the host sending first. M2.1 and M2.2 cover malformed and missing hellos. M1.6 now
measures connect-to-open time, including startup. None of these checks needs SESSION's later handshake.

**Previous Must Fix — session-proof-promises-delivery-after-death: resolved. Verified by acceptance-item and
design inspection.**

M1's bar, M1.5 and M2.9 distinguish surviving links from event-ended links. Healthy links must deliver all
accepted reliable messages after the final drain. Interrupted links deliver an ordered prefix, with no remaining
old-link traffic replayed onto a replacement. Design lines 360–368 explicitly limit judgement of a killed
process to the surviving host's observations; no report or receipt tally is demanded from the dead process.
This matches Quinn's immediate-close semantics, inspected in `src/connection.rs:403–436`.

The repair retains a meaningful interruption test. Links carry load until closure; the cut must leave accepted
messages undelivered, and the host must accept messages for the killed client after the kill. Harness observations
of destructive events are separate from endpoint close reasons. Draining every link before disruption no longer
satisfies M1.5. The quiet-link timeout test remains in the common lifecycle script. M2.9 uses the same session
command and mechanism at the closing commit.

**Previous Suggestions: both resolved.** `datagram-cap-policy` is explicit: discard the oldest queued datagram
to admit a new one, counting capacity drops separately from expiry (design lines 229–232).
`lifecycle-contract-citation` now points to NETWORKING's mobile-suspend rule (lines 251–258), with background
close integration still assigned to the shell and SESSION.

**Suggestion — shorten-expiry-proof-wording. Verified wording issue; not a missing mechanism.**

Design lines 332–341 begin with a prescribed sequence that sends until the queue backs up, then finish by
allowing Chrome's queue not to back up. M1.3's question and the planner's response make the intended exception
clear. On the next edit, state the required observations and the Chrome distinction together, leaving the
harness route to the builder. Preserve stopped-production expiry, separate loss/cap/expiry accounting and the
positive-expiry requirement on Quinn and the in-memory link. This is a suggestion to simplify the existing
proof, not to add steps or require a positive browser-expiry count.

**Comparison with the human's request.**

I read the whole `briefs` array in [ACCEPTED.json](../../plans/TRANSPORT/RECORD.json), oldest first, and
[BRIEF.md](../../plans/BRIEF.md) before the design. The initial task commissions TRANSPORT after SHOWCASE;
the second requests repair of the previous review and includes a log-length correction. Neither changes the
human's request, recorded in [the next-goal page](../../design/NEXT-GOAL-SWARM-GAME.md): several players play
together over the network, as part of an original swarm-survival game on a reusable engine.

Opaque messages, an optional socket crate, nonblocking calls, bounded per-link storage, host identity and an
in-memory rehearsal link fit TRANSPORT's assigned responsibility. SESSION retains game meaning, authority,
epochs, admission and recovery. The hello is transport framing, not a competing session protocol. The revision
adds no game state or unrequested hosting service. SHOWCASE:M8 remains the prerequisite.

As in the previous review, [PLAN.md](../../plans/TRANSPORT/PLAN.md) explicitly presents loopback-only proof,
Chrome-only browser qualification, no WebSocket fallback and no physical iPhone as assumptions for the human.
M2.8 preserves those limits in the maturity ledger. They remain limitations of this module's qualification,
not evidence that the full online-game goal has been achieved. No new narrowing of the human's request is
hidden in this revision.

**Changed-item review.** These are design-stage assessments, not milestone passes. I read every item in both
milestones, including each item named in the review request.

| Changed definition | Assessment against the previous findings |
|---|---|
| M1 bar / dependency | The bar now conditions complete reliable delivery on the link staying up. SHOWCASE:M8 is unchanged. |
| M1.1 | Virtual-time stall, stopped-production expiry and idle host-first startup cover the first two repairs. The shared script and deliberate duplicate fault remain. |
| M1.2 | Sender expiry must be positive in both directions and separate from relay loss. The 90% delivery floor excludes intentional limit/expiry discards while retaining lanes/load coverage. |
| M1.3 | Still requires the engine's Wasm adapter and same script. Browser queue behaviour is recorded; unavailable browser discards are absent, not zero. Mechanism 8 supplies the deliberate browser distinction discussed above. |
| M1.5 | Judgeable with delivery-or-closure, surviving-end reports and evidence of interrupted sends. Healthy links retain full delivery requirements. |
| M1.6 | Connect-to-open includes the hello. Costs remain baselines under both controllers, not performance gates. |
| M2.1 | Adds the actual hello decoder to malformed/partial-input tests and stdin targets. Available after M1; no later session code is needed. |
| M2.2 | Adds incorrect-first-frame and never-send-hello peers beside healthy links. Together with the startup cap/deadline, this tests the new pre-open resource lifetime. |
| M2.7 | Requires consumer documentation of late datagrams and reliable-message loss at close, completing the shared-interface repair. |
| M2.9 | Repeats the corrected session result using the same command; it no longer requires a killed endpoint to deliver or report. |

The remaining definitions retain their previous judgeability: simulator provenance, public-boundary and
dependency inspection, identity checks, hostile/slow-peer isolation, lifecycle observations, maturity limits and
exact-baseline verification. Newly empty `verdict`/`review` fields do not alter their obligations. None needs
SESSION or SWARM built to demonstrate TRANSPORT's assigned behaviour.

**Evidence and limits.** This review was performed independently without delegation. I read the full revised
design, plan and done-when file, the previous review, both planner tasks, relevant shared contracts and consumer
guidance, A0/A5 decisions, the revision diff and planner response/probe report. I checked the changed mechanisms
against installed library source and the primary WebTransport specification. The planner discarded its probe
source, so its numerical rows are supporting claims to repeat in M1, not independent runtime evidence here.
I did not build adapters, rerun socket/platform probes, inspect game imagery or claim device qualification.
Only this review file was edited; DONE.json and the plans remain unchanged.

Validation: `./dev check-docs` passed (765 Markdown files, 4,813 local links, 30 module records, crate policy
and current design hashes). Its code-size notes concern existing files outside this review. `git diff --check`
passed; the working-tree status lists only this review file.

DESIGN OK
