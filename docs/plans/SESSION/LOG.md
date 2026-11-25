# SESSION log

- 2026-11-13 Planned from the lead's task (row 19 of the module list); no words of the owner's were passed on, so
  the design quotes the owner's page of 10 November. Six milestones replace the brief the module list carried,
  whose text is in git (`git show e119993a:docs/plans/ARCHITECTURE.md`). M1 waits on `SIM-2D:M3`, M2 on
  `TRANSPORT:M1` and M5 on `TRANSPORT:M2`.
- 2026-11-13 Planner's decisions, all three left to the planner by that brief. Rollback: the crowd is drawn at the
  confirmed tick and only a player's own character is predicted, because a correction of the whole crowd at
  10,000 bodies is about 50 ms against an 8.3 ms frame. Digests: every 30 ticks to start, a match setting from 1
  to 600, with the host's digest carried in the set stream and compared by each client. A modal choice: the offer
  is world state and the answer is a command on the reliable lane, admitted at one tick and sent in that tick's set.
- 2026-11-13 Planner's decisions the brief did not name. The admitted sets travel as datagrams that repeat what a
  client has not acknowledged, not on the reliable lane. The input lead is sized from the margin the host reports,
  not fixed. A world holds player slots and the session holds the roster, so a reconnect never changes world
  bytes. Every client starts from a baseline, the first included. The envelope stays at 64 bytes.
- 2026-11-13 Planner's probe, in [the evidence folder](../../../.orchestrate/evidence/SESSION/planning-probe.md): a timing model
  with no engine code. The lead settled at 4.6 ticks at A0 and 9.6 at 250 ms round trip with 5% loss; a lead fixed
  at five ticks missed every tick there. Sets on datagrams could be stepped 76 ms after the host committed them
  at A0, against 227 ms on a reliable lane. Two repeated frames in an input datagram recovered nearly all of A0's
  loss.
- 2026-11-13 Planner's findings for other modules. `thrum-core` restores a capture only into the runtime that made
  it, so the world image is new work there, given to this module's M1 by a seam ruling. The reference crowd's
  rules take no actions, so commands are proved on a second rule set kept with the tests. The engine's world lane
  steps by its own clock, so `SWARM`'s app runner needs one that steps when the session says: an ask in the
  module list. A0's proposed 64 kbit/s is exceeded by A5's input batch alone at 60 Hz.
- 2026-11-13 Not measured while planning, because `SIM-2D` and `TRANSPORT` are not built: the image's size
  (estimated at 1.3 MiB for 10,000 bodies), the engine's tick and digest, and every byte count. M1, M2 and M5
  record them.
- 2026-11-13 Researched: Factorio's lockstep, latency hiding and join (Friday Facts 149 and 302), Fiedler's
  deterministic lockstep, Unity Netcode for Entities' command-age feedback, Photon Quantum's prediction culling
  and the coherence case study of Vampire Survivors' co-op. Two assumptions moved here from the module list:
  that a native process hosts with no migration, and that every peer holds the whole arena.
- 2026-11-13 Checks: every item check without a lock parses as shell and fails, as it should before the work
  exists. The six verifier items were not dry-run, so as not to take the GPU; they are the command `SIM-2D` and
  `TRANSPORT` use. `./dev check-docs` fails, and did before this plan: older evidence files still link to
  `ACCEPTED.json`, which the move to `RECORD.json` renamed. It reports nothing in a file this plan wrote.
- 2026-11-13 The design has not been reviewed. No builder starts until a design review returns Design Approved.
- 2026-11-13 [Design review](../../../.orchestrate/evidence/SESSION/design-review.md): changes requested, four Must Fix findings
  and two suggestions. The plan was revised the same day; the next four entries say how.
- 2026-11-13 Finding 1, a set reported commitment before the host committed. The host now steps first and sends
  a set only for a committed tick, so a set's arrival is the confirmation. Cost: the host's tick, about 5 ms at
  10,000 bodies, joins every client's wait; M2.8 records it. A host step that fails ends the match with a
  match-ended message naming the last committed tick, which resolves unanswered inputs as not committed. New item
  M2.11 proves it against an ordinary tick. Sending early with a later commit mark was weighed and left out: it
  waits a whole tick longer, or lets a client's world run ahead of the authority.
