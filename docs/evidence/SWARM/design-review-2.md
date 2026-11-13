# SWARM design review

Design changes requested: **2 Must Fix findings**. The revised design still covers the human's requested original swarm survivor, reusable 2D engine support and networked cooperative play. It resolves the original five findings at the level of the chosen behavior and acceptance policy. Two inconsistencies remain in how the revised behavior is assembled and proved.

Reviewed on 13 November 2026 at `b7fbe03e9d36faebaac6a58060bec72e1a0abbaf`, against the previous review at `cea01c770118`. This is a design review, not implementation, visual or device qualification.

**1. Must Fix — `run-end-priority-precedes-damage` — verified design inconsistency; M3.17 and the tick walkthrough.**

The tick order does not produce the newly specified priority between a boss death, player defeat and the final deadline. [DESIGN.md](../../plans/SWARM/DESIGN.md), lines 164–173, says the boss's death wins over defeat, and defeat wins over the deadline, on the same tick. But its tick walkthrough, lines 505–510, advances the run's state and clocks in step 2, before weapons, damage and deaths in step 6. It then commits once in step 8, with no terminal adjudication between damage and commitment. The description of deciding the end from “one committed state” does not explain how the decision includes that tick's staged deaths.

Take the final wave with one clock tick left, the boss alive, and the last standing hero about to take lethal contact damage. Advancing the clock and run state in step 2 sees a standing hero and awards the timeout win; the winning transition also clears the field. The damage that should make this a loss has not yet been evaluated. Conversely, evaluating only the preceding committed deaths makes the run's outcome lag the death tick. Neither produces M3.17's same-tick result.

Align the tick mechanism with the selected priority: identify the tick endpoint used for deadline, boss death and hero defeat, and where their combined outcome and cleanup are decided before publication. Keep M3.17's existing simultaneous-event cases; additional test procedures are unnecessary. This does not reopen the choice to win by outlasting the final wave. It is a new integration defect exposed by that choice and its explicit same-tick priority, rather than a repeat of the previous missing timeout outcome.

**2. Must Fix — `prediction-equality-includes-forced-fall` — verified contradictory proof; M5.6.**

[M5.6](../../plans/SWARM/DONE.json), lines 706–709, now requires the test hero to fall at least once, while still requiring prediction to equal the confirmed hero tick for tick wherever the host admitted the player's frame. It also requires a correction when the hero falls. Those requirements conflict on an ordinary fall tick with admitted movement input.

