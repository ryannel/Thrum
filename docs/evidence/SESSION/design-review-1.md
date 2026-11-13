# SESSION design review

Reviewed 13 November 2026 at `8f038dec20027bd7c8c70809943556e536197718`.

**Design changes requested: four Must Fix findings.** The design covers the intended module: several players
share the swarm game over the network, with one authority, late joins, reconnects, commands and local prediction.
Its broad scope fits the human's request. The protocol has two unresolved correctness mechanisms, its digest
guarantee is stronger than its mechanism, and one acceptance item requires more slots than the match permits.

This is the first design review; no previous `design-review.md` was present. Findings below are verified against
the written design and interfaces, not observations of a built SESSION implementation.

## Must Fix findings

### 1. Must Fix — A set reports commitment before the authority commits

**Verified by design and existing runtime inspection.** This breaks the result semantics preserved by C13 and
leaves M2.5 unable to distinguish admission from successful execution.

[DESIGN.md](../../plans/SESSION/DESIGN.md), lines 86–99, sends an admitted set before the host steps. Lines
157–160 make that set the committed/rejected result; the host walk at lines 411–416 confirms the ordering.
Clients use the set to advance their confirmed world. But [C13](../../design/CONTRACTS.md), lines 655–666,
separates receipt from execution, and the [A5 timing decision](../../design/decisions/A5-INPUT-TIMING-RECONCILIATION.md)
explicitly reports committed origins only after a successful step. The proposed addition to C13 does not change
those semantics.

The underlying [runtime](../../../crates/thrum-core/src/sim/runtime.rs), lines 282–296 and 1256–1276, permits a
tick to fail without advancing the world. Its precommit failure path at lines 1557–1563 returns an error after
aborting tentative changes. Consider a host that sends set `n` and then fails that preflight, or stops before
committing. The client already has what SESSION calls the committed result for `n`, although the authority is
still at `n - 1`. No later success signal, cancellation or failure rule is specified. A decoder receipt or
identical deterministic rules cannot establish that the host actually committed.

**Required outcome:** define when a set becomes authoritative confirmation, how successful host commitment is
communicated, and what clients and input results do when the host step fails. If sets still travel before the
step, their early arrival must not by itself confirm execution. M2's result proof needs to distinguish this
case from an ordinary successful tick. This finding does not require changing the shared-input approach.

### 2. Must Fix — Receipt acknowledgements cannot detect a slow simulation

**Verified by a counterexample to the specified acknowledgement rule.** N7 and M3.5 require a client stepping
at half speed to be closed as unable to keep up. The specified mechanism does not observe that condition.

[DESIGN.md](../../plans/SESSION/DESIGN.md), lines 132–155, acknowledges the highest contiguous tick whose sets
the client **holds**, and retains only unacknowledged sets on the host. Lines 222–225 separately buffer those
sets before stepping them. Yet mechanism 10, line 339, says a client that steps too slowly fills its host-side
ring and thereby triggers repair and eventual closure.

A client can continue polling its link, authoring input and acknowledging every received set at 60 Hz while
committing only 30 ticks a second. With a three-tick delay each way, an abstract 600-tick walkthrough of those
rules leaves the host retaining at most six sets, while the client has received through tick 597, committed
through tick 299, and accumulated 298 unstepped sets. The host's ring never approaches overflow; the client's
backlog keeps growing. This is an ordinary slow simulation, not a malicious acknowledgement. Correct digests
for older ticks also do not communicate current progress under the specified protocol.

**Required outcome:** define a bounded client backlog and a way to observe and handle simulation lag separately
from receipt of sets. State which progress measure triggers repair or closure and how that interacts with
join catch-up. M3.5 must exercise a client that keeps receiving and acknowledging normally while stepping too
slowly; stopping its reads would test a different case. The walkthrough above models only the written queue
rules and makes no engine performance claim.

### 3. Must Fix — The promised divergence detection assumes differences cannot disappear

**Verified by a deterministic state transition counterexample.** N6 and the unconditional next-digest promise
need a detection guarantee the chosen mechanism can actually provide.

