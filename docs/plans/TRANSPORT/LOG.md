# TRANSPORT log

- 2026-11-13 Planned from the lead's task (row 17 of the module list); no words of the owner's were passed on, so
  the design quotes the owner's page of 10 November. Two milestones replace the brief the module list carried,
  whose text is in git (`git show ee9ac160:docs/plans/ARCHITECTURE.md`). M1 waits on `SHOWCASE:M8`.
- 2026-11-13 Planner's decisions, all three left to the planner by that brief. The WebTransport crate: Quinn with
  `web-transport-quinn` on the host, and the browser adapter binds the page's `WebTransport` by hand, because
  `web-transport-wasm` needs a build flag on every Wasm build; `wtransport`, `webtrans` and `h3-webtransport` were
  audited and not chosen. The runtime: one I/O thread with a single-threaded Tokio runtime, meeting the world lane
  only at bounded queues. The WebSocket fallback is not built.
- 2026-11-13 Planner's probe, in [the evidence folder](../../evidence/TRANSPORT/planning-probe.md). One Quinn
  endpoint served a Mac client, an iPhone-simulator client and headless Chrome 157 on one port, with datagrams, a
  pinned 13-day P-256 certificate and no browser flag. So the fallback the brief named, input on the reliable
  stream for Chrome, is not built; it is what a planner applies if M1's Chrome row fails.
- 2026-11-13 Planner's decision from the probe: BBR on the Quinn ends. Under A0's 2% loss Quinn's default Cubic
  held about 50 KB/s, so a 1 MiB message from the host took 18 s and delayed everything behind it; BBR took 0.7 s
  and sent up to 3.2 times the bytes. Quinn marks its BBR experimental, so the profile keeps Cubic selectable and
  M1 records both for `SESSION`'s planner. Chrome's upload is its own controller: 256 KiB up took about 5.5 s.
- 2026-11-13 Planner's findings for other modules. A frozen Chrome page keeps its link open, because Chrome's
  network process answers keep-alives, so `SESSION` must time out silent input itself. A join ticket is about a
  hundred characters, a link or a paste and not a typed code, which answers part of `GAME-UI`'s open question.
  Chrome 157 has no `getStats`, so a browser link's transport figures come from the host's side.
- 2026-11-13 Planner's decision on proof: nothing that opens a socket runs in `./dev check`, which every module's
  builder runs inside the Codex sandbox; socket proofs sit behind one command. The A0 conditioner is read as 50 ms
  ± 10 ms and 2% loss in each direction, applied to UDP packets by a relay and to messages by the in-memory link.
- 2026-11-13 Researched: the candidate crates' sources and release history, RFC 9000 and RFC 9221, MDN and the W3C
  draft on `serverCertificateHashes`, Mathis et al. (1997) on loss-based throughput and Cardwell et al. (2016) on
  BBR. Two assumptions moved here from the module list: the proof on this Mac, and Chrome with no WebSocket.
- 2026-11-13 Checks: every item check parses as shell and fails, as it should before the work exists. The two
  verifier items were not dry-run, because a builder held the GPU lock; they are the command `SIM-2D` uses.
  `./dev check-docs` passes with the new files.
- 2026-11-13 The design has not been reviewed. No builder starts until a design review returns Design Approved.
- 2026-11-13 [Design review](../../evidence/TRANSPORT/design-review.md) of `f5749921`: design changes, three Must
  Fix findings and two suggestions. [The planner's answer](../../evidence/TRANSPORT/design-review-response.md)
  holds the reasoning for each entry below, what was considered and not chosen, and a second probe.
- 2026-11-13 Must Fix, datagram expiry after handoff: the promise is narrowed and said outright. A datagram's age
  runs from the send call until it leaves the link's own queue, where a timer enforces it. Beneath, a new profile
  setting, the stack allowance, caps what Quinn or the browser holds with no deadline. `SESSION` rejects old
  datagrams by content. The link script gains an expiry part and the in-memory link a stall.
- 2026-11-13 Must Fix, native link open needs wire progress: the link hello. The client writes it as it opens the
  stream, the host answers, and each end reports open on reading the other's, under a startup timeout. The script
  opens an idle link and takes the host's message first.
- 2026-11-13 Must Fix, session proof promised delivery after death: M1.5 and M2.9 are rewritten from delivery or
  closure. A link open at the end loses nothing; an ended link delivers an unbroken run from the first message and
  nothing after; the kill is the harness's note; the record must show sends that were interrupted.
- 2026-11-13 Suggestions, both taken: at its cap the outbound datagram queue discards the oldest and counts it
  apart from expiry, and the suspend rule is cited to the networking design, not C11.
- 2026-11-13 Planner's second probe, Quinn 0.11.12 on sockets. A stream nobody wrote to was never accepted, and a
  four-byte hello fixed it. On a cut path the link's queue expired all it held and at most three datagrams arrived
  late, 0.7 to 5 s old. BBR sent 273 to 451 KB into the cut before blocking, so the expiry part sends until the
  queue backs up, not for a fixed count.
- 2026-11-13 Also changed: the 90% floor counts the script's lanes and load parts only; M1.6 records connect to
  open; M2.7's guide and the module list's row and entry carry the two new limits on what `SESSION` may assume.
  Not run: Chrome or the simulator on a stalled path.
- 2026-11-13 Checks: no item's command changed, and each one without a lock still fails, as it should before the
  work exists. The script's dry run waited on a lock and was stopped, so the two verifier items were again not
  dry-run. `./dev check-docs` passes. The revised design has not been reviewed; no builder starts until a design
  review returns Design Approved.
