# SWARM plan

The swarm-survival game itself: its name, rules, characters and art. [Design](DESIGN.md) ·
[done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Read [Mechanisms](DESIGN.md#mechanisms), [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[Limits on every milestone](DESIGN.md#limits-on-every-milestone) and
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-swarm) before starting any milestone, and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before landing. The design was written before any of the
five engine modules was built, so start each milestone by reading what they left.

## Open for the human

- Assumption: SWARM waits for the V1 exit, as you said on 14 November, and that includes its first milestone, your pick of a concept. The engine modules it stands on no longer wait.
- Assumption: "upgrades between waves" means upgrades are chosen only in the break between two waves, as in Brotato. Nothing pauses in the middle of a wave, and there is no Vampire Survivors style level-up during play.
- Assumption: "thousands on screen at once" means at least 2,000 enemies and projectiles inside one player's view at the run's peak. That makes a common enemy small: about a fiftieth of the screen's height, 22 pixels at 1080p. The concepts in WP1 show this at real size, and you can trade count for size there. A window of another shape shows less of the field, never more.
- Assumption: the game is one arena and one run of about ten minutes, with four heroes, six kinds of enemy and a boss, six weapons and about twenty upgrades. A run is won by killing the last wave's boss or by outlasting that wave. Nothing is kept between runs: no unlocks, no currency, no saved progress.
- Assumption: in co-op, pick-ups raise one level the whole team shares, every player chooses their own upgrades at the same time between waves, and a clock makes the choices for anyone who is slow. A fallen hero stands again at the next wave with half its health, and the run is lost when nobody is standing. The crowd grows with the number of players.
- Assumption: every player gets the crowd size that Chrome can hold, since one match has one crowd. A larger crowd for Mac-only play is not built.
- Assumption: a Mac hosts, and Chrome and the iPhone simulator only join, by a link. "Online" is proved between separate processes on this Mac with delay and loss added. No second machine, internet path or public host is run (owner decision 10).
- Assumption: no sound or music, no gamepad, and English only.
- Assumption: the art is made in this repository by the agents, original, with no paid generation service. WP1's concepts are made the same way, so they show what that can deliver. Fonts are the builder's own, public domain, CC0 or Apache-2.0.
- Assumption: the code is named `swarm` whatever you call the game; your name for it appears on the title screen.
- Assumption: enemies hurt a hero and never push it, so that your own character can answer at once in co-op without being corrected all the time.
- Assumption: the iPhone simulator proves that the game works, never its speed. A simulator player dropped from a four-player match for being too slow is recorded and is not a failure.
- Assumption: "smooth" is judged from frame records and real-time engine capture, with no frame time as a pass mark (brief decision 3). A measurement on the built-in panel itself needs someone at the Mac and is named as not run.
- Assumption: two changes the game needs inside other modules are built by SWARM's own builder under rulings in the module list: a repository guard that admits a second game, and whatever the window and page still assume of the V1 game. The shared iPhone shell and a world that steps when a match says are engine work, built before the game in `GAME-UI` WP6 and `SESSION` WP7.
- Assumption: you are asked twice: to pick the concept in WP1, and to play the finished game in WP8. Track 3 closes only on your acceptance, and changes you ask for come first unless you say they can wait.

## WP1 The owner picks the game's name, premise and look from three or more concepts, each shown with its own art in a crowded arena

Criteria: WP1.1–WP1.4 in the done-when file.

No engine code is involved and no package is added: the concepts, their sprite sources, the script that
composes the mock arenas and the check live in `examples/swarm/concepts/`, which is not a covered path, so this
milestone needs no verifier run. [The planning probe](../../../.orchestrate/evidence/SWARM/planning-probe.md) gives the body
size the mocks use. Give each concept one attempt: most will be turned down. Do every other item before
stopping for the owner's pick.

## WP2 One hero holds out alone against a crowd that grows into the thousands, with attacks that fire on their own, on the Mac and in Chrome

Criteria: WP2.1–WP2.18 in the done-when file.

Every later item tests against the cap you pick, so take WP2.1's record first, on a rough slice, to pick it
by, and again at the claimed commit. `SIM-2D` left the profile, the queries, the reference crowd in `examples/crowd-2d`
with its runners in `crowd-2d-run`, and `scripts/sim2d_conformance.py` and `scripts/sim2d_profile.py`.
`RENDER-2D` left the crowd view in `examples/crowd-2d-view`, a second program on `thrum-app`'s window and page:
it is the nearest thing to this game's shell. The repository guard is `scripts/repo_check.py` with
`.config/architecture.toml`, changed under seam ruling 1; `thrum-app` and the capture and packaging scripts are
changed under ruling 2 only where a game with no sky kit cannot run. Art in this milestone is the first set in
the chosen look: one hero, two enemies, a shot, a pick-up and the ground.

## WP3 A player plays a whole run alone on the Mac or in Chrome: waves that grow, upgrades chosen between them, a win or a loss, with the status display and menus

Criteria: WP3.1–WP3.17 in the done-when file.

`GAME-UI` left `thrum-ui`, the interface scene in `examples/v1-game` as a worked example of a status display, a
modal choice and event routing, and `scripts/game_ui.py`, which shows how scripted events enter the Mac window.
`./dev` is the command runner the one-command item extends. Changing a rule refreshes the retained digests of
WP2's trace: do it on purpose, with a line in the log.

## WP4 The whole game is there and looks finished: four heroes, six kinds of enemy and a boss, six weapons and about twenty upgrades, in the chosen look

Criteria: WP4.1–WP4.16 in the done-when file.

Before the review, get a fresh opinion on the art and the late-game captures from an agent that has seen none
of the work, given only the items, the quality bar and `examples/swarm/CONCEPT.md`, and keep the best attempt.

## WP5 Four scripted players share one run on a simulated bad connection: every copy of the game stays identical, a player joins mid-run, one drops and returns, and upgrades are chosen together

Criteria: WP5.1–WP5.9 in the done-when file.

`SESSION` left the session in `thrum-network`, the scripted players and the match harness in
`examples/match-proof`, and `scripts/session_match.py`; `TRANSPORT` left the in-memory conditioned link in
`thrum-transport`. Nothing here has a window. `SESSION`'s builder may still be working in `thrum-network`:
leave it and `thrum-transport` as they are.

## WP6 The game plays alone on the iPhone simulator by touch, in both orientations, clear of the notch and the home indicator

Criteria: WP6.1–WP6.6 in the done-when file.

`GAME-UI` WP6 left the iOS shell in `thrum-app`, generic over a game, with the V1 game's app and the crowd
view's as its users. `swarm-ios` is the third. A change the shell needs is a proposal in the log, not an edit
here.

## WP7 A player hosting on this Mac and a friend who opens a link in Chrome play a run together over a real connection, each hero answering at once

Criteria: WP7.1–WP7.11 in the done-when file.

`SESSION` WP7 left the driven world in `thrum-core` and `thrum-app`, and a windowed match of the reference
crowd that shows a driver fed by a session. `TRANSPORT` left the socket adapters, the relay and
`scripts/transport_link.py`, which shows how a page reaches a host.

## WP8 Two to four players on a Mac, in Chrome and on the iPhone simulator play a whole run together through delay and loss, the owner has played and accepted it, and track 3 is closed

Criteria: WP8.1–WP8.13 in the done-when file.

The maturity ledger is `docs/design/maturity.json`. Do every other item before stopping for the owner's play,
with the closing record and the ledger not yet saying track 3 is closed; add that afterwards, quoting the
acceptance.