[DESIGN.md](../../plans/SESSION/DESIGN.md), line 75, covers any one value in the digest; lines 283–286 assert
that a difference, once made, stays because the digest covers everything a later tick reads. That conclusion
does not follow. [SIM-2D's digest](../../plans/SIM-2D/DESIGN.md#4-the-world-digest) hashes the committed endpoint,
not the history of endpoints.

For example, a registered authority resource can be assigned the current tick by a deterministic rule every
tick. Change its value on one client immediately after tick 11. Tick 12 overwrites it identically on both peers;
at the next scheduled digest, tick 30, the complete worlds agree. The same issue occurs when a held control is
replaced by the next admitted input. There was a real covered-state difference, but the sampled digest cannot
report it. A simple integer walkthrough of the resource example confirms equality at ticks 30 and 60 despite
the mutation after tick 11. No hash collision is involved.

**Required outcome:** make N6, mechanism 8 and M3.4 agree on the guarantee. Either explicitly scope detection to
differences that remain at a compared endpoint, with repair tests that preserve the injected difference until
comparison, or provide a mechanism that supports the stronger intervening-state guarantee. The human's request
does not itself require reporting a difference that has already disappeared, so narrowing this claim need not
expand the module.

### 4. Must Fix — The concurrent-join item requires five reserved slots

**Verified by the acceptance item's arithmetic.** [DONE.json](../../plans/SESSION/DONE.json), M3.2, starts with
three players, has a fourth join, and requires a second join started while the first is in transit to complete.
[DESIGN.md](../../plans/SESSION/DESIGN.md), lines 45–46, caps a match at four slots; lines 272–273 reserve a slot
at welcome. Three present players plus two simultaneous joining players require five slots. M2.6 also requires
a full match to refuse a hello.

**Required outcome:** give the concurrent-join proof enough free slots, or explicitly make the additional join
a full-match refusal. Keep the late fourth-player proof. As written, a builder cannot satisfy both M3.2 and
the capacity contract in the same scenario. A separate concurrent-join scenario is sufficient; no larger match
is requested.

## Suggestions

### 5. Suggestion — Exercise independent client clock origins

**Verified limitation of the planning evidence; implementation risk remains unproven.** Mechanisms 5–6 refer
to authoring ahead of the present and to a separate playout clock. The
[planning probe](planning-probe.md) authors at `now + lead` in one virtual-time model. It does not establish how
a late-starting process maps its local monotonic clock to the host's current tick, especially after a long
baseline transfer. The older networking design explicitly calls for estimating server time.

Clarify that mapping and include independent clock origins in the lead proof. This strengthens the existing
mechanism rather than requiring clocks to share wall time. I did not establish that the proposed feedback loop
must fail once a suitable mapping is supplied.

### 6. Suggestion — Distinguish expired reassociation from slot-number reuse

**Verified wording conflict.** M3.3 says a return after grace expiry, or without a token, “never gets the old
slot.” Mechanism 9 frees an expired slot, and a new player may take any free slot. Clarify that the former
player cannot reclaim its old association or character through an expired token; a new association may
legitimately reuse that numeric slot. This is consistent with N4's intended slot reuse.

## Request, coverage and proof assessment

I read the [project brief](../../plans/BRIEF.md), the task in
[RECORD.json](../../plans/SESSION/RECORD.json) and the
[owner's swarm-game request](../../design/NEXT-GOAL-SWARM-GAME.md) before the design. The sole recorded planner
task is “Plan module SESSION. It needs: SIM-2D, TRANSPORT.” It supplies no replacement for the human's request.
The resulting requirements are a reusable networked match for several players, the same crowd and ordered
gameplay consequences, and compatibility with the engine's authority and target contracts.

The default two-to-four-player scope, native host, confirmed crowd and predicted local character are explicit
assumptions in PLAN.md. Host migration, public hosting and matchmaking were not requested. The absence of
drawing here is an explicit module boundary: SWARM owns the playable game and the app-runner clock integration.
I found no reason to block those scope choices. The headless proof must not be reported as qualification of the
eventual game's visual responsiveness or of an internet path; the design and closing ledger item already say so.

I read DESIGN.md, PLAN.md and all 55 DONE.json items, along with the relevant module-list seams, SIM-2D's state
and rule contracts, TRANSPORT's link limits, C02/C03/C13, the networking design and A5 timing/controller records.
I also read the planning probe and inspected the existing runtime's step/refusal boundary. This is a design
reconciliation review with that code context, not a blind technique study. No delegation was used.

| Milestone | Can its proof be judged at that stage? |
|---|---|
| M1, 10 items | Yes. SIM-2D M3 supplies the retained traces; the seam ruling authorizes image encoding/restore. Cross-process continuation and unchanged retained digests reject a same-process capture substitute. |
| M2, 10 items | Initial baseline transfer, separate worlds and the second rule set make the proofs feasible after M1 and TRANSPORT M1. Finding 1 must resolve commitment before M2.5 can establish truthful results. |
| M3, 10 items | Its prerequisites are available. Findings 2–4 affect slow simulation, divergence detection and concurrent joining. The other lifecycle, malformed-input and roster proofs are judgeable. |
| M4, 7 items | The shared movement seam and SIM-2D cost record are available. Isolation, exact stored-state comparison and ring-exhaustion fallback reject a predictor that merely returns confirmed state. Visible feel remains SWARM's work. |
| M5, 10 items | TRANSPORT M2 supplies the sockets and relay. Separate target processes and peer-owned records reject in-process and substituted-target proofs. These establish loopback function; no physical-phone speed or internet qualification follows. |
| M6, 8 items | The guide, closure checks, second rule set and final reruns are available after M5. The ledger explicitly separates measured results from unrun targets and paths. |

The cost account separates estimates from future measurements and treats performance figures as tripwires.
The 95% admitted-input floor is a functional control against a broken lead, not a frame-time performance gate.
No additional performance gate is requested by this review.

## Verification limits

No SESSION product, sockets, browser, simulator or visual output was run. Its dependencies and implementation
are planned work. The two small abstract walkthroughs above test consequences of the written rules, not a
replacement implementation. Existing runtime code was read to confirm the failed-step interface. External
case-study claims were not independently requalified; none is needed to establish the four findings.

`./dev check-docs` was run and failed on existing links to removed `ACCEPTED.json` files in other evidence
documents, the same problem SESSION's planning log records. It reported no error in this review. All review
link targets exist; the four finding headings match the result count. No code or device checks were run.

Only this review file was changed. DONE.json and the design remain untouched.

DESIGN CHANGES: 4 must_fix findings