[DESIGN.md](../../plans/SWARM/DESIGN.md), lines 307–313, puts `standing` in the controller and explicitly says a fall is not announced by a command: the client learns it when it commits the tick. [SESSION's controller](../../plans/SESSION/DESIGN.md), lines 442–465, predicts without the crowd and compares the saved prediction with the newly confirmed controller **before** restoring and replaying it. For a hero moving from x=0 to x=1, a lethal contact on that tick gives a saved prediction `(x=1, standing=true)` and confirmed state `(x=1, standing=false)`, even when the host admitted exactly the authored frame. The correction is legitimate; pre-correction equality is false.

Comparing only after restoration would make equality automatic and let a broken predictor pass. Arranging for the host to hold the fall tick would evade the ordinary admitted-input case. State the equality requirement for ticks without a legitimate correction cause, and separately require the recorded fall/revival corrections and their reconciled results. The selected controller mechanism can support that proof without any new engine capability.

The previous suggestion, `prediction-correction-causes-omit-defeat`, was taken: defeat and presence changes are now represented and named as causes. This new finding concerns the remaining unconditional equality clause once a fall is mandatory; the former surviving-hero proof could avoid that conflict.

**Disposition of the previous Must Fix findings.**

| Previous finding | Result against this revision |
|---|---|
| `spawn-depends-on-local-view` | Resolved. Mechanisms 3 and 9 use committed hero positions and rule-set constants for exclusion, and constrain every local view to the resulting sight square. The count is explicitly scoped to 16:9. M2.17 checks authority-only spawning, M2.18 checks the actual view function, and target and co-op items check each peer's own view. |
| `final-wave-timeout-has-no-outcome` | The missing outcome is resolved: survival to the deadline wins, the last wave has no intermission, and M3.17 requires the deadline case and ties. Finding 1 above concerns the incompatible execution order for that newly defined result. |
| `whole-run-tick-bound-conflicts-with-waiting` | Resolved. Combat time, waiting and world ticks are distinguished. Only retained scripted traces in M3.5 and M4.4 are limited to 36,000 ticks; M3.17 requires a longer intermission wait followed by a whole next wave. |
| `cap-measurement-reintroduces-timing-gate` | Resolved. M2.1 records cost without selecting or rejecting the cap by a timing threshold. Mechanism 3, the cost section, risks and limits consistently use measurements as baselines and tripwires. Count, observed smoothness and functioning peers remain requirements. |
| `owner-requested-fixes-can-remain-at-close` | Resolved. M8.12 requires acceptance and completion of requested changes or the owner's explicit deferral. M8.10 quotes that recorded acceptance; the plan keeps closure pending until it arrives. Recording unfinished work alone cannot pass. |

**Changed-item audit.** All 16 modified items and four additions were read against the old review and their milestone's available dependencies. No milestone verdict or planner field was changed.

| Items | Assessment |
|---|---|
| M2.1, M2.2 | The baseline-only cost record and authority cap are judgeable on the slice. They preserve the crowd requirement without a numeric timing gate. |
| M2.8 | Counts actual committed bodies against the selected 16:9 view. The revised window policy does not weaken that count. |
| M2.9, M2.10 | Window-shape and resize runs compare real world digests and submitted rows. These can be exercised after the shell and renderer dependencies. |
| M2.17, M2.18 | Separate authority exclusion from view containment, with a code question tying the tested camera function to the game. Prediction itself is not needed in M2 to supply an arbitrary drawn position to that function. |
| M3.1, M3.4 | Run transitions and seed outcomes remain real committed-state proofs; ending behavior has moved to M3.17 rather than disappeared. |
| M3.5, M4.4 | The retained-trace bound is now a bound on the chosen fixture, not on allowed player waiting. |
| M3.17 | The timeout, simultaneous-event and long-wait cases are judgeable at M3. Finding 1 concerns the mechanism they are meant to prove. |
| M4.16 | Finished cooked sprite sizes and upgraded speed are available here, so the spawn-margin proof has its inputs. |
| M5.4 | Four-hero exclusion includes fallen heroes, preserving their views; the co-op rules are testable with the in-memory session. |
| M5.6 | Finding 2 must be resolved so that a correct predictor can pass and a comparison after restoration cannot impersonate prediction accuracy. |
| M6.2, M7.2, M8.1 | Rotation and peer-specific view checks extend the same spawn policy to the simulator and socket runs. Separate peer digests remain required. |
| M8.10, M8.12 | Closing evidence and human acceptance now agree, with explicit owner authority over deferrals. |

The unchanged items were also read: **94 items in total**. The milestone sequence still brings concepts, a playable slice, a whole solo run, finished content, a rehearsed match, touch play, windowed sockets and final acceptance in a viable order. No additional content, platform or engine module is required by this review. The disclosed assumptions about one arena, content counts, no sound or gamepad, same-machine socket qualification and simulator speed remain visible to the owner.

The review used both verbatim tasks under `briefs` in [RECORD.json](../../plans/SWARM/RECORD.json), the [project brief](../../plans/BRIEF.md), the [owner's swarm goal](../../design/NEXT-GOAL-SWARM-GAME.md), the module list and SWARM seam rulings, the full revised design and plan, all items, relevant C02/C03/C13 contracts, and the SIM-2D, RENDER-2D and SESSION interfaces affected by this revision. The planner's log was read after forming the findings. The lead's re-plan request was not treated as authority to change the human's goal.

A small Python calculation checked view containment for landscape, portrait, square, the Mac panel aspect and the 32:9/9:32 extremes, and the 216,000 world ticks in an hour's wait. A controller-state example checked the pre-reconciliation inequality at an admitted-input fall. These are checks of the written mechanism, not SWARM runtime tests. No game, browser, simulator or visual capture was run. External game references were treated as proposed quality references, not independently verified research.

Validation: `./dev check-docs` passed (797 Markdown files, 5,343 local links, 30 module records, crate policy and current design hashes). Only this review file is changed; it is left uncommitted for the review script.

DESIGN CHANGES: 2 must_fix findings