- 2026-11-13 Finding 2, receipt could not show a slow simulation. A client now reports two ticks, held and
  stepped; their difference, the backlog, shows stepping lag with no link delay in it. A playing client whose
  backlog stays above the behind threshold for the catch-up deadline is closed as unable to keep up, and is not
  repaired. A joiner is judged by the join deadline until it is in step. The client's buffer has a cap. M3.5
  lost its half-speed case to new item M3.11, where the slow client keeps reading and acknowledging.
- 2026-11-13 Finding 3, the digest promise was too strong. Detection is now scoped to a difference still in the
  world at a digest tick; one the rules overwrite first is not reported, and an interval of 1 compares every
  committed endpoint. N6, mechanism 8, M3.4 and M5.2 to M5.4 say so, and M3.4 changes state the rules carry
  forward. The module list's line on the digest follows.
- 2026-11-13 Finding 4, M3.2 needed five slots. M3.2 keeps the late fourth player. New item M3.12 has two
  players joined by two more at once, a hello refused as full while both are joining, and a reservation freed at
  the join deadline. A reserved slot counts as taken.
- 2026-11-13 Suggestions, both taken. 5: the design says how a client places its authoring tick with no host
  clock, and M2.3 gives each client its own clock origin and one a different rate, because the probe shared one
  virtual time and a builder would otherwise have had to invent the mapping. 6: an expired or missing token makes
  a new player, who may be given the same slot number; M3.3 and mechanism 9 say so.
- 2026-11-13 Checks after the revision: the three new item checks parse and fail, as they should before the work
  exists, and `./dev check-docs` passes. The verifier items were again not dry-run. The revised design needs a
  new design review.
- 2026-11-14 Owner: "Let's move the 2d engine side of things to before v1 and decouple them. But we don't want to start working on the swarm game till v1 lands." SESSION waits for nothing of V1's. A driven world,
  engine work that `SWARM` M7 had carried under its ruling 4, is now mechanism 12 and a new M7 here, proved
  with the reference crowd in a window on the Mac and in Chrome; it waits for `RENDER-2D:M5`. M1 to M6 are
  unchanged. The sandbox assumption is deleted: the owner gave Codex full access. The revised design needs a
  design review.
- 2026-11-14 Third design review, of commit `5ea9bc40` ([the latest review file](../../../.orchestrate/evidence/SESSION/design-review.md)
  until a later one replaces it): changes requested, one Must Fix and two suggestions. The four earlier
  findings stay resolved.
- 2026-11-15 The Must Fix: catch-up after an underrun jumped at least two ticks, whatever the alpha. Mechanism 6 now
  uses one playout position that never moves back: it commits a tick only on reaching the one before, catches up at
  up to 1.1 (slow-down 0.85, Unity Netcode defaults), stalls on a missing set, and skips only above the behind
  threshold, recorded and shown. Host commitment unchanged. New need N12; new items M2.12 (old policy as failing
  control) and M7.5 (burst, underrun and skip told apart); old M7.5 to M7.8 renumber to M7.6 to M7.9.
- 2026-11-15 Planner's choice in that fix, both halves: continuity inside a bound, and a named jump beyond it. A
  glide alone would take ten ticks to regain one, too slow after a long stall or a hidden tab; extrapolating past
  the newest confirmed tick was left out, since C02 allows interpolation only. The bound reuses the behind
  threshold, so no new setting decides it. Mechanism 12 now has the driver state its position as the lane's
  clock sample does, so a frame is placed at its own display time and not at the last advance.
- 2026-11-15 Suggestions, both taken. 2: M7.4 says no tick is committed twice and no frame is drawn behind an
  earlier one; drawing the same pair again is legal, and M7.2 says so. 3: the no-window and no-GPU statements
  are scoped to M1 to M6, and the sandbox paragraph and the plan's stop rule follow the project settings (Codex
  has full access). No interface, seam ruling, contract or other module's file changed.
