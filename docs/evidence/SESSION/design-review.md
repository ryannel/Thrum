# SESSION design review

Reviewed 13 November 2026 at `00949955de5a5610d25cc5ce3a7fd55c50eaa9b6`, against the previous review at
`8f038dec20027bd7c8c70809943556e536197718`.

**Design approved. All four previous Must Fix findings are resolved; no new Must Fix findings.** The design
provides the match layer needed for several players to share the swarm game over the network. Its authority,
join, recovery and prediction mechanisms fit that request and the module boundaries. This approves the design
and its planned proofs, not an implementation or the eventual game's visual responsiveness.

## Previous Must Fix findings

### 1. A set reports commitment before the authority commits — resolved

**Verified against the revised design and runtime interface.** [DESIGN.md](../../plans/SESSION/DESIGN.md),
lines 88–114, now sends a set only after the host commits its tick. The host walk at lines 488–495 follows that
order, including taking the digest before transmission. This agrees with C13's separation of receipt and execution
and the [A5 timing decision](../../design/decisions/A5-INPUT-TIMING-RECONCILIATION.md).

The failed-step path is now explicit: no set for the failed tick, no retry, and a reliable match-ended message
after the outstanding committed sets, naming the last committed tick. Clients resolve the remaining inputs as
not committed on that orderly end; a vanished host leaves unanswered inputs uncertain. Mechanism 9 retains the
answer-before-close timeout, consistent with TRANSPORT's delivery-or-closure contract.

M2.5 checks the send/commit ordering. New M2.11 requires an actual runtime refusal, a successful control with
the same inputs, no client advancement into the refused tick, final digest equality, and truthful results for
unanswered inputs. Withholding a packet in the harness would not satisfy its question. I checked
`crates/thrum-core/src/sim/runtime.rs`, lines 285–296 and 1557–1573: the refusal and rollback boundary needed
for this proof exists. M2.8 accounts for the added host-step latency rather than retaining the old cost claim.

### 2. Receipt acknowledgements cannot detect a slow simulation — resolved

**Verified against the revised queue and progress rules.** DESIGN.md, lines 155–161 and 180–189, separates
`held` from `stepped`, retains sets by receipt, and caps the client's buffer. Mechanism 10, lines 403–420,
uses their difference to detect a client that receives promptly but cannot step fast enough. Sustained lag
closes a playing client; synchronising and repairing clients instead have the join deadline. A baseline is no
longer proposed as the cure for a persistently slow simulation.

The old counterexample now triggers the new mechanism. In an abstract ideal-link walkthrough starting with
zero backlog, one received set per host tick and one client step every two ticks, backlog first exceeds 30 at
host tick 61. With the specified “longer than 600” deadline, closure is due at tick 662 with backlog 331, while
the host's unacknowledged ring is empty. This is queue arithmetic, not a device measurement; it assumes a buffer
large enough to reach that deadline. Smaller buffers have the separately specified refusal-to-acknowledge path.

New M3.11 requires precisely the formerly invisible case: full-rate reads, acknowledgements and input with
half-rate stepping. It also requires a recovering client to remain connected and a joiner to use the join
deadline. M3.7 checks the client's cap and overflow outcome; M3.8 records backlog and time above the threshold.
M3.5 retains the distinct stalled-link and silent-client cases. Together these proofs distinguish receipt lag,
stepping lag and temporary catch-up without replacing one with another.

### 3. The promised divergence detection assumes differences cannot disappear — resolved

