# GAME-SDK log

- 2026-11-25 Owner: "What I would like you to plan is a high end, intuitive, expressive, readable, SDK that game
  developers can use to build new games. this should be what V1 uses ideally and what we will use to build the
  swarm game. It's VITAL that we get this right as it's the entire interface that developers and coding agents
  will use to build a variety of games. It's the front of the product we're building. So it needs to be as
  intuitive, well documented and easy to use as possible."
- 2026-11-25 Owner, on the lead's three questions: "V1 first, Rust only, I'll approve the design." So the V1 exit
  does not wait for this module, games are written in Rust with no scripting layer, and no builder starts until
  the owner approves the design.
- 2026-11-25 Planned from the lead's task, as row 37 and track 7 of the module list. The vision and scope
  document did not carry the request when this was planned, so the design quotes the owner from the task.
- 2026-11-25 Planner's choices, each with its reason in the design's research section: plain-data components and
  rule functions over the registered simulation, not a node tree with scripts; rules run in the order written,
  with no schedule graph; rules and frame functions told apart by type; looks as components; the profile's
  numbers for every game; actions as typed constants and derives only where hand-writing is long; an interface
  described each frame and kept by the engine. Read for it: Bevy 0.19 and 0.20, Godot 4.6, Unity, Unreal's
  gameplay framework, Fyrox 1.0, macroquad, lightyear and bevy_ggrs.
- 2026-11-25 Read before the design was written: `thrum-app`'s game contract, the starter, the crowd view, the
  reference crowd's rules, `thrum-core`'s registered systems, the session guide and the creator guide. The design
  is not a blind study. One engine fact set a rule of the API: structural edits resolve when a tick ends, so
  spawns and despawns land there.
- 2026-11-25 Left for the owner: whether physics, animation and audio are in the first release. The plan assumes
  2D queries and sprite animation are, and rigid bodies, skeletal animation and audio are not.
- 2026-11-25 Ten work packages named, WP1 and WP2 with items. `SWARM`'s work packages gained edges to WP3, WP5,
  WP6 and WP7; its design is still written against the engine's own API and is re-planned onto the SDK once the
  owner has approved this design.
- 2026-11-25 Design review 1 asked for changes, with 15 Must Fix findings
  ([the review](../../../.orchestrate/evidence/GAME-SDK/design-review.md)); each is settled, and
  [what changed for each](../../../.orchestrate/evidence/GAME-SDK/design-review-1-changes.md) is on record.
- 2026-11-25 Review 1, findings 1 to 5: the world's component list and `scene.component` (mechanism 2), `Attach`
  and declared actions; `PlayerJoined` at a scene's first tick and scenes with no camera; a per-player rule as
  the whole movement step; a rule's contract with file and socket refusals (ruling 1); a game's own rule
  crate, new item WP2.10 (ruling 5).
- 2026-11-25 Review 1, findings 6 to 10: the SDK-only guard keyed on naming `thrum`, so the showcase meets it
  in WP9; shared and local settings; what a command guarantees; frame functions on the pair the draw uses,
  with an event cursor; turning and scaled models in WP4 (new ruling 8).
- 2026-11-25 Review 1, findings 11 to 15: one space index for each layer; what WP1 compiles; a panic by
  target; witnesses beside digests in WP1.2 and WP2.5 to WP2.8; no lock on WP2.6 and WP2.7.
- 2026-11-25 Suggestions, all three taken: WP1.5 asks about the loop inside a rule and allows one call for each
  rule through the engine's callback; mechanism 11 gives a repeated control a key; WP4 writes the kit games on
  the SDK and WP10 removes the old route (ruling 6).
- 2026-11-25 Owner, asked whether the first release should keep the planner's default (2D queries and sprite
  animation in; rigid bodies, skeletal animation and audio out) or add physics and audio: "Planner's default".
  Written into the design and the plan as decided, and taken off the list of what is left for the owner
  (owner decision 38 in the module list).
- 2026-11-25 Design review 2 asked for changes, with 5 Must Fix findings
  ([the review](../../../.orchestrate/evidence/GAME-SDK/design-review.md)). It was the second in a row, so no
  review reads these fixes before the owner does.