- 2026-11-15 Checks after the revision: every item check parses; the two new ones (M2.12, M7.5) fail, as they
  should before the work exists. The verifier items were not dry-run, so as not to take the GPU.
  `./dev check-docs` fails on ten older links in `CREATOR`, `CULLING` and `CLOUDS` files and on nothing here.
  The revised design needs a design review.
- 2026-11-18 Built portable images for registered worlds, with paired codecs and bounded validation before install.
  The Mac, Chrome Wasm and iPhone simulator each restored six Crowd and Mixed images and matched 1,000 retained
  checkpoints. Chrome and the simulator wrote bytes equal to the Mac's. Core tests, identity controls and the
  Mac SIM-2D conformance run passed. The [M1 proof](../../../.orchestrate/evidence/SESSION/M1.md) records target revisions and
  both Mac and Chrome cost runs. The exact-image verifier is provided by the shared main-branch check.
- 2026-11-18 M1.2 now runs both Chrome captures and all six restores under one Chrome lock after the Mac
  images are ready. The revised check matched all six images and 1,000 retained checkpoints in under
  22 minutes including lock wait. The [M1 proof](../../../.orchestrate/evidence/SESSION/M1.md) records the retry.
- 2026-11-18 M1's first [review](../../../.orchestrate/evidence/SESSION/M1-review.md) failed on M1.5, M1.6, M1.7 and M1.9, and
  sent the last to a planner. Diagnosis: the plan was at fault. M1.4 and seam ruling 1 have the compiler refuse
  a type with half the encoding, while the M1 note and M1.9 froze `SIM-2D`'s tests, whose fixtures in
  `crates/thrum-core/tests` register such types. No build passes both.
- 2026-11-18 Planner's ruling: one boundary. Profile registration and the resource hook ask for both halves,
  and an accepted `SIM-2D` test may change only to give its types the decoding half and follow the encoding
  interface. The fixture edits in `95ed6591b` and `6be22749e` are of that kind. A second registration for
  sendable types was left out: two kinds of rule set, and a later refusal. Mechanism 7, ruling 1, the M1 note
  and M1.9 say so; M1.9 now asks about assertions, inputs and expected values, not any edit.
- 2026-11-18 The M1.6 finding was a gap in the design: an identity derived at build time cannot tell Crowd from
  Mixed, which one run-time flag selects, and the digest does not hold the flag. Mechanism 9 now puts
  everything a tick reads in the world or in the identity, so the two are two rule sets. M1.6 was written
  again with both identities and the refusal each way. The flag could not join the world: `SIM-2D`'s retained
  digests would move.
- 2026-11-18 M1.5 was written again so a refusal is proved on a runtime that then installs the intact image;
  as it stood, a destination that refuses everything passed it. M1.7 is unchanged: the missed flag on the
  image-writing tick is a defect in `scripts/session_match.py` against the item as written. M1.1 to M1.4, M1.8
  and M1.10 are unchanged. Check commands are the approved ones and were not run again, so as not to take
  Chrome and the simulator. The revised design needs a design review.
- 2026-11-18 Bound the image identity to the configured Crowd or Mixed rules and verified refusal in both
  directions. Each untrusted-image fixture now rejects malformed input on a fresh destination that then
  accepts the intact control. All three targets matched the retained digests. The Mac cost record flags the
  increase in the 5,000-body write tick. The [M1 proof](../../../.orchestrate/evidence/SESSION/M1.md) records the target runs,
  SIM-2D fixture audit and the shared exact-image check.
- 2026-11-19 Built caller-clocked deterministic sessions with post-commit set delivery, baseline restore,
  adaptive input lead, ordered commands, receipt-based results, listen-host admission and bounded playout.
  The Mac proofs matched all 1,200 digest checkpoints on four independent clients at both link profiles.
  The command, failure, negotiation and playout controls passed. Separate release cost runs recorded engine
  and session time under load and flagged the 64 kbit/s peer bandwidth tripwire. [M2 proof](../../../.orchestrate/evidence/SESSION/M2.md)
