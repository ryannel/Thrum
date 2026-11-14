# TRANSPORT log

- 2026-11-14 Target diagnostics found and fixed close ordering: retain the native WebTransport session until its
  CONNECT close capsule completes, and let the browser's session-close observer supply the peer's code before a
  stream error wins. A direct Chrome coded close now passes. The reconnect harness waits for the host to observe
  the old close before opening against its four-link cap; a live conditioned-host control test passes. Chrome
  shutdown now terminates its owned process group. Allocation probing under A0 measured one 189-byte caller-thread
  allocation per warmed reliable send/drain. These development checks are not promoted milestone evidence.

- 2026-11-14 Builder checkpoint: added the dependency-free bounded link, caller-clock conditioned carrier,
  versioned tickets, optional pinned Quinn/WebTransport carriers and shared Rust proof tooling. Four virtual-time
  tests and strict native/Wasm Clippy pass. Real development exchanges reached Chrome 157 and the iOS 27.2
  simulator; these dirty-tree runs are diagnostics, not M1 acceptance evidence. Final target proofs remain to run.
- 2026-11-14 The repository check currently fails on pre-existing absent evidence links in CREATOR, CULLING and
  CLOUDS. Those files are also absent on fetched main `2df2e3c5`; no evidence or acceptance claim was fabricated to
  silence them. Transport-specific dependency-policy issues found during construction were fixed.

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
- 2026-11-14 Owner: "Let's move the 2d engine side of things to before v1 and decouple them. But we don't want to start working on the swarm game till v1 lands." M1 no longer waits for `SHOWCASE:M8` and waits for
  nothing. The assumptions about waiting for V1 and about asking to leave the Codex sandbox are deleted: the
  owner gave Codex full access the same day. No item and no word of the design changed, so its approval stands.
- 2026-11-15 Re-plan after M1 ended `BLOCKED needs_dependency`: the builder found M1.10 false before this module
  existed, and [its handoff](../../evidence/TRANSPORT/M1-handoff.md) asked `CONTENT-LOOP` to gate its tests or the
  planner to settle the scope. The plan was at fault: it said nothing in the repository opened a socket, so
  M1.10 claimed the whole of `./dev check`. The cooker's loopback dev-server tests have run there since 6 November.
- 2026-11-15 Planner's ruling, in [the module list](../ARCHITECTURE.md#build-order-after-the-v1-exit): the module
  that owns a test decides whether it runs in `./dev check`, and there is no repository-wide ban on sockets.
  `CONTENT-LOOP` changes nothing. Not chosen: gating its tests, which takes the dev server's origin checks out of
  the only gate and breaks `TEST-SUPPORT`'s promise that every test that ran still runs; a rule in `REPO-RULES`.
- 2026-11-15 M1.10 is written again as this module's claim: the Wasm lint, and no socket, Chrome or simulator in
  what it adds to `cargo test`'s defaults or `./dev check`. The design's sandbox paragraph is rewritten to match,
  with the full access of 14 November, and "What exists" names the dev server. No other item or check changed.
- 2026-11-15 Measured while planning: `cargo test -p thrum-cook --test serve_and_watch` passes its four loopback
  tests in 5.3 s. Left as they are: `SWARM` M7.10 and a sentence of `SESSION`'s design carry the old wording,
  which the module list now reads as each module's own additions; reword them when those plans next change.
- 2026-11-15 The changed design and M1.10 have not been reviewed; no builder resumes M1 until a design review
  returns Design Approved. `./dev check-docs` passes with these files.
- 2026-11-15 Design review of `4e9956d6` (commit `d686fe75`): design changes, one Must Fix and one suggestion
  carried forward. The socket-test ruling and M1.10 stood.
- 2026-11-15 Must Fix, transport start conflicts with the approved brief: M1 waits for `SHOWCASE:M8` again. The
  brief's decision 1 says "transport stays after V1", only the owner changes the brief, and the owner's words of
  14 November name "the 2d engine side of things". The edge is scheduling, not use. Not chosen: rewording the
  brief, which is the owner's page. The design says when the module is built, the plan carries the question as
  an assumption, and the module list's split, edge table, build order, row 21 and owner decision 7 follow.
- 2026-11-15 Suggestion, shorten the expiry wording: taken, since the design was open anyway. The script's
  expiry part now lists what must be seen, with Chrome's exception beside it, and leaves how much to send to the
  builder. Nothing asked changed: expiry with sending stopped, the three sender counts kept apart from loss and
  lateness, some expiry on Quinn and the in-memory link, none required in Chrome.
- 2026-11-15 No item's statement, check or question changed; M1's `depends_on` did. `SESSION`'s files are
  unchanged, and its M2 onward now wait for V1 through `TRANSPORT:M1`. The work on `build/transport` stays for
  the builder that resumes M1. The design has not been reviewed again; no builder starts until Design Approved.
- 2026-11-15 Owner, on transport's timing: "it can come before V1 if we have space for it, but it's not urgent".
  This overturns the planner's assumption of `c7882c36` that the brief's decision 1 keeps transport after V1.
  M1 no longer waits for `SHOWCASE:M8` and waits for nothing; the assumption is deleted from the plan.
- 2026-11-15 Priority, from the lead with that ruling: `TRANSPORT` is the lowest, and the lead starts its builder
  only when a builder slot is free. The design's "When it is built" says so, with the owner's words. The brief's
  decision 1 and its "Not part of this" list, and the module list's row 21, split, edge table, build order,
  entry and owner decision 7, carry the ruling. Transport is still no part of the V1 exit.
- 2026-11-15 No item's statement, check or question changed; M1's `depends_on` and one paragraph of the design
  did. `SESSION`'s files are unchanged: its M2 onward wait on `TRANSPORT:M1` alone, not on V1. The work on
  `build/transport` stays for the builder that resumes M1, as the plan's note says. The changed paragraph has
  not been reviewed; no builder starts until a design review returns Design Approved.
