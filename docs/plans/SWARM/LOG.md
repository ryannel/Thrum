# SWARM log

- 2026-11-13 Planned from the lead's task, "Plan module SWARM. It needs: SIM-2D, RENDER-2D, GAME-UI, TRANSPORT,
  SESSION" (row 21 of the module list); no words of the owner's were passed on, so the design quotes the owner's
  page of 10 November. Eight milestones replace the brief the module list carried, whose text is in git
  (`git show 97762099:docs/plans/ARCHITECTURE.md`). M1 waits on `SHOWCASE:M8`; M2 on `SIM-2D:M3`, `SESSION:M1`,
  `RENDER-2D:M5` and `GAME-UI:M5`; M5 on `SESSION:M4`; M7 on `SESSION:M6`.
- 2026-11-13 Planner's reading of the owner's words. "Upgrades between waves" is taken as Brotato's shape:
  upgrades are chosen only in an intermission and nothing pauses inside a wave. "Thousands … on screen at once"
  (the brief's default) is taken as at least 2,000 enemies and projectiles in one 16:9 view. Both are
  assumptions in the plan.
- 2026-11-13 Planner's decisions on co-op, left to the planner by the brief. One team level from shared
  pick-ups; each player chooses their own upgrades at the same time in the intermission, which a clock ends when
  more than one is present; a fallen hero stands at the next wave with half its health; an away hero leaves the
  field; the budget scales with present heroes. A choice never pauses the run, because the session's ticks never
  pause.
- 2026-11-13 Planner's decisions the brief did not name. The run is a state machine in the world, changed only
  by ticks and admitted actions, so a match, a join and a replay all carry it. The rules see only admitted
  frames, from a local, a host or a client driver. Enemies never move a hero, so the predicted controller is
  corrected only by input, a speed upgrade or leaving the field. The cap on bodies is set from a measurement in
  Chrome in M2, since Chrome steps and draws on one thread and one match has one crowd. Directions are unit
  vectors turned by constant rotations, so the profile gains no trigonometry. Packages are named `swarm`
  whatever the owner calls the game.
- 2026-11-13 Planner's decision on the asks of other modules. The repository guard for a second game, the
  window and page for a game with no sky kit, the iOS shell for a second game and a driven world are built by
  this module's builder under four seam rulings in the module list, as the other track 3 modules' asks were. The
  iOS shell is lifted into `thrum-app` and shared, not copied. No other module's design, plan or done-when file
  changed, and no contract's text changed.
- 2026-11-13 Planner's probe, in [the evidence folder](../../evidence/SWARM/planning-probe.md): arithmetic on
  how many bodies fit a 16:9 view. Bodies a thirtieth of the short side cannot reach 2,000; a fiftieth reaches
  2,170 at 40% cover. So a common enemy is about 22 px at 1080p.
- 2026-11-13 Researched: Brotato's waves and what happens between them (its wiki), Vampire Survivors' online
  co-op of October 2025 (batched level-ups, a pause that does not stop the others) and its couch co-op's
  revival, and spawn budgets counted in threat. Two sources were read only through a search summary and are
  marked so in the design.
- 2026-11-13 Not measured while planning, because no engine module is built: the game's tick, its cap in
  Chrome, its world image and every byte count. M2, M5 and M7 record them.
- 2026-11-13 Design review: changes requested, five Must Fix
  ([review](../../evidence/SWARM/design-review.md)). The entries below are the planner's answers.
- 2026-11-13 `spawn-depends-on-local-view`. The spawn rule no longer names a view. Constants of the rule set
  (the sight spans, the view slack, the spawn margin) give each hero a sight square; the budget spawns outside
  every present hero's, and the view is held inside it. The view fits the sight spans, so a window that is not
  16:9 shows less and never more; the claim that the count in view is the same in every window is withdrawn.
  New M2.17, M2.18 and M4.16; M2.9, M2.10, M5.4, M6.2, M7.2 and M8.1 written again to prove it across window
  shapes, a resize, a rotation and each peer's own view.
- 2026-11-13 `final-wave-timeout-has-no-outcome`. The last wave has no intermission. The run is won when its
  boss dies or when its clock ends with a present hero standing, read from the owner's "hold out" and Brotato's
  rule; on a shared tick the boss's death beats the loss and the loss beats the clock. New M3.17, split from
  M3.1. It is an assumption in the plan.
- 2026-11-13 `whole-run-tick-bound-conflicts-with-waiting`. Waiting alone in an intermission stays as it was.
  The 36,000-tick bound on a run is withdrawn: combat time is bounded by the waves' clocks, waiting and a
  session's world ticks are not, and 36,000 ticks is the length of the retained scripted traces (M3.5, M4.4).
  M3.17 steps a wait longer than that.
- 2026-11-13 `cap-measurement-reintroduces-timing-gate`. M2.1 is a baseline record again, under brief decision
  3. The builder picks the cap with it in hand; the cap is bounded by the count in view, smoothness to the eye
  and peers in step, and no time sets it, stops the build or fails an item.
- 2026-11-13 `owner-requested-fixes-can-remain-at-close`. M8.12 passes only on the owner's acceptance; a change
  they ask for is made first unless they say in their own words that it may wait. M8.10's record says track 3
  is closed only by quoting that acceptance. M8 is renamed to say so.
- 2026-11-13 Suggestion `prediction-correction-causes-omit-defeat`: taken. Whether the hero is standing is part
  of the controller, so a fall, a standing again, a leaving and an entering each correct the prediction once;
  M5.6's run now includes a fall and allows those causes, and a speed upgrade the rules make when a clock runs
  out. Left as written, M5.6 could not be met by a hero that spends a run in the crowd.
- 2026-11-13 The revised design still needs its review.
- 2026-11-13 Second design review: changes requested, two Must Fix and no Suggestions
  ([review](../../evidence/SWARM/design-review.md)). The entries below are the planner's answers.
- 2026-11-13 `run-end-priority-precedes-damage`. The priority is unchanged; the tick now produces it. Run rules
  early in a tick count the clock and judge no ending. One rule, the last of the tick, reads what the tick has
  staged (the boss's death, who still stands, the clock) and stages the new state, so one commit carries the
  event and its outcome. A tick is stepped for the state it began in, and the first tick of an intermission, a
  win or a loss clears the field, because `thrum-core`'s staged edits cannot be withdrawn. The loss beats the
  clock of any wave, not only the last. M3.17 keeps its cases and now reads each outcome from the state that
  same tick committed.
- 2026-11-13 `prediction-equality-includes-forced-fall`. M5.6 written again from what it claims. Equality before
  any restore is asked of ticks the host admitted on which no rule changed the hero's speed or standing. A fall
  and a standing again are each asked for as one correction, on its own tick, to the confirmed controller, with
  equality from the next tick. The causes on record now include the session's own return from a stall or a
  repair, which `SESSION:M4.2` names. No engine capability is added.
- 2026-11-13 The revised design still needs its review.