- 2026-11-25 Review 2, finding 1 (the cooked rock's draw): mechanism 7 and ruling 8 now name the one path an
  entity's `Model` is drawn by, the 3D scene's entity rows with `CONTENT-LOOP`'s cooked mesh rows, so the
  rock turns through the rows WP4 changes. The cooked placement stays as built and draws `Scenery`, a new
  scene-level textured mesh that stands still (example C's valley). An entity's `Model` has no texture in the
  first release, because entity rows' materials take none. WP4 is judged on the cooked rock.
- 2026-11-25 Review 2, finding 2 (the predictor's tick): the controller's state carries the tick number, so a
  predicted or replayed step hands the per-player function the tick the host commits it at, from a baseline
  at any tick, with `SESSION` unchanged (mechanisms 3 and 12). WP2.1 proves the function is handed the tick
  the other rules see.
- 2026-11-25 Review 2, finding 3 (reaching round `thrum`): the guard covers every package of a game, a kit
  game or a template by where it is, whatever it names, across path, git and registry dependencies, with a
  registry list of packages on the engine's own API (mechanism 1, ruling 5). The showcase is listed until
  WP9 and the proving ground for good.
- 2026-11-25 Review 2, finding 4 (a dropped picture): frame functions run once for each encoded picture and
  are never run again for a dropped one, so everything they do happens once. A `burst` is kept by the SDK and
  shown in the next submitted picture (mechanism 4). Moving the cursor at submission was turned down: it runs
  game code twice on the same events.
- 2026-11-25 Review 2, finding 5 (a late upgrade): the glance table gives mechanism 6's guarantee, and
  example D's choice names the wave it answers, which `take_choices` checks.
- 2026-11-25 Review 2's suggestions, both taken: the plan's `scene.component` assumption gives the multiplayer
  exception, and the panic table names `Game::test()` by target.
- 2026-11-26 Owner, after an industry review of the design
  ([the review](../../../.orchestrate/evidence/GAME-SDK/owner-review-2026-11-26.md)): "Yes I agree" to all ten
  recommendations and all four owner decisions. They are the design's decisions D1 to D4 and R1 to R10.
- 2026-11-26 Three agents each wrote a whole game against the design as the review amended it (an online 3D
  arena shooter, a 2D platformer, a 3D exploration game; reports in the main checkout under
  `tmp/sdk-dogfood/`). All three said "not yet" and praised the same core: written rule order, the typed wall
  between rules and the screen, phases and windowless tests.
- 2026-11-26 Owner, asking for this rewrite: "We're looking for something that's readable, expressive,
  intuitive and powerful. Don't optimise at all for reducing the amount of re-work needed. Let's just find the
  best possible design."
- 2026-11-26 Design rewritten whole. Planner's choices beyond the owner's decisions, each for the owner to
  overturn: a scene is a type whose fields are its arguments; one written order for a scene, with phases and
  control lines in it; action maps read as plain fields with default bindings on the map; a control function
  may read anything, and a match runs ahead a named list; one package with a `rules` module checked by being
  built alone; `Screen<T>` for `Local<T>` and `Outbox` for `Send`; no tick is stepped while paused.
- 2026-11-26 The earlier design's rule that spawns land when a tick ends is gone (R2). Ruling 2 of the module
  list now asks core for a barrier after each registered system, and WP1.1 proves it first.
- 2026-11-26 Scope left to the owner as ten one-line questions in the design: 3D collision and character move,
  gamepads, audio, predicted projectiles, rewound hits, 3D multiplayer, a host with no window, skeletal
  animation, text fields, levels from an editor's file. The first-release scope of 25 November stands meanwhile.
- 2026-11-26 Work packages: WP1 and WP2 rewritten, WP11 (prefabs and the tree) added between them, WP12 (level
  collision and content) and WP13 (saves and recordings) named. WP3, WP5, WP6 and WP7 keep the outcomes
  `SWARM` waits on; WP6 gains commands and offers, so the swarm game's upgrade choice still arrives with it.
- 2026-11-26 `SWARM`'s design is still to be re-planned onto the SDK once the owner approves this design, as
  before. Nothing of `SWARM`'s was changed here.
- 2026-11-26 Design review of the rewrite asked for changes, with 18 Must Fix findings
  ([the review](../../../.orchestrate/evidence/GAME-SDK/design-review.md)). Each is settled below, in the
  design and the items. `SESSION` is used as built throughout, and no other module's file changed.
- 2026-11-26 Finding 1 (the roster in a frame function): `Players` only looks and `PlayersMut` changes, as
  `Phase` and `PhaseMut` do. Frame functions, spawn functions, conditions and captures take read-only
  parameters by type (mechanisms 3, 6, 10, 16). WP2.4 holds the compile error for a spawn function, and WP3
  carries the one for a frame function.
- 2026-11-26 Finding 2 (the factory outside the check): mechanism 1 lists every route by which a game
  registers code that makes or changes the world, the spawn function among them, and all must be in the
  `rules` module. Starting values are part of the rule-set identity. WP2.10 refuses each route in an outside
  game.
- 2026-11-26 Finding 3 (a reader beside a writer): two parameters conflict over a component when both name
  it, either writes it and one entity matches both, a control function's character included. It is tested on
  each new set of components when the rule that made it ends (mechanism 3). WP1.4, WP11.3 and WP2.4 cover
  writer with writer, writer with reader, and a change of membership.
- 2026-11-26 Finding 4 (a phase against a scene): a tick makes one transition, the first request stands
  whatever its kind, and a request made by an enter or exit function is the next tick's first (mechanism 4,
  the tick walk). Example D's dead hero records no clear, and example E's `next_wave` says when it lands.
  WP1.2 (4) and (5).
- 2026-11-26 Finding 5 (a gate closed during play): the world holds the level's recipe, the tile map by
  identity and each shape a rule added, so it is in the digest, a baseline and a rewind, and the collision of
  each recent recipe is kept by identity (mechanism 8). Carried into WP12 and WP5.
- 2026-11-26 Finding 6 (what is reconciled): the predicted controller's whole state, with the resources, the
  phase, the level, the pause and the input state the step reads, is compared with the host's, so a changed
  setting corrects a hero that stands still. Every rule parameter has a row in the control table, `Events<E>`
  among those not run ahead. A predicted event is shown once, by tick (mechanisms 6, 10, 15). WP5's carried
  proof now asks for a correction and equality afterwards.
- 2026-11-26 Finding 7 (bots and joins): seats are the SDK's, in world state. A person takes a free seat,
  and where bots hold them all takes a bot's: the bot leaves, the person joins fresh. A rule answers an offer
  for a bot with `Offers::pending` and `answer_for` (mechanisms 6, 7, 15, example F). WP2.1, and carried into
  WP6 and WP5.
- 2026-11-26 Finding 8 (the lobby and a paused match): the lobby is an ordinary scene on the match's clock,
  started when hosting starts, so a ready mark, a host's panel choice and *Start* are input at a tick, and
  `starts_with` carries the choice into the scene. A paused match keeps committing ticks in which no rule
  runs and the game's tick stands, so any allowed player can end the pause and nothing is caught up
  (mechanisms 4, 15). The proposal to `SESSION` to stop committing is withdrawn.
- 2026-11-26 Finding 9 (button edges): a button travels as its held state and a running count, with a bit
  that says the frame was sampled. A repeated frame presses nothing, a neutral frame keeps the last count,
  and `Button::presses` gives two taps between frames (mechanism 6). WP2.1.
- 2026-11-26 Finding 10 (what a save brings in): a restore is admitted input, the save's bytes in pieces
  that fit an action, gathered in world state under a 64 KiB default limit, and lands whole or not at all. A
  saved resource's starting value is part of how a run starts, held by a recording. Every save request names
  its type (mechanism 12). Carried into WP13 and WP5.
- 2026-11-26 Finding 11 (a prefab a rule picks later): names are declared in a `prefabs!` block and each
  knows its block, so the SDK knows every name of a block the description touches and refuses an undefined
  one at startup. Only a block the description never names is first checked at use (mechanism 5, the
  examples). WP11.3 holds both.
- 2026-11-26 Finding 12 (`keep_world`): it holds where the place can be written as a local transform, and
  is refused with a typed error and nothing changed under a zero-scale parent or shear (mechanism 5).
  WP11.2 proves it exactly under quarter turns, WP2.3 under another angle, WP11.3 the refusal.
- 2026-11-26 Finding 13 (starting a test): `test()` returns a result. Fixtures are given on `test_setup()`
  and installed before the checks and the first scene's entry. The lookups' return types are in a table, and
  `character` tells a waiting join from a missing character (mechanism 17). Every example's test now takes
  the result.
- 2026-11-26 Finding 14 (the starters): Rockfall's `crash` asks every player's ship and the ships stand side
  by side. Example B gives each player a ball to roll round the rock, which stays WP4's reference.
- 2026-11-26 Finding 15 (input contexts): `InputContext`, `#[context(..)]` on a map, `Screen<InputContext>`
  to choose one, and a rule for a key held across a change, which a modal follows too (mechanisms 6, 14).
  Carried into WP3. Proposed in the glossary.
- 2026-11-26 Finding 16 (proofs before their interface): each carried proof now sits with the first work
  package that has what it uses. WP12 gains shared settings as a game resource. WP3 keeps what needs no
  command and no `Ui`. Commands in a dropped picture and the pause menu move to WP6, and reconnects to WP5.
  Example D has a second coyote test, for a press after the window, and its window is six ticks so the
  first test's press is inside it.
- 2026-11-26 Finding 17 (a hit on the level): `Level2d::ray` returns `SweptLevel`, whose hit names a layer.
  A `Space2d` hit keeps its entity (mechanism 8, the reserved 3D shape alike).
- 2026-11-26 Finding 18 (content): a record's field types are listed, a raw float is a compile error, and
  the validator is the function `#[content(validate = ..)]` names, with an invalid record shown failing
  before play (mechanism 12). Carried into WP12.
- 2026-11-26 The review's suggestions, all three taken: bounds are written on `SceneType`, `PhaseType` and
  `ContentType`, which the derives implement; the numbers table says an integer past 16,777,216 is rounded
  once, and WP2.3 holds it; WP2.2 says the set of hits is `thrum-sim2d`'s and the order is the SDK's.
- 2026-11-26 Planner's choices in this revision, each for the owner to overturn, are in the plan's
  assumptions. Left open for builders: how a game of fewer than four players keeps `SESSION`'s other slots
  closed, and the width of a button's count.
- 2026-11-26 Second design review of the rewrite asked for changes, with 10 Must Fix findings
  ([the review](../../../.orchestrate/evidence/GAME-SDK/design-review.md)). It was the second in a row, so no
  review reads these fixes before the owner does; each was checked against its finding. `SESSION` is still
  used as built, and no other module's file changed.
- 2026-11-26 Finding 1 (a restore and the host's frame): the pieces of a save are budgeted against the host's
  whole 920-byte frame, after that tick's commands and answers, about 850 bytes a tick, so 64 KiB takes about
  80 ticks. A failed restore is undone and the tick stepped again inside the SDK's one world step, so
  `SESSION` sees no error (mechanisms 7 and 12, the tick walk). WP5's carried proof restores a save near the
  limit while the host sends commands.
- 2026-11-26 Finding 2 (the clock a control function reads): the predicted controller's state carries the
  ticks the scene and the phase were entered at, beside the game's tick, so `in_scene()`, `in_phase()` and
  `Phase::time()` are run ahead from a baseline at any tick. Equality with the host is promised for a step
  whose frame the host admitted and whose dependencies had not changed (mechanisms 6 and 15, WP5's proof).
- 2026-11-26 Finding 3 (one effect, seen once): an emission is named by control line, tick, event type and
  ordinal. The SDK keeps how many of each it has shown for each recent tick, so a confirmed tick that sends
  one more than prediction did shows only that one. Predicted emissions leave the step through a note in the
  `Rules` value the SDK gives the controller, and confirmed ones through the ring (mechanism 10).
- 2026-11-26 Finding 4 (accesses inside one parameter): the conflict is defined on accesses, between two
  parameters and inside one. A component named twice in one `Query`, or twice by a control function, with a
  writer, is refused at startup. A control function's query is tested against the character its player has,
  after `possess` too (mechanisms 3, 6 and 16). WP1.4 and WP2.4.
- 2026-11-26 Finding 5 (the seat limit): the SDK keeps it over `SESSION`'s four slots. It hands `SESSION` no
  link while people hold every seat and none is away, and a newcomer admitted while an away player's seat is
  kept gets no seat, is told full and leaves. `TestGame::join`, `test_match` and `TestMatch::join` return
  results, as example F writes them (mechanisms 6, 15 and 17). WP2.1, and WP5's proof. The open question on
  closing `SESSION`'s other slots is settled and removed.
- 2026-11-26 Finding 6 (a boss nobody defined): every `prefabs!` block registers itself with the built
  program at link time, so the SDK knows every declared name before play with no line of the game rooting a
  block. No authored reference is first checked at use (mechanism 5, the mistakes table, risk 10). WP11.3
  holds the block the description never names, and WP2.5 to WP2.7 carry a witness on the three targets.
- 2026-11-26 Finding 7 (two floors in one record): each tile map in cooked content has an identity of its
  own, its record and the path of fields to it, which the recipe holds and the built collision is kept under
  (mechanisms 8 and 12). Rule-added shapes stay in the recipe. Carried into WP12.
- 2026-11-26 Finding 8 (a phase asked for by a leaving scene): a held phase request stands only if its scene
  is running once the landing ends. From an exit function of a scene being left it returns `false`, holds
  nothing and does not reserve the next tick. Scene requests are held as before (mechanism 4). WP1.2 (5).
- 2026-11-26 Finding 9 (a tap during a pause): the presser's machine keeps pause and play apart. It counts no
  press while its own screen shows the pause, and the world reads no input in a paused tick. A press made in
  play that reaches a paused match is read when the pause ends (mechanisms 4 and 6). Carried into WP3 and WP5.
- 2026-11-26 Finding 10 (exact reparenting): `keep_world` promises a bound, one part in 2^17, and exactness
  only where every number composes exactly. It is refused under a parent scaled to zero or outside 1/10,000
  to 10,000, for shear, and past the bound (mechanism 5). WP11.2 uses whole-number children and proves the
  bound at x = 1/10, WP11.3 the refusals, WP11.4 asks whether the bound is tested on every call.
- 2026-11-26 The review's suggestions, both taken: the sentence that says a press is never lost carries the
  count's bound; and the query reference gives overlaps distance order and sweeps contact order.
- 2026-11-26 Planner's choices in this revision, each for the owner to overturn, are three new assumptions in
  the plan: what counts as a press during a pause, how a small match is kept to its seats, and the bound on
  `keep_world`.