- 2026-11-19 Addressed the M2 review findings. The Mac checks passed for expired hellos, full-round-trip
  adaptive startup, the carrier observer, negotiated playout rates and complete stall time. The lead control
  stayed below the admission floor. Both full matches agreed at all digest checkpoints and recorded costs
  by negotiated slot, with a whole image-writing tick and comparisons under load.
  [M2 review fix proof](../../../.orchestrate/evidence/SESSION/M2-review-fix.md)
- 2026-11-19 Added concurrent slot reservations, token-based return, tick-committed roster events, same-link
  repair, and bounded peer retention with distinct synchronization, absence and catch-up deadlines.
  The Mac checks matched the independent digest traces through late and overlapping joins, link faults and repairs.
  Two release runs at each crowd size recorded image and catch-up costs under load. The reconnect record shows
  replayed old-epoch bytes released by a carrier stall after the new epoch. [M3 evidence](../../../.orchestrate/evidence/SESSION/M3/README.md)
- 2026-11-19 Held-control expiry now neutralizes axes when a staged frame is still due. The Mac slow-peer run
  observed neutral axes at tick 114 and absence at 332, with the other replicas still matching. Set batches
  validate contiguous receipts before publishing progress. Mac tests cover payload prefixes on both set lanes
  and a 196,813-byte image refused within a 64-byte storage cap. [M3 evidence](../../../.orchestrate/evidence/SESSION/M3/README.md)
- 2026-11-19 Reliable set delivery now closes a peer only if a required send still reports a full queue at
  the deadline. In the Mac regression, one peer drained its queue, returned to datagrams on the same link
  and matched the authority past the old deadline. A peer that kept the lane full closed for synchronization
  at the deadline. [M3.7 proof](../../../.orchestrate/evidence/SESSION/M3/M3.7-reliable-delivery.md)
- 2026-11-20 Added a client-owned predicted controller with a 64-tick history, exact reconciliation and adjacent presentation endpoints.
  Both 36,000-tick Mac runs matched every retained admitted prediction, traced corrections to held frames, and recovered from a stall, repair and reconnect.
  The Mac reproduced both retained SIM-2D traces. The landed implementation's four release cost runs recorded the machine load.
  The shared exact-image check passed both targets on a main revision that contains the implementation.
  [M4 evidence](../../../.orchestrate/evidence/SESSION/M4/README.md)
- 2026-11-20 Kept the confirmed controller through a post-stall skip and through gaps between recovery batches.
  A committed digest disagreement now refreshes the fallback on the same poll. A refused step keeps the prior endpoint.
  Both 36,000-tick Mac release runs verified the public controller throughout recovery. Four release cost runs
  refreshed the measurements under load. [M4 evidence](../../../.orchestrate/evidence/SESSION/M4/README.md)

- 2026-11-23 Planning archive. The [SESSION planning probe](../../../.orchestrate/evidence/SESSION/planning-probe.md)
  modelled timing. Its source was discarded. The M1–M4 reviews hold the implementation receipts.
- 2026-11-23 M5: the SESSION build ran `lock simulator -- xcrun simctl bootstatus <UDID> -b` at 09:54 CET.
  The [boot log](../../../.orchestrate/evidence/SESSION/M5/simulator-boot.log) identifies the stock iPhone 18 Pro
  and the lock log records release at 09:54:05. The socket peer helper boots or reuses that phone and has no
  shutdown path. SIM-2D's helper, also used by SESSION's iOS image check, has the same gap. HOST and
  APPLE-PRESENTATION normally use temporary devices and attempt to remove them. M5 adds one shared lifecycle
  and a post-run device-state proof.

## M8 workspace timing baseline

- 2026-11-23 These are the cases from the passing `747909d1c` workspace check, with seconds from that run.

