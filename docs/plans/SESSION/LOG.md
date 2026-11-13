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
- 2026-11-13 Planner's probe, in [the evidence folder](../../evidence/SESSION/planning-probe.md): a timing model
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
- 2026-11-13 [Design review](../../evidence/SESSION/design-review.md): changes requested, four Must Fix findings
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
