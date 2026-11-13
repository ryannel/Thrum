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
- 2026-11-13 The design still needs its review.