| Seconds | Test |
|---:|---|
| 9.102 | `thrum-network::it reliable_delivery::a_peer_that_drains_the_reliable_lane_returns_to_datagrams_and_stays_in_play` |

## M5 simulator lifecycle design review

- 2026-11-23 Must Fix 1. CULLING M2.2, COOKED-DRAW M1.7–M1.9, PLATFORM-RUNTIME-IOS M7.2 and the published
  T2 capture command now declare `simulator`. M5.11 requires that lock through verified shutdown,
  including across the LaunchServices handoff. The five Done items keep their original proof records.
- 2026-11-23 Must Fix 2. The affected SESSION command is `image --target ios-sim` (M1.3). Its eight accepted
  device operations alone took 1,872.5 seconds. The design makes each writer, restore and
  conformance case its own bounded turn and keeps the live peer restart on one boot. M5.11 guards the turn
  boundary and teardown reserve, M5.12 checks real cleanup, and M5.13 records all six restores and 1,000
  checkpoints on the migrated path. [Review](../../../.orchestrate/evidence/SESSION/design-review.md)
- 2026-11-23 Took the suggestion to define a fresh start on the reused device. The design and M5.12 cover
  app, proof and XCTest state after cancellation. The T2 guide describes verified shutdown and reuse after M5.
- 2026-11-24 Final design review, Must Fix 1. HOST M4.5 keeps its single-definition assertion and execs the
  HOST command. APPLE-PRESENTATION D6.7 execs the presentation command and its route deadline goes through
  supervised cleanup. M5 defines which PID the runner waits for, the ten-second grace and recovery before the next
  device use. M5.14 exercises outer cancellation and the presentation deadline with the installed runner,
  a queued contender and an early-release control. [Review](../../../.orchestrate/evidence/SESSION/design-review.md)

## M8 reliable-lane fixture

- 2026-11-24 Kept both reliable-delivery cases in the fast tier with one link and an unpadded integer world.
  On the Mac, recovery resumed datagrams on the same link and matched the authority digest at stop. The
  persistent-full control closed at the deadline. The first two-case workspace batch passed at 0.032 s and
  0.060 s. The individual runs, baseline and assertion audit are in the [M8 evidence](../../../.orchestrate/evidence/SESSION/M8/README.md).
## M5 socket sessions

- Added release entry points for the Mac authority and replica, the simulator replica and the Chrome Wasm
  replica. Each runs the registered crowd through the production session and its target's transport adapter.
  The harness drives A0 packet relays, records process and build identity, and compares digests written by the
  independent worlds. Native 10,000-body runs use a bounded rule worker pool. The socket profile records its
  catch-up and drain settings, plus the requested Darwin scheduling class.
- Verified the Mac, Chrome and simulator fault sequences, the Mac stalled-client control, both Chrome renderer
  freezes, and the Mac listen host with Chrome and simulator replicas. The cost campaigns retained both repeats
  for BBR and Cubic at 2,000 and 10,000 bodies, with initial lobby join phases and a playing interval. The Chrome
  endpoint wire counter is absent and the relay counts both wire directions.
- Real socket runs exposed missed authoring periods during delayed client polls and receive-credit exhaustion
  during delayed authority steps. The session now bounds its authoring horizon, offers timed input before
  replica catch-up and earns receive credit in elapsed tick periods. The focused network tests and adaptive
  lead regression passed. [Socket evidence](../../../.orchestrate/evidence/SESSION/M5/README.md)


- 2026-11-24 Bounded the socket authority's catch-up rate and verified the 36,000-tick match with four independent replicas.
  Each matched all 1,200 digests. The three fault targets, both stalled-peer checks, the listen host and both cost matrices passed.
  The shared simulator lifecycle verified shutdown after success, failure and cancellation. All eight image turns released after shutdown,
  with six byte-exact images and 1,000 matching continuation checkpoints. The scoped workspace check passed all 60 selected tests.
  The merged tip receives the shared exact-image check. [M5 evidence](../../../.orchestrate/evidence/SESSION/M5/README.md)

