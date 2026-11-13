# SWARM design review

**Design approved. No Must Fix findings or new Suggestions.** The design provides the human's requested original swarm survivor, reusable engine support for 2D, and cooperative play over the network. Both Must Fix findings from the previous review are resolved without weakening the required outcomes.

Reviewed on 13 November 2026 at `f0a23e96738f6623cf4974e22bbc205aee72cb11`, against the review at `b7fbe03e9d36faebaac6a58060bec72e1a0abbaf`. This approves the design and its acceptance criteria; it does not qualify implementation, visuals or device performance.

## Previous Must Fix findings

### `run-end-priority-precedes-damage` — resolved, verified

[DESIGN.md](../../plans/SWARM/DESIGN.md), lines 171–200 and 555–563, now gives all three events one endpoint and places adjudication after the tick's other rules. The final rule reads staged boss death, hero standing/presence and the counted-down clock, then stages the outcome for the same commit. The priority remains boss death, defeat, then deadline. It applies to an ordinary wave's deadline as well as the last wave's. The last slot leaving explicitly overrides this with a return to gathering.

This can use the existing engine boundary. [TickGame](../../../crates/thrum-core/src/sim/runtime.rs), lines 298–307, supplies shared mutable candidate scratch. The runtime passes that candidate through callbacks, validates after preparation, applies the edits and captures committed state afterwards (lines 1444–1616). A later rule can therefore read game-owned staged results without reading a future committed world or adding another commit. [EditWriter](../../../crates/thrum-core/src/sim/transaction.rs), lines 30–35, confirms that queued operations cannot be removed, while their candidate payloads remain late-bound.

The revised cleanup respects that boundary: the first tick beginning in intermission, won or lost removes enemies and shots, including anything spawned on the deciding tick. Combat and spawning are gated by the state at the tick's start, so those remaining bodies cannot deal damage after the outcome. This is a one-tick delay in field cleanup, not a delay in deciding or publishing the result. M3.13 still judges the transition's visible clarity.

The written sequence now produces the required ordinary cases:

| Events at the tick endpoint | Outcome committed on that tick |
|---|---|
| Boss dies and last standing hero falls | Won |
| Boss lives, last hero falls and final clock reaches zero | Lost |
| Boss lives, a hero stands and final clock reaches zero | Won |
| Last hero falls as an earlier wave's clock reaches zero | Lost |
| A hero stands as an earlier wave's clock reaches zero | Intermission |

These are walkthroughs of the specified mechanism, not executed game tests.

**M3.17:** its revised statement and reviewer question require the deciding event and outcome in the same committed state. The existing deadline, lethal-contact and boss-death cases remain; reading a later tick cannot satisfy the question. The longer-than-36,000-tick wait still steps real rules and must leave the next wave's whole clock available. M3 has the solo run and registered runtime needed to prove this; it does not depend on later networking work.

### `prediction-equality-includes-forced-fall` — resolved, verified

[DESIGN.md](../../plans/SWARM/DESIGN.md), lines 341–352, and [M5.6](../../plans/SWARM/DONE.json) now distinguish accurate prediction from a legitimate correction. On admitted-input ticks without a rule changing speed or standing, the saved prediction must equal the confirmed controller bit for bit **before** restoration. Falls and revivals must instead produce their own corrections, followed by restored agreement. The player keeps steering while down, so a predictor that continues moving a fallen hero cannot pass by leaving the controls neutral.

This matches [SESSION's controller](../../plans/SESSION/DESIGN.md), lines 449–465: compare the ring entry for confirmed tick `t`, restore on a difference, then replay later authored frames. Replaying updates the future predicted states from the corrected standing state; the already authored future frames do not require repeated fall corrections. When prediction is dropped for repair or an excessive unconfirmed span, the design does not demand a fall correction from an inactive predictor.

For the previous counterexample, saved `(x=1, standing)` versus confirmed `(x=1, down)` is now an expected correction, even when the host admitted the player's frame. Replayed steering while down leaves the position unchanged. Revival is checked separately as another correction. Comparing only after restoration cannot impersonate prediction accuracy because the revised question explicitly rejects that comparison.

**M5.6:** the whole-run crowd load remains, and revival and continued steering strengthen the fall case. Corrections must be accounted for from SESSION's report and confirmed controller changes. M5 follows SESSION M4, which provides the prediction and in-memory match machinery. No new engine capability or later socket milestone is needed to judge this item.

## Scope and requirements

I read all three verbatim planner tasks under `briefs` in [RECORD.json](../../plans/SWARM/RECORD.json), oldest first, and the [approved project brief](../../plans/BRIEF.md) before comparing the revision. The two re-plan tasks request repairs; they do not supersede the human's [swarm goal](../../design/NEXT-GOAL-SWARM-GAME.md).

The complete design and plan still cover growing crowds, automatic attacks, between-wave upgrades, original identity and art, shared cooperative rules and actual socket play. The game remains outside engine crates. The module list's four seam rulings authorize the necessary shell, repository-guard and driven-world integration. Scope assumptions remain disclosed: one arena, the proposed content counts, no sound or gamepad, same-machine network qualification and simulator function rather than speed. M1 retains the owner's concept choice; M8 retains acceptance and completion of requested changes unless the owner explicitly defers them.

I read every design change and all 94 acceptance items. A comparison against `b7fbe03e9d36` confirms that only M3.17 and M5.6 changed. Neither becomes easier to satisfy with a convincing fake, and both are judgeable at their respective milestones. The unchanged items retain the previous review's resolutions for authority-only spawning and view containment, the final-wave timeout outcome, unbounded waiting, baseline-only cost measurements and owner acceptance at close. No further Must Fix emerged from the revised assembly.

The review also used README, ORCHESTRATE, the module list and seam rulings, the review skill and Thrum review guidance, the graded examples, relevant C02/C03/C13 contracts, SESSION's controller design and the core candidate/commit implementation. The planner's log was read after examining the fixes. External game references were treated as proposed quality references, not independently verified research.

No game, browser, simulator, device timing or visual capture was run. No captures were needed for this design review. Only this review file is changed, left uncommitted for the review script; DONE.json and all planner fields are unchanged.

Validation: `./dev check-docs` passed (799 Markdown files, 5,367 local links, 30 module records, crate policy and current design hashes). `git diff --check` passed.

DESIGN OK