**Verified against the revised guarantee and SIM-2D's endpoint digest.** N6 and mechanism 8, DESIGN.md lines
330–347, explicitly limit detection to differences present at a compared committed endpoint. They acknowledge
that a difference overwritten before comparison is not reported, including the possible incorrect presentation
between comparisons. This matches [SIM-2D's digest](../../plans/SIM-2D/DESIGN.md#4-the-world-digest).

M3.4 now injects state the rules carry forward and requires the differing digest at the comparison tick. It
retains component, free-list and resource coverage, repair on the same link/slot/epoch, repeated-fault closure,
and intervals 30 and 1. M5.2, M5.3 and M5.4 carry the same persistence condition into their target-specific socket
proofs. M6.1 makes the limitation part of the guide. The earlier overwrite counterexample is now an expressly
excluded guarantee, not a promised result this mechanism cannot produce. The human did not request detection
of a difference that has already disappeared, so this narrowing does not remove requested functionality.

### 4. The concurrent-join item requires five reserved slots — resolved

**Verified by the revised scenarios and reservation rule.** M3.2 keeps the late fourth-player proof with three
existing players. New M3.12 starts with two players and two free slots, then requires overlapping joins into
those slots. It separately requires a further hello to be refused while both reservations are occupied, and
an expired reservation to become available. DESIGN.md, lines 321–326, defines the matching capacity and
catch-up rules. Two held slots plus two reserved slots fit the four-slot limit and agree with M2.6's full-match
refusal. The proof no longer requires a fifth slot.

## Previous Suggestions

Both were taken and are resolved in the design and planned proofs.

- **Independent client clock origins:** mechanism 5, lines 242–256, now places the authoring tick from recently
  received sets and handshake timing, rather than a shared clock or a stale welcome tick. M2.3 requires distinct
  clock origins, a different clock rate, the admission floor, and reported lead beside true lead. It excludes
  passing host time or the conditioning profile into the client. This supplies the missing mapping; the
  implementation must still demonstrate that its feedback converges.
- **Expired reassociation versus slot-number reuse:** mechanism 9, lines 381–385, and M3.3 now distinguish a new
  player association from reuse of a free numeric slot. An expired token grants no claim on the old character.

No additional Suggestions are raised.

## Every changed acceptance item

I compared the done-when file with `8f038dec2002`. The following are all 16 changed or added items; none was
removed. Each revised statement and its check/question is judgeable with its milestone and preceding work.

| Item | Assessment against the previous review |
|---|---|
| M2.3 | Strengthens Suggestion 5 with independent origins and clock drift; keeps adaptive-lead and pinned-lead controls. |
| M2.5 | Resolves finding 1's result-ordering gap by inspecting the actual send after commitment. |
| M2.8 | Records the cost of finding 1's fix as a baseline, without adding a performance gate. |
| M2.11, added | Exercises the runtime failure boundary behind finding 1, with a successful control and observable client results. |
| M3.2 | Resolves finding 4's impossible capacity arithmetic while preserving late fourth-player coverage. |
| M3.3 | Resolves Suggestion 6 without weakening new-epoch, uncertain-command or returning-player checks. |
| M3.4 | Aligns finding 3's injection with the endpoint guarantee and retains real repair and repeated-failure coverage; adds client-step refusal inspection. |
| M3.5 | Preserves receipt stalls, non-reading clients and silence; the half-speed case moves to M3.11 rather than disappearing. |
| M3.7 | Extends bounded-storage inspection to the client buffer required by finding 2. |
| M3.8 | Adds backlog and catch-up-duration evidence for finding 2's thresholds, with figures remaining baselines. |
| M3.11, added | Rejects the exact receipt-only substitute in finding 2 and distinguishes permanent lag from recoverable backlog. |
| M3.12, added | Proves concurrent reservations, full refusal and reservation expiry within finding 4's four-slot constraint. |
| M5.2 | Mac socket repair now injects a persistent difference, consistent with finding 3. Its lifecycle proof remains. |
| M5.3 | Chrome socket repair receives the same correction; it still requires the Chrome client and reload/reconnect path. |
| M5.4 | Simulator socket repair receives the same correction; it still requires that target's process restart and recovery. |
| M6.1 | Exposes both the host-failure policy and sampled-digest limitation to game makers. |

The whole file has 58 items across six milestones. I read the unchanged items as context as well. M1 can prove
portable images against SIM-2D M3's retained traces. M2 can use M1's image and TRANSPORT M1's conditioned link;
its small second rule set supplies commands and runtime refusals that the reference crowd lacks. M3 builds on
those mechanisms. M4's controller isolation and stored-state comparisons remain intact. M5 explicitly waits
for TRANSPORT M2 and keeps separate processes, target-owned digests and real socket paths. M6 checks the guide,
contracts, dependency closures and final reruns. The revision introduces no proof that depends on a later
SESSION milestone.

## Fit to the human's request

The [project brief](../../plans/BRIEF.md) and [swarm-game request](../../design/NEXT-GOAL-SWARM-GAME.md) require
several players playing together over the network, reusable engine capabilities, and shared authoritative
rules. I read both tasks in [RECORD.json](../../plans/SESSION/RECORD.json), oldest first: the original module
planning task and the request to resolve four findings. Neither is a new human requirement or permission to
narrow the original goal.

The design covers a shared crowd, ordered player choices, joining and returning to the run, correction of a
wrong copy, and local responsiveness through a predicted controller. It preserves one authority and leaves
game rules in the game. The native host, two-to-four-player default, confirmed crowd, lack of host migration,
and loopback-only qualification remain explicit assumptions. SWARM still owns the playable game, drawing and
app-runner integration; headless SESSION proofs do not establish how the eventual game feels. These boundaries
were accepted in the previous review and the revision does not expand or silently narrow them.

The performance account remains appropriate to the brief: it identifies repeated work and bounded storage,
accounts for the added host-step wait, and requests measurements with tripwires. The unchanged 95% input
admission floor checks a functioning lead; it is not a device-speed gate.

## Verification and limits

This was an independent design reconciliation review, with no delegation. I read the full revised design,
plan and done-when file, the prior review, the task history and planner's response; checked the relevant module
boundaries, C13, A5 timing, SIM-2D state/digest contract, TRANSPORT lane/closure/bounds contract, and the runtime's
refusal boundary. The abstract lag walkthrough above checks the new rule's consequence only.

No SESSION implementation, socket match, browser, simulator, GPU or visual output was run. The proposed checks
remain future implementation obligations. Unchanged external case-study claims were not independently
requalified; this verdict rests on the local contracts and the specified mechanisms and proofs.

`./dev check-docs` passed: 785 Markdown files, 5,155 local links, 30 module records, crate policy and current
design hashes. It emitted existing file-size notes, with no errors. Only this review file is changed; the
design, DONE.json and all other repository files are untouched. No captures were needed.

DESIGN OK