- 2026-11-24 The merged steady run matched all digests, but Chrome admitted 93.87% of warm inputs.
  A diagnostic recorded browser wakeups on one-second boundaries and a close for absence. The headless
  proof now disables background timer throttling. The 36,000-tick rerun matched all 1,200 digests on each
  replica, with warm admission between 96.69% and 100%. Chrome's fault and freeze scenarios, the listen-host
  match, and all eight Chrome cost rows also passed. [M5 evidence](../../../.orchestrate/evidence/SESSION/M5/README.md)

- 2026-11-24 The next clean run showed a Chrome replica backlog and a host close for falling behind.
  Socket builds now optimize across crate boundaries, and Chrome yields between replica steps.
  A clock regression reproduced skipped future input targets after a delayed poll at the staging limit.
  The clock now keeps those targets consecutive. The steady run matched every digest on all four replicas,
  with 99.73% to 100% warm admission. The other eight socket cases and the adaptive-lead regression passed.
  [M5 evidence](../../../.orchestrate/evidence/SESSION/M5/README.md)

- 2026-11-24 The merged steady run showed Chrome receiving sets while its replica fell behind. The grid now packs
  circles by cell, sorts occupied cells for wide queries and selects the circle predicate before traversal.
  The 36,000-tick release match matched all 1,200 digests on each of the four peers and admitted every warm input.
  The isolated outer runner now stages the checkout's Python modules. Both cancellations and the deadline verified
  shutdown before the contender entered. The workspace check and branch-runnable milestone checks passed.
  The iPhone image proof matched six images and 1,000 checkpoints across eight separately verified shutdowns.
  [M5 rerun evidence](../../../.orchestrate/evidence/SESSION/M5/README.md)

- 2026-11-24 The WP5 review rerun used the seeded script for each slot. All four replicas matched all
  1,200 host digests over 36,000 ticks, and the authority committed four distinct sampled inputs at
  1,180 warm ticks. Both cost campaigns passed all eight rows. The host kept committing through each
  transfer, and each client replayed through the host's tick at restore. The records retain the first
  Playing transition, shared interpolation phase, and byte counters with their measurement windows.
  The simulator proof fixtures now read the migrated plan names. [Target evidence](../../../.orchestrate/evidence/SESSION/WP5/review-fix/README.md)

## WP6 game-maker guide and headless proof

- Added the [game-maker guide](../../SESSION.md), copied the deterministic profile wording into C13
  verbatim and documented how SESSION uses the A0 and A5 decisions. Recorded N1's implementation,
  correctness, device scope and unrun paths in the maturity ledger.
- The V1 game, starter and creator scenes omit both networking crates. The in-memory proof reaches
  neither socket adapter's added dependencies. The reconnect token now uses the already locked
  entropy provider version that predates TRANSPORT.
- On the Mac, all 31 integer/session tests passed. At A0 and on the harsh profile, all four in-memory
  replicas matched 1,200 digests each. The headless socket match repeated the same digest count on
  two Mac clients, Chrome and the iPhone simulator over A0-conditioned loopback. The simulator
  verified shutdown. [Builder evidence](../../../.orchestrate/evidence/SESSION/WP6/README.md)

- 2026-11-25 WP6 review fixes. The guide now drains frame and command results after every client advance
  and documents game-owned proof programs for both carriers. On the Mac, the timed-only control reproduced
  the 8,192-frame stall. Draining results kept the client Playing through tick 9,500. The CLI ran the
  integer rules over memory and Mac sockets with matching digests, and successfully launched supplied
  Mac, Chrome and simulator reference programs. [Review evidence](../../../.orchestrate/evidence/SESSION/WP6/review-fix/README.md)

- 2026-11-25 The clean socket check recorded a simulator replica going Behind and returning to
  Playing on the same link, epoch and slot. It reached the closing tick with every digest equal.
  The seeded-input check now accepts those Playing transitions and verifies their identity before
  comparing the sampled input. [Closing checks](../../../.orchestrate/evidence/SESSION/WP6/steady-recovery/README.md)
