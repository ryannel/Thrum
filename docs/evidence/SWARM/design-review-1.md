# SWARM design review

Design changes requested: **5 Must Fix findings**. The design covers the human's requested swarm survivor, original identity, reusable 2D engine support and networked cooperative play. Its broad scope is appropriate. The findings below concern mechanisms and acceptance criteria that would make building it as written go wrong.

Reviewed on 13 November 2026 at `cea01c770118aafb3d7b65302d8fccb3c17ef483`. This is a design review, not an implementation, visual or device qualification. There was no previous design review in this checkout.

**1. Must Fix — `spawn-depends-on-local-view` — verified design/interface gap.**

The spawn rule cannot determine the promised positions from the authority inputs it is allowed to read. [DESIGN.md](../../plans/SWARM/DESIGN.md), lines 169–172, places enemies outside every present hero's view. Lines 300–312 give each player a view around their own hero, sized from the window's shorter side. But lines 405–409 put the view in presentation memory, and lines 548–550 forbid rules from reading presentation. The declared actions carry no view extent (lines 149–150). [SIM-2D's profile](../../plans/SIM-2D/DESIGN.md), lines 124–135, permits only committed state, admitted inputs and constants; [RENDER-2D](../../plans/RENDER-2D/DESIGN.md), lines 163–175, makes the view a per-frame value based on the output extent.

An ordinary landscape/portrait pair exposes the gap. With short-side world span `S`, 1920×1080 shows `1.777778S × S`, whereas 1080×1920 shows `S × 1.777778S`. Relative to the same hero, `(0.75S, 0)` is inside the first view and outside the second. A square window shows only `S²`, so the assertion that fixing the short side makes the amount of world and count in view independent of the window is also false. A headless host has neither player's local view. Resizing or rotating must not change the shared simulation through an unreplicated input.

Resolve the spawn exclusion in authority terms and state how it relates to the supported view extents. A fixed gameplay region and a constrained view policy are possible choices; the planner should choose the behavior. Acceptance should establish equal spawn results for the same admitted inputs across different window shapes and a resize/rotation, while checking the chosen off-screen spawning promise. M2's conformance and M7–M8's shared-crowd proofs currently do not settle this missing input.

**2. Must Fix — `final-wave-timeout-has-no-outcome` — verified mechanism gap.**

M3.1 promises a complete run with a win or a loss. [DESIGN.md](../../plans/SWARM/DESIGN.md), lines 155–167, makes the boss's death the winning event and gives the run a fixed number of timed waves. Lines 225–226 unconditionally enter an intermission when a wave's clock ends and clear its enemies. There is no final-wave exception.

Consider a living player who reaches the final wave's deadline with the boss still alive. The specified transition removes the boss and enters an intermission. It defines neither a loss nor a remaining wave in which the boss can be killed. Treating cleanup as the winning death would instead award victory without defeating it. The twenty-seed balance check can miss this by producing only early deaths and wins before the deadline.

Define the final wave's timeout outcome and its priority relative to boss death and player defeat, then make M3.1 prove that outcome from committed state. This needs a rule, not another successful winning trace. No runtime reproduction was possible because SWARM is not built; the counterexample follows the written transitions.

**3. Must Fix — `whole-run-tick-bound-conflicts-with-waiting` — verified contradictory criterion.**

[DONE.json](../../plans/SWARM/DONE.json), lines 261–264 (M3.1), requires a whole run to take at most 36,000 ticks. [DESIGN.md](../../plans/SWARM/DESIGN.md), lines 165–167, gives the same bound because the dependencies prove that trace length. Yet lines 230–232 let a sole present player remain in an intermission without a deadline. The local driver keeps its fixed clock (line 259); a hosted match also keeps committing session ticks while waiting. All-away play separately holds the wave clock (lines 296–297).

A player who waits 601 seconds in the first intermission consumes 36,060 world ticks before any subsequent wave. This is allowed play, not an invalid input. M3.1 cannot require both the total tick bound and the specified waiting behavior. A fast scripted winning run would conceal the contradiction.

Distinguish combat duration, waiting time and world ticks, and align the design and M3.1 with the intended bound. The dependencies' 36,000-tick evidence is a tested range, not a mechanism that limits a session's lifetime. Preserve the stated waiting behavior or explicitly revise it, and describe what the retained run proves without claiming a universal bound it cannot meet.

**4. Must Fix — `cap-measurement-reintroduces-timing-gate` — verified conflict with the human's decision.**

[DONE.json](../../plans/SWARM/DONE.json), lines 79–82 (M2.1), fails the run unless a measured median of tick plus frame CPU time is below the tick period at a population of at least 3,000. [DESIGN.md](../../plans/SWARM/DESIGN.md), lines 175–181 and 484–485, explicitly stops the builder on that result. This conflicts with [BRIEF.md](../../plans/BRIEF.md), decision 3, and the [6 November owner decision](../../design/decisions/PERFORMANCE-PHILOSOPHY-2026-11-06.md): numeric performance figures are baselines and tripwires, and the latter expressly overrides numeric admission gates. The design itself repeats that rule at lines 560–561.

Measuring Chrome early and using the measurement to inform capacity is sensible. The defect is making that timing threshold an additional mandatory gate. The requirement to deliver thousands of bodies, smooth visible play and functioning peers remains; measurements can guide the implementation and diagnose failure of those requirements. Revise M2.1 and the stop instructions to follow that distinction. If a timing gate is desired instead, it needs an owner decision; the lead's planning task cannot supply one.

**5. Must Fix — `owner-requested-fixes-can-remain-at-close` — verified acceptance loophole.**

[DONE.json](../../plans/SWARM/DONE.json), lines 1043–1048 (M8.12), accepts the owner having said whether the game is ready “or what to change.” Its inspection accepts each requested change either being made or merely written in the plan as work still to do. M8 is the last milestone and M8.10 writes the record closing track 3.

For example, the owner could say that co-op feels bad and must change before closure. Recording that work in the plan satisfies the literal inspection while delivering none of it. This is especially consequential because [DESIGN.md](../../plans/SWARM/DESIGN.md), lines 325–326 and 505–506, delegates co-op feel and human balance to this judgment.

Make closure depend on the owner's acceptance and on requested prerequisites being resolved. An owner-authorized deferral can count as such; a planner recording unfinished work cannot substitute for that authorization. Negative feedback should leave the closing item open with the required changes recorded.

**Suggestion — `prediction-correction-causes-omit-defeat` — verified wording gap, not a build blocker.**

M5.6 lists only held input, a lead change or a command that changed the controller as correction causes. The design also allows a hero to leave or enter the field (lines 245–248), and enemy damage can make a hero fall without a command (lines 294–295). Clarify how controller presence is handled at defeat and revival, and allow the corresponding legitimate reconciliation in the proof. A movement test whose hero survives throughout can be judged as written, so this does not by itself block M5.

The review derived the required behavior first from [RECORD.json](../../plans/SWARM/RECORD.json)'s sole verbatim brief, the project brief and [the owner's swarm goal](../../design/NEXT-GOAL-SWARM-GAME.md). It then read the full SWARM design and all **90 items**, the plan, relevant dependency mechanisms and shared contracts, the seam rulings, the planning probe and finally the log. The view dimensions and waiting-time arithmetic above were checked with Python. Existing world-lane, frame-world and repository-guard entry points were inspected for the proposed integration seams; this was not a full code audit. No game, browser, simulator or visual capture was run, and no claim of shipped-game quality or performance is made. External game comparisons were treated as proposed quality references, not independently verified research.

