# SESSION M2 review fixes

The target was an Apple M3 Max on macOS 26.6.2. The checks and release runs used source commit
`bd10cd5912de4cd013dd666222f86d567eed7d20`. The release records include the executable hash, compiler, source
state and system load. [Mac test receipts](M2/mac-review-fix-checks.txt)

- **M2.3, input lead.** At a 100 ms handshake round trip the first adaptive target is the newest set tick plus six
  ticks for the round trip plus five for the starting lead. The pinned control sets five relative to the
  half-trip host estimate used to report lead and disables margin adjustments. Across the four adaptive
  scenarios, every player admitted at least 96.88% after warm-up. The best result among the pinned players was 80.92%.
  The steady true lead averaged 3.00, 5.50 and 10.62 ticks on local, A0 and harsh respectively. It rose and fell
  in the changing run. Client 2 used its own clock at 100 ppm over the host. The record joins admissions to
  players by negotiated slot, including reordered hellos, and retains the authority's counts by slot.
  [Lead runs](M2/memory-lead-review-fix.json)
- **M2.6, negotiation.** The host expires the current phase before draining queued messages. A valid hello
  at 3,999 ms is welcomed. At 4,000 and 4,001 ms it closes without a slot. The A0 check still uses two real rule
  sets and keeps the host link open until a refusal reaches the client. The session protocol version advances
  to 2 for the welcome's two new rate fields.
  [Negotiation tests](../../../crates/thrum-network/tests/negotiation.rs)
- **M2.8, costs.** The A0 and harsh runs each compare 1,200 digests on each of the four independently restored
  clients. The proof attributes held ticks to each player's repeat depth through its negotiated slot. On warmed
  tick 1,801 it measures the ordinary authority advance and the real world-image writer in one timed interval,
  and reports the engine step and serialization within that interval. The probe writes the image of the
  running match without adding a joining player to the four measured paths. The command compares equivalent
  rows against the most recent comparable record, attaches both loads, and flags a rise above 25%. A control
  setting only the new mean session time to twice its retained value flagged that row on both profiles. The
  original held counts and stall times cannot serve as comparison rows because their measurements changed.
  [Cost runs](M2/memory-costs-review-fix.json)
- **M2.9, caller-driven session.** The carrier collects losses during queue processing, drops the sender's
  lock, then calls the observer with the original bytes. The regression test polls the sending endpoint
  from inside that observer. The caller supplies session time, the link dependency has no socket features,
  and the authority encodes one committed set for all peers.
  [Loss observer regression](../../../crates/thrum-transport/tests/link_script.rs)
- **M2.12, playout.** A client sampled at its held position throughout an interval without client advances
  resumes at that position and reports the full interval through the recovery poll. The buffered, underrun and
  skip assertions run at the default and at negotiated 1.175 and 0.925 rates, using samples between advances.
  The test requires the negotiated catch-up rate and refuses eager commitment with the same motion checks.
  A separate clock test exercises the nondefault slow-down rate.
  [Playout tests](../../../crates/thrum-network/tests/playout_position.rs)

The image-writing tick took 3.93 ms at A0 and 3.97 ms on the harsh profile under load. On the harsh link,
depth one belonged to slot 1 and held 3,339 ticks, while depth two belonged to slot 0 and held 949.
Both profiles crossed the bandwidth tripwire in seven peer directions. The comparison was applied to
the completed measurements at `ee8d94a21` using the original retained cost record. Each profile compared
103 rows, with none more than 25% above its prior value. Workspace input changes altered the wire rule
identity without changing the workload. Both identities are recorded with the comparison.
