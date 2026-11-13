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