The remaining proof structure is viable at its milestone boundaries:

- **M1.1–M1.4:** Concrete concept artifacts and recorded owner choice are judgeable without engine code. The limited bar for discarded concepts is appropriate.
- **M2.1–M2.16:** The dependencies provide rules, queries, encoding, sprites and UI before the slice. Real committed state, independent hit searches, wire/local conformance, in-view counts and native-size motion inspection reject the main fakes. Findings 1 and 4 need resolution.
- **M3.1–M3.16:** The whole-run, upgrade, input-routing and lifecycle proofs are otherwise concrete. Findings 2 and 3 affect the state-machine criterion.
- **M4.1–M4.15:** Content must appear in real runs, upgrades affect later gameplay, and art is inspected both on sheets and in the late game. Counts alone cannot pass the look and motion items.
- **M5.1–M5.9:** Separate worlds on the conditioned in-memory link are appropriate for this milestone. The plan explicitly has no window here, so the driven-world integration arriving in M7 is not a missing M5 dependency.
- **M6.1–M6.6:** Real simulator touches, safe-area inspection and unchanged predecessor checks can establish function after the shared-shell move. They make no device-speed claim.
- **M7.1–M7.11:** The driven world and real socket pair arrive after SESSION M6. Per-peer digests, link entry, lifecycle and captured response are materially stronger than a shared-process demonstration; the spawn/view issue still applies.
- **M8.1–M8.13:** The closure assembles the earlier capabilities and labels absent evidence. Finding 5 affects acceptance. The simulator-speed exception is explicitly disclosed in the plan; a dropped simulator must remain reported as incomplete participation, not evidence of a completed four-player run on all targets.

The scope assumptions are visible to the owner: timed waves, a bounded arena, the content counts, no sound or gamepad, same-machine network qualification and no public deployment. Those are not hidden reductions of the request. The four engine integration changes have named owners and predecessor preservation rules. This review requires no expansion into new arenas, internet hosting, sound or additional engine modules.

Validation: `./dev check-docs` passed (797 Markdown files, 5,339 local links, 30 module records, crate policy and current design hashes). Only this review file was added; the design, plan and done-when file were left unchanged.

DESIGN CHANGES: 5 must_fix findings
