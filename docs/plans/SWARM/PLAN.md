# SWARM plan

The swarm-survival game itself: its name, rules, characters and art. [Design](DESIGN.md) ·
[done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Read [Mechanisms](DESIGN.md#mechanisms), [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[Limits on every milestone](DESIGN.md#limits-on-every-milestone) and
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-swarm) before starting any milestone, and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before landing. The design was written before any of the
five engine modules was built, so start each milestone by reading what they left.

## Open for the human

- Assumption: SWARM waits for the V1 exit, as all of track 3 does. Its first milestone, your pick of a concept, could be made earlier if you want the choice in before the engine work ends.
- Assumption: "upgrades between waves" means upgrades are chosen only in the break between two waves, as in Brotato. Nothing pauses in the middle of a wave, and there is no Vampire Survivors style level-up during play.
- Assumption: "thousands on screen at once" means at least 2,000 enemies and projectiles inside one player's view at the run's peak. That makes a common enemy small: about a fiftieth of the screen's height, 22 pixels at 1080p. The concepts in M1 show this at real size, and you can trade count for size there. A window of another shape shows less of the field, never more.
- Assumption: the game is one arena and one run of about ten minutes, with four heroes, six kinds of enemy and a boss, six weapons and about twenty upgrades. A run is won by killing the last wave's boss or by outlasting that wave. Nothing is kept between runs: no unlocks, no currency, no saved progress.
- Assumption: in co-op, pick-ups raise one level the whole team shares, every player chooses their own upgrades at the same time between waves, and a clock makes the choices for anyone who is slow. A fallen hero stands again at the next wave with half its health, and the run is lost when nobody is standing. The crowd grows with the number of players.
- Assumption: every player gets the crowd size that Chrome can hold, since one match has one crowd. A larger crowd for Mac-only play is not built.
- Assumption: a Mac hosts, and Chrome and the iPhone simulator only join, by a link. "Online" is proved between separate processes on this Mac with delay and loss added. No second machine, internet path or public host is run (owner decision 10).
- Assumption: no sound or music, no gamepad, and English only.
- Assumption: the art is made in this repository by the agents, original, with no paid generation service. M1's concepts are made the same way, so they show what that can deliver. Fonts are the builder's own, public domain, CC0 or Apache-2.0.
- Assumption: the code is named `swarm` whatever you call the game; your name for it appears on the title screen.
- Assumption: enemies hurt a hero and never push it, so that your own character can answer at once in co-op without being corrected all the time.
- Assumption: the iPhone simulator proves that the game works, never its speed. A simulator player dropped from a four-player match for being too slow is recorded and is not a failure.
- Assumption: "smooth" is judged from frame records and real-time engine capture, with no frame time as a pass mark (brief decision 3). A measurement on the built-in panel itself needs someone at the Mac and is named as not run.
- Assumption: four changes the game needs inside other modules are built by SWARM's own builder under rulings in the module list: a repository guard that admits a second game, a window and page for a game with no sky kit, an iOS shell moved into `thrum-app` so both games share it, and a world that steps when a match says. No milestone is added to those modules.
- Assumption: builders and reviewers ask to run Chrome, the simulator and sockets outside the Codex sandbox, one request for each run. The other choices are owner decision 11 in the module list.
- Assumption: you are asked twice: to pick the concept in M1, and to play the finished game in M8. Track 3 closes only on your acceptance, and changes you ask for come first unless you say they can wait.

## M1 The owner picks the game's name, premise and look from three or more concepts, each shown with its own art in a crowded arena

Criteria: M1.1–M1.4 in the done-when file.

No engine code is involved and no package is added: the concepts, their sprite sources, the script that
composes the mock arenas and the check live in `examples/swarm/concepts/`, which is not a covered path, so this
milestone needs no verifier run. [The planning probe](../../evidence/SWARM/planning-probe.md) gives the body
size the mocks use. Give each concept one attempt: most will be turned down. Do every other item before
stopping for the owner's pick.

## M2 One hero holds out alone against a crowd that grows into the thousands, with attacks that fire on their own, on the Mac and in Chrome

Criteria: M2.1–M2.18 in the done-when file.

Every later item tests against the cap you pick, so take M2.1's record first, on a rough slice, to pick it
by, and again at the claimed commit. `SIM-2D` left the profile, the queries, the reference crowd in `examples/crowd-2d`
with its runners in `crowd-2d-run`, and `scripts/sim2d_conformance.py` and `scripts/sim2d_profile.py`.
`RENDER-2D` left the crowd view in `examples/crowd-2d-view`, a second program on `thrum-app`'s window and page:
it is the nearest thing to this game's shell. The repository guard is `scripts/repo_check.py` with
`.config/architecture.toml`, changed under seam ruling 1; `thrum-app` and the capture and packaging scripts are
changed under ruling 2 only where a game with no sky kit cannot run. Art in this milestone is the first set in
the chosen look: one hero, two enemies, a shot, a pick-up and the ground.

## M3 A player plays a whole run alone on the Mac or in Chrome: waves that grow, upgrades chosen between them, a win or a loss, with the status display and menus

Criteria: M3.1–M3.17 in the done-when file.

`GAME-UI` left `thrum-ui`, the interface scene in `examples/v1-game` as a worked example of a status display, a
modal choice and event routing, and `scripts/game_ui.py`, which shows how scripted events enter the Mac window.
`./dev` is the command runner the one-command item extends. Changing a rule refreshes the retained digests of
M2's trace: do it on purpose, with a line in the log.

## M4 The whole game is there and looks finished: four heroes, six kinds of enemy and a boss, six weapons and about twenty upgrades, in the chosen look

Criteria: M4.1–M4.16 in the done-when file.

Before the review, get a fresh opinion on the art and the late-game captures from an agent that has seen none
of the work, given only the items, the quality bar and `examples/swarm/CONCEPT.md`, and keep the best attempt.

## M5 Four scripted players share one run on a simulated bad connection: every copy of the game stays identical, a player joins mid-run, one drops and returns, and upgrades are chosen together

Criteria: M5.1–M5.9 in the done-when file.

`SESSION` left the session in `thrum-network`, the scripted players and the match harness in
`examples/match-proof`, and `scripts/session_match.py`; `TRANSPORT` left the in-memory conditioned link in
`thrum-transport`. Nothing here has a window. `SESSION`'s builder may still be working in `thrum-network`:
leave it and `thrum-transport` as they are.

## M6 The game plays alone on the iPhone simulator by touch, in both orientations, clear of the notch and the home indicator

Criteria: M6.1–M6.6 in the done-when file.

The iOS shell is `examples/v1-game-ios` (`src/ios.rs`, `lease.rs`, `gesture.rs`, `world_policy.rs` and the
`ios/` scripts), owned by `PLATFORM-RUNTIME-IOS` and extended by `COOKED-DRAW`, `RENDER-2D` and `GAME-UI`. It
moves into `thrum-app` under seam ruling 3; those modules' simulator checks are M6.3's and stay as they are.

## M7 A player hosting on this Mac and a friend who opens a link in Chrome play a run together over a real connection, each hero answering at once

Criteria: M7.1–M7.11 in the done-when file.

The world lane is `crates/thrum-core/src/sim/world_lane.rs` with `clocked_world.rs`, and the frame host reads
a world through `FrameWorld` in `crates/thrum-render/src/runtime.rs`; the lane's task already owns its clock,
so read how far that carries a driven world before changing anything under seam ruling 4. `TRANSPORT` left the
socket adapters, the relay and `scripts/transport_link.py`, which shows how a page reaches a host.

## M8 Two to four players on a Mac, in Chrome and on the iPhone simulator play a whole run together through delay and loss, the owner has played and accepted it, and track 3 is closed

Criteria: M8.1–M8.13 in the done-when file.

The maturity ledger is `docs/design/maturity.json`. Do every other item before stopping for the owner's play,
with the closing record and the ledger not yet saying track 3 is closed; add that afterwards, quoting the
acceptance.
