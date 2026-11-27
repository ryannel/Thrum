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
- 2026-11-27 Four agents each wrote a whole game against the rewritten design, with scope questions 1 to 3
  taken as yes: a 2D platformer (would choose Godot, 7 out of 10), a 3D exploration game (no, 5), an online 2D
  swarm survivor (yes, 7) and an online 3D arena game (Unreal, 2, because the first release could not put a
  3D game online). Reports and source are in the main checkout under `tmp/sdk-dogfood-2/`.
- 2026-11-27 Owner, on those reports: "Let's just be clear on the dog fooding. It seems like it's complaining
  about missing features, that's fine. But what we should be very confident in is our approach and shape of
  our core SDK design. If we're missing features, that's fine, as long as we know how we would add them in a
  delightful way. So let's flesh the plan out enough so that's clear. And let's bring multiplayer back into
  scope. Out goal is to lock in the shape of the SDK, not design every feature."
- 2026-11-27 That is a scope decision, the design's O5 and O6: the shape is what is locked, and multiplayer is
  in the first release for 3D games as well as 2D. Scope question 6 is answered yes. Questions 1 to 3 (3D
  queries and a kinematic move, gamepads, audio) stay the owner's, and so do 4 and 5 (predicted projectiles
  and contact; hits judged as the shooter saw them), each with a recommendation of no for the first release.
- 2026-11-27 Design revised for that. The nineteen mechanisms became a core of fourteen sections, each with
  what always holds and the names a game touches. The reserved shapes became an extension map: eight
  extension points, thirteen entries with game code and eleven smaller ones in a table, each adding names and
  changing none. Example C was cut to a slice, and the design went from 3,518 lines to about 3,250 with the
  map and the names added.
- 2026-11-27 Core changes, each from a round-2 report that showed the core fighting a genre, and each for the
  owner to overturn: a pause is one thing alone and in a match, with ticks committing while the game's clock
  stands, commands, answers, requests and saves admitted, and `.even_when_paused()` on a rule (the
  exploration game's talk box could never close); `PhaseMut::resume` for a save; rays, sweeps and moves asked
  of a space meet the level, with one `Hit` and one `move_shape` given the character's own collider; a
  `Player` names one stay in a seat; `spawn` returns its `Entity`; a frame function takes `Tick` and the
  level; `#[derive(Params)]`; a player's offers queue and the interface takes no direction input outside a
  modal; `Camera2d::exactly`; the lobby is the scene `Lobby`.
- 2026-11-27 3D matches: nothing in a match knew 2D from 3D, so the change is to what is proved. WP5
  carries a rehearsed 3D match with no window and the new WP14 hosts the 3D starter on the Mac and joins it
  from Chrome. Predicting more than the character is a declaration on a control line (`.predicts`,
  `.against`) and judging a hit as the shooter saw it is one method on a space over a remembered layer; both
  use `SESSION` as built and are in the extension map.
- 2026-11-27 New scope question 11, from the swarm survivor's one gap it could not work round: saved progress
  for every player, on each player's own machine. Recommended yes. Sheets, clips and tiles cooked from source
  images are planned into WP3, because sprites are in the owner's scope and nothing said how art got in.
- 2026-11-27 Items changed to match: WP1.1 (the handle `spawn` returns, `Params`, `game.run`), WP1.2
  (`resume`, `lasts` from a function), WP1.4 (a conflict through a `Params` field), WP1.7 and WP2.3 (the
  design's section numbers), WP11.2 (attaching what a rule just spawned, `.under`, `detach(..).at`), WP2.1 (a
  `Player` as one stay, a bot's button) and WP2.4 (an action set that must fit the input frame). No check
  command changed. No other module's file changed; the module list's track 7 names WP14 and the decision.
- 2026-11-27 The vision and scope document does not yet carry the owner's words of 27 November. Adding them is
  the lead's.
- 2026-11-27 Design review of the revision asked for changes, with 17 Must Fix findings
  ([the review](../../../.orchestrate/evidence/GAME-SDK/design-review.md)). Each is settled below. Shape
  questions are fixed in the core, and findings on features the first release does not build are settled in
  their entries of the extension map, in the fewest words that keep the shape sound. `SESSION` is used as
  built, and no other module's file changed.
- 2026-11-27 Finding 1 (replay from a one-file game): a game with no `rules` module is offered nothing that
  rests on the check. `--record`, `--replay` and `play_recording` refuse it as `Game::multiplayer` does, each
  naming the one step, so every recording has checked rules behind it and D1 stands as the owner wrote it
  (sections 1, 13 and 14, the mistakes table). Carried into WP13.
- 2026-11-27 Finding 2 (a phase's length, and old saves): the checked routes are every function that runs in a
  tick or whose answer enters the world, and now name the function given to `lasts` and a save's capture. A
  `Save` type and its chain of older types are in the `rules` module, because every machine runs the
  conversion when a restore lands and the derive calls it (sections 1, 4 and 10; extension point 5). WP2.10
  refuses a `lasts` function outside the module, and WP13 carries the capture and the conversion.
- 2026-11-27 Finding 3 (two accesses to one resource): a resource has no entity to tell them apart, so two
  accesses with a writer are refused at startup, through a `Params` struct too, and each of the SDK's writing
  parameters is taken once (section 3, the mistakes table). WP1.4 holds three cases and a control.
- 2026-11-27 Finding 4 (which character): a player has one character at a time, as world state. `character`,
  `Me::character`, `of_player`, the following camera, `respawn` and prediction all mean it. A scene has one
  `players().spawn`, further control lines only `.during` other phases, and a drone is an ordinary entity
  that `possess` can make the character (section 5). WP2.1 and WP2.4.
- 2026-11-27 Finding 5 (two players' dust in one tick): a predicted emission is named by its player, tick,
  type and ordinal in that player's step (section 9, the state table). Carried into WP5.
- 2026-11-27 Finding 6 (predicted contact): a move is told who is moving, with `.except(me)` on `Move2d`, in
  the core. In the entry, `.against` names every layer of colliders the function may ask, the level needs no
  naming, and another layer refuses the tick on the host. The example excludes its own craft. Carried into
  WP12 for the core's part.
- 2026-11-27 Finding 7 (an existing firing rule): stated plainly. Depth 2 predicts what the control function
  launches and nothing else, so a separate firing rule moves into it, and no rule that judges a hit changes
  (section 11's depths, the entry).
- 2026-11-27 Finding 8 (which picture a shot is judged against): input carries the view, a tick and how far
  between the pair, as a latest-value action. The world keeps each seat's view as a lag behind the present,
  so it moves on through a held frame, and a player who has left is given the present (the entry).
- 2026-11-27 Finding 9 (skipped paused ticks): a paused tick changes the world only by the input it admits. A
  rule marked `.even_when_paused()` runs in a paused tick that admits input and in no other, and the digest
  counts the game's ticks, so skipping an empty one is the same world alone, in a test and in a match
  (section 4, cost). Carried into WP3 and WP5.
- 2026-11-27 Finding 10 (a second phase machine): a scene has one, and a second `scene.phases` is refused at
  startup. The entry is now a mode kept in a resource or a component, in the core already (section 4, the
  smaller entries). WP1.4.
- 2026-11-27 Finding 11 (authored IDs and pack variants): an ID written in Rust is declared in an `ids!`
  block that registers itself as a `prefabs!` block does, and `Id::new` is gone, so startup knows every one.
  A pack adds records and declares no prefab name: a new kind is a record that names a declared base
  (section 10, example D, the data packs entry, risk 10). Carried into WP12.
- 2026-11-27 Finding 12 (a guest's progress on a bad link): its pieces travel as the guest's ordered commands,
  which `SESSION` admits in sequence, and a stalled transfer ends by the seat's grace (the entry).
- 2026-11-27 Finding 13 (the save example): `ValeSave::placed` is a `List<Id<RelicDef>, 32>`.
- 2026-11-27 Finding 14 (3D or planar): example B's online page gives the ball a hop that leaves the ground
  and tilts, and WP5's rehearsed match and WP14 are proved on it, with height and rotation read back.
- 2026-11-27 Finding 15 (the renderer's turn): WP4 is judged on the rock turned by the rule `turn`, between
  two different tick turns, with the decorative turn proved apart.
- 2026-11-27 Finding 16 (proofs before their surface): WP2.10 no longer speaks of a frame function. WP3
  carries a registered frame function with float arithmetic in an outside game and a 2D rig's damping, and
  WP4 the orbiting rig's distance.
- 2026-11-27 Finding 17 (the type a caller receives): every type a game is handed is defined in `thrum` or its
  derive crate, judged as it resolves. WP1.5 says so and adds controls the surface check must fail, and its
  check command runs them (section 1).
- 2026-11-27 Suggestions. Taken: `TestGame::offer`'s return type, `Setup` as the game's own type, and
  `host().start()` pressing the real *Start* (K10); a glossary entry for pause, proposed. Not taken: splitting
  WP1.1 and WP2.1, which are already grouped by labelled outcome and would prove nothing more split.
- 2026-11-27 Items changed: WP1.4, WP1.5 (and its check command), WP2.1, WP2.4 and WP2.10. Planner's choices
  in this revision, each for the owner to overturn, are the plan's four new assumptions and the three it
  extended.
- 2026-11-27 Design review of the second revision asked for changes, with 7 Must Fix findings
  ([the review](../../../.orchestrate/evidence/GAME-SDK/design-review.md)). It is the second in a row, so no
  reviewer reads these fixes before the owner does. Each is settled below, and checked against its finding.
  No other module's file changed, and `SESSION` is used as built.
- 2026-11-27 Finding 1 (the committed tick in pause and replay proofs): two counts are named. The committed
  tick is the engine's, counts every commit and is in the digest. The game's tick is what `Tick` gives and
  stands through a pause. An empty paused tick moves only the committed tick, so how many are committed
  stays the driver's, and a recording holds each input at the committed tick that admitted it, which keeps
  the count and which inputs shared a tick. Runs are compared at the same committed tick: a replay equals
  its run at every one, and two live runs under different drivers are compared value by value (section 4,
  section 13, cost). Carried into WP3, WP13 and WP5, whose proofs no longer ask one digest of two different
  committed ticks. The glossary gains "game's tick", proposed.
- 2026-11-27 Finding 2 (the first solo game's replay): there is no unchecked way to play. A one-file game
  keeps its rules in a `mod rules { .. }` block of that file, which the check lifts out and builds alone,
  so examples A and B record, replay and go online as written. A game with no `rules` module is a sketch a
  test steps, and `run()` and every `cargo play` command refuse it with the step to take, so D1 holds for
  every game played (sections 1, 12, 13 and 14, examples A and B). WP2.10 gains the one-file layout and the
  sketch, WP3 carries `run()`'s refusal, and WP13 carries example A recorded on the Mac and replayed in
  Chrome. The plan's assumption is rewritten to match.
- 2026-11-27 Finding 3 (two players and one entity): an entity is one player's character at most, and the
  `Player` component, which the SDK alone gives and takes, says whose. `possess` returns whether this call
  made the entity the player's character: false, with nothing changed, for another player's character, the
  player's own or a despawned entity (section 5, the mistakes table). WP2.1 observes both players'
  characters, components and control after a refused `possess`, and WP2.4 holds the compile errors for a
  `Player` component a game writes. In the extension map a launched thing no longer carries the SDK's
  `Player` component, so the rule holds there too.
- 2026-11-27 Finding 4 (the bound on WP1.5): the guard refuses a type of any crate but `thrum`, its derive
  crate and Rust's standard library, wherever it stands in a public signature and inside a standard
  container, and allows Rust's own types and a generic parameter for the game's own. WP1.5 is written again
  with seven controls: five the check must fail, an `Option` of an engine type among them, and two it must
  pass, `Option<Entity>` and a generic `&T` (section 1, proof). Its check command is unchanged.
- 2026-11-27 Finding 5 (a recovered press and the shooter's view): a button marked `#[seen]` sends the view
  at its newest press with the set's latest values, standing until the next press as its count does, so a
  press read from a later frame comes with its own picture. `space.as_seen_at_press(player, ACTION)` gives a
  rule that view (the entry, depth 3's row in section 11).
- 2026-11-27 Finding 6 (a teleport in a remembered layer): the memory marks each cut, and a collider whose
  newer remembered tick is a cut stands at its new place whatever the fraction, as the picture showed it
  (the entry).
- 2026-11-27 Finding 7 (room for a loading seat's save commands): the set's `Heading` and `#[sent]` values
  travel in one action, so a sampled frame always leaves seven slots for commands (section 5), and a seat
  still loading sends idle frames with no action, so all eight take pieces (the entry).
- 2026-11-27 Suggestions, both taken because each example would otherwise teach a mistake: the bolt keeps who
  fired it and leaves that hull out of its sweep, with the depth sentence corrected to say depth 3 is asked
  for in the hit rule; and the lobby's `Setup` has a written `Default` that names a declared arena.
- 2026-11-27 Items changed: WP1.5, WP2.1, WP2.4 and WP2.10. No check command changed. Planner's choices in
  this revision, each for the owner to overturn: a sketch is refused by `run()`, a refused `possess` returns
  false and does not refuse the tick, a game cannot write the `Player` component, and presses read in one
  tick share the newest one's view.
- 2026-11-27 Round 3: four agents each wrote a whole game and judged the core and the extension map apart
  (reports and source in the main checkout under `tmp/sdk-dogfood-3/`). Three would lock the core in
  (confidence 7, 8 and 7). The online swarm game's author would not (7), because no save was a guest's to
  keep. The lead asked for the core frictions they rank to be fixed at shape depth, and for one reference
  place for every public type's methods, keeping the owner's aim of 27 November, quoted above: "Out goal is
  to lock in the shape of the SDK, not design every feature."
- 2026-11-27 **Round 3 read the design of 26 November, not the rewrite of 27 November**: the main checkout
  the four worked in had not been brought up to it. So each ranked friction was checked against the rewrite
  first, and part was answered there already
  ([the notes](../../../.orchestrate/evidence/GAME-SDK/round-3-revision.md)). The next round should read the
  design from a checkout that has it.
- 2026-11-27 The reference: the Names blocks of the core's sections are gathered into one chapter with an
  index, every public type once with its methods, and each section points at it. K10 now points at it.
- 2026-11-27 Core changes from the five items the lead named, each for the owner to overturn: every save has
  an owner, the match's or a player's profile; a frame function reads the tick's clock through `Tick` and
  `Frame`; what a tile map places is made by a spawn function; a launched thing's function has the control
  function's contract; and a capture needs no tick. Whether the first release builds a profile stays scope
  question 11.
- 2026-11-27 Further core changes, each from a report that showed the core fighting a genre: `then_run`, the
  `Disabled` filter, `pair_mut`, a lobby setup that stays a resource, `face`, the pointer following the input
  context, an offer with no expiry, and example D with dying kept on the hero. The notes list each with its
  report, and what was left as a missing feature.
- 2026-11-27 Items changed to match: WP1.1 (`pair_mut`), WP1.2 (`then_run`, and `left`, `fraction` and
  `ready_after`), WP1.7 (it now names the reference's group R3) and WP11.2 (the `Disabled` filter and what
  `disable` returns). No check command changed. The proofs later work packages carry are extended in the
  design for WP12, WP3, WP4, WP6, WP13 and WP5. No other module's file changed, and nothing new is asked of
  one. The glossary gains "profile", proposed.
- 2026-11-27 Design review of the round-3 revision asked for changes, with 11 Must Fix findings
  ([the review](../../../.orchestrate/evidence/GAME-SDK/design-review.md)). Each is settled below. No other
  module's file changed, `SESSION` is used as built, and no decision of the owner's is touched.
- 2026-11-27 Finding 1 (a guest's unlock after a reconnect): a machine keeps its own slot equal to what the
  world confirms for its own player, comparing after every tick it commits and every baseline it installs,
  so a best committed while its link was down is written when the baseline arrives. "Its own player" is the
  `Player`, seat and count of joins: a return within the grace is the same one and is reconciled, and after
  the grace a later holder of the seat is another, never written (section 10, the entry, state and owners).
- 2026-11-27 Finding 2 (rules run unchecked through `run()`): the check records each pass against the rule
  sources' digest, the built program carries the digest it was compiled from, and `run()` refuses before the
  first tick without a pass for it. `cargo play` builds the record in, and a build Cargo made alone looks in
  the build folder (sections 1, 8, 12 and 14, risk 13, the open question). WP3 carries the proof: the same
  game runs through `run()` after a pass and is refused after a file read is put in its `rules` module.
- 2026-11-27 Finding 3 (a resumed timer's progress): a `Timer` and a `Cooldown` in a save keep the whole wait
  and the time left, so `left` and `fraction` both come back (sections 4 and 10, R2). WP13 carries the
  reviewer's two timers, with fractions of one half and three quarters, and a cooldown's ring.
- 2026-11-27 Finding 4 (a phase's deadline on resume): a phase's clock is world state fixed on entry, read as
  `Phase::clock()` and kept in a save as a `PhaseClock`. `resume(phase, clock)` carries on with it and asks
  no `lasts` function and no `then_run` already run again (section 4, R2, R3, example G). WP1.2's clause 8
  is written again on a length that came from a function whose answer has since changed, and on `then_run`
  before and after its time; WP13 carries the same through a save.
- 2026-11-27 Finding 5 (the platformer's screen with two heroes): example D's `animate_hero` walks every
  hero and shows a dying one its own clip. The same mistake is taken out of two entries of the extension
  map, skeletal animation and camera rigs, which asked for a `Single` that a second player breaks.
- 2026-11-27 Finding 6 (a bolt whose shooter loses their character): each seat has a turn on the control
  line, the control function where the player has a character and then the function of each thing launched
  from that seat. A thing in flight is stepped in every tick that is not paused, with its launcher's
  character gone, its launcher away or gone, and in a phase the line is not `.during`. The predicting
  machine goes on running its own things ahead by the same rule (the entry).
- 2026-11-27 Finding 7 (the shot a recovered press aimed): a `#[seen]` press carries its view, the tick it
  was sampled for and the set's `Heading` and `#[sent]` values. `space.at_press(player, ACTION)` gives a
  rule the picture, the place the world had the shooter's craft at that tick and the aim, and the rail
  example fires from them. It replaces `as_seen_at_press`. Presses read in one tick still share the newest
  one's (the entry).
- 2026-11-27 Finding 8 (a tile sheet a rule chose): the sheet a load named is in the level's recipe with its
  map, so it is digested, rewound and sent to a late joiner. Built collision is kept by what is solid, so
  two recipes that differ only in a sheet share one (section 7, state and owners, cost). Carried into WP12,
  WP3 and WP5.
- 2026-11-27 Finding 9 (the reference and the starter games): R1 lists what `#[bind(..)]` takes, with
  `Touch::drag_x()`, and a slider's `step` and `unit`; R3 says what `Spawn` and `Bundle` are, and examples A
  and B return `impl Spawn` as every spawn function does; R5 has `SunAngle` and how a frame function sets
  it, and `bar(..).mine(..)`. Every call of examples A to F was then looked up in the reference again.
- 2026-11-27 Finding 10 (a modal in WP3): WP3 proves the pointer's capture and release by switching the
  input context, and the modal that frees and returns it is WP6's.
- 2026-11-27 Finding 11 (a tile's smaller shape): a new smaller entry. The shape is said on the glyph's line
  of the scene's `Tileset`, is the rules' and never the sheet's, and is met by the same move and queries.
- 2026-11-27 Suggestions, both taken: a recording holds the profile a lone player or a host started with
  (section 13), which costs a sentence and closes a real ambiguity; and WP11.2's clause 6 says once which
  queries visit a disabled entity.
- 2026-11-27 Items changed: WP1.2 and WP11.2. No check command changed. Planner's choices in this revision,
  each for the owner to overturn: a game built by Cargo alone runs only after a passed check; a guest who
  returns after the grace keeps what their machine last heard; a launched thing flies on with no launcher;
  and a tile's smaller shape belongs to the scene's tile line. The reference gains `PhaseClock`, `SunAngle`,
  `Spawn`, `Bundle` and the binding sources, each a spelling a builder may change.
- 2026-11-27 Second design review of that revision asked for changes, with 3 Must Fix findings
  ([the review](../../../.orchestrate/evidence/GAME-SDK/design-review.md)). Each is settled below. This is the
  last round, so each fix was read against its finding's own case. No other module's file changed,
  [ruling 1](../ARCHITECTURE.md#seam-rulings-for-game-sdk) stands as written, `SESSION` is used as built, and
  no decision of the owner's is touched.
- 2026-11-27 Finding 1 (a pass that certifies another Cargo configuration): feature-dependent rule selection
  is refused. The game check refuses `#[cfg(..)]`, `#[cfg_attr(..)]` and `cfg!(..)` in the `rules` module's
  sources, on or off, so the same sources compile to the same rules under every feature and a pass still
  applies after a change of one (sections 1, 8, 11 and 12). Ruling 1 is unchanged. WP2.10 gains two refused
  cases and WP3's handoff a run with a feature turned on
  ([the notes](../../../.orchestrate/evidence/GAME-SDK/round-4-revision.md)).
- 2026-11-27 Finding 2 (the rail's origin between two ticks): a `#[seen]` press now carries a fourth thing,
  its own character as the screen was drawing it: the entity, and the pair of ticks it was drawn between
  with how far. The space's memory keeps whose character each remembered collider was.
  `press.character()` is that entity where the memory has it as that player's, and goes on naming it after
  a respawn. `press.place()` is rebuilt by the world between its own two places for that character, with
  `as_seen_by`'s limits for a cut and the memory's edge, the last remembered place for one despawned, and
  the world's places winning over a wrong prediction. They replace `press.place_of(entity)`. The reviewer's
  craft, moving from x = 0 to x = 1 and fired at 0.5, fires from 0.5 (the entry and its rail example).
- 2026-11-27 Finding 3 (a launched thing's action map once its player has gone): its function reads its
  launcher's map while that player is here, what their character is given while they are away, and
  `A::idle()` in every tick once that stay has ended, with every value at zero. A newcomer in the seat
  steers only their own character and shots (the entry; R1 says what `idle()` holds).
- 2026-11-27 Suggestion, taken: WP13's handoff compares the cooldown's ring in a picture at the committed
  tick on both sides, because a picture between two ticks can differ rightly and WP3 already proves that
  movement. It costs a clause.
- 2026-11-27 Item changed: WP2.10, by two cases in its list of refusals. No check command changed. Planner's
  choices in this revision, each for the owner to overturn: no `#[cfg]` in a `rules` module, `#[cfg(test)]`
  included; a press names the craft it was made with, and the game says whether a craft since lost still
  fires; and a launched thing whose player has gone flies under idle input.
- 2026-11-27 Owner, asked "Approve the core and take the recommended scope answers?" after dogfood round 4,
  in which all four games' authors would lock the core in (confidence 7, 7, 8 and 8 out of 10): "I approve
  the sdk plan." Recorded as O7. O3 is met, and builders start with WP1.
- 2026-11-27 The scope questions are answered with the planner's recommendation on each, as approved (O8):
  1 yes, 3D collision queries and a kinematic move, no rigid bodies; 2 yes, gamepads; 3 yes, audio in its
  small form; 4 no, first in the release after; 5 no, after 4; 6 already yes; 7 yes, a host with no window;
  8 skeletal animation stays out and is the first engine module after; 9 no; 10 after the first release;
  11 yes, a profile for every player.
- 2026-11-27 The plan's assumptions as they stood were part of what the owner approved. Each stands as the
  design states it, so its line is deleted from the plan. One is kept, because it is about builders and not
  the design: SDK work packages never take a builder V1 is waiting for.
- 2026-11-27 What the yeses add: WP15 (gamepads), WP16 and WP17 (3D space, with no window and on screen, with
  camera pitch), WP18 (audio) and WP19 (the owner's one sitting with a pad and the sound on); WP13 gains a
  profile for a game played alone, and WP5 a guest's profile and a host with no window. They come after WP7,
  so nothing `SWARM` waits on waits for a module not yet planned. Two engine modules are named in the module
  list, `SIM-3D` (row 38) and `AUDIO` (row 39), with what each owns and what the SDK asks of it. They are not
  planned here: a planner limited to this module may not write another module's folder.
- 2026-11-27 Round 4's four frictions are in the core as additions, with no name or invariant changed
  ([the table](DESIGN.md#what-round-4-found)): a save keeps entities, `Kept<N>`, filled by `Keep` and brought
  back by `entities.respawn(save)`; a spawn's own `.child(..)`, `CharacterSpawned` and `query.under(parent)`;
  `PerPlayer::departed(player)`; and `mover_position` and `remaining_motion` on a `Hit`.
- 2026-11-27 Smaller additions of the same kind taken from round 4's other notes: `LevelPiece` and `remove`;
  `.choices_with(f)` on an offer; `WithDisabled`, tuples of filters, `Option<&T>` and `.disabled()`;
  `.during(phases)` on a rule; a validator that reads other content; a `List` in a scene's fields; what keys a
  `Random` draw; a `teleport` starting a new path; the lobby's `continues`; and harness and reference lines.
  Not taken, as changes of shape: a `List` in a component, a second way into a phase, and a test's scene
  registered from test code. Three went to the map: a character's children run ahead, events from
  `par_each`, and a lobby with the game's own rules.
- 2026-11-27 Entries of the map sharpened from round 4: 3D space (one terrain drawn and stood on, `place`,
  what a contact's surface is, `ground_height`, marks), gamepads (a trigger, `invert_y`), audio (asset
  kinds, volume and pitch, the listener, a pause), navigation, predicted projectiles (`end_at`) and the
  player panel's signature. Example G keeps its relics and wardens with `Kept`.
- 2026-11-27 Items added: WP11.5, WP2.11 and WP2.12. No existing item's statement or check changed, and WP1
  is as it was. Planner's choices in this change, each for the owner to overturn: camera pitch comes with
  3D space; a pad is the Mac's and Chrome's, not the phone's; the owner judges pad and sound once, in WP19;
  kept entities come back under kept parents; and one terrain is drawn and stood on from one source.
- 2026-11-27 Left as they are, because they are other modules' files: `GAME-UI`'s plan still assumes no
  gamepad and `SWARM`'s no sound and no gamepad. The vision and scope document does not carry the approval.
- 2026-11-27 The design review after the approval asked for changes, with 12 Must Fix findings
  ([the review](../../../.orchestrate/evidence/GAME-SDK/design-review.md)). Each is settled below. Every
  change adds names or says what an existing name does in a case it left open; no name and no invariant of
  the core changes, no decision of the owner's is touched, and no other module's file changed.
- 2026-11-27 Finding 1 (a wall slide loses the path the stomp needs): a written position is a straight path,
  and `at.follow(&moved)` gives the entity the move's own legs for the tick, each with the share of the tick
  the move spent on it. `motion_of(entity).sweep()` sweeps those legs, so the courier that slid from (1, 1)
  to (1, 3) meets the watch at (1, 2). The path is the tick's working memory and in no digest (section 7,
  R2, R3, cost, risk 19). The examples move a body with `follow`. Carried into WP12 with the review's numbers.
- 2026-11-27 Finding 2 (an overlapping sweep has two positions): a `Hit` gains `depth`. A path meets a thing
  on the way, begins touching it or begins inside it, and each has one `fraction`, `mover_position`, `depth`
  and `remaining_motion` (the table in section 7). `mover_position` is always where the mover stood, and
  `mover_position + normal * depth` always touches without overlapping. A move that begins inside something
  is not pushed out and goes no deeper. WP2.12 proves the three against a collider and WP12 against the level.
- 2026-11-27 Finding 3 (a kept prefab missing what its spawn must bring): `keep.all` of a `Prefab<N>`
  compiles only into a `Kept` that lists its `N`, and a file that names a prefab its field cannot spawn is
  `Loaded::Failed` (section 10, R2, R3, the mistakes table, risk 18). WP13 proves two rocks with their own
  speeds.
- 2026-11-27 Finding 4 (a reloaded map duplicates or resurrects kept entities): a `Kept` holds the prefab
  names it was filled from, and `respawn` first takes out what stands of those prefabs, so the load places
  the level and the save decides those things, with the load called first (sections 7 and 10).
  `Kept::none()` is for a conversion. WP13 proves the review's two coins.
- 2026-11-27 Finding 5 (the lobby cannot choose the scene a save names): `.continues_with(slot, f)`, where
  `f` is a read-only function of the save that returns the scene, asked on every machine at the tick the
  save lands (section 11, R1). `host().continue_saved()` presses it in a test. Carried into WP5.
- 2026-11-27 Finding 6 (a host with no window has no screen to read its save from): its process reads the
  slot its lobby continues from before the first tick and the world it seeds holds the bytes, so *Continue*
  is shown from world state to the lowest seat, whose press carries no bytes. The world lets them go when
  the lobby is left. `SESSION` is used as built: its host with no window has no slot to send input from. A
  frame function's save and a game's own `restoring` stay with a host that has a screen (sections 10 and 11,
  the entry, the state table). Carried into WP5, with `start_match_with_no_window()`.
- 2026-11-27 Finding 7 (WP11.2 against WP11.5): WP11.2's clause 6 now speaks of an ordinary query, one with
  neither filter, and of the `Disabled` filter. WP11.5 judges `WithDisabled`.
- 2026-11-27 Finding 8 (a terrain with no layer): `LevelMut3d::load(&terrain, LAYER)`. Every solid thing in
  a level is on the layer the rule that put it there named, the recipe keeps it, and `ground_height` takes
  layers. The ray example, example G's `load_the_valley` and what the SDK asks of `SIM-3D` say the same.
  The module list's row for `SIM-3D` points at that text and is unchanged. Carried into WP16.
- 2026-11-27 Finding 9 (a live mesh edit parts the drawn surface from its collision): a running game keeps
  the pair it started with. An edit that changes the cooked collision waits for the next start and is named
  on the console and the panel; one that leaves it as it was is redrawn at once; a test, a replay and every
  machine of a match use the pair they started with (section 10, the 3D entry). Carried into WP17.
- 2026-11-27 Finding 10 (`invert_y` on a `Heading`): `invert_y` takes an `Aim` and nothing else. WP15 proves
  `sensitivity` and `invert_x` on a `Heading`, and WP17 builds and proves `invert_y` with the `Aim`.
- 2026-11-27 Finding 11 (starts counted where voices are bounded): `.limit(n)` bounds the voices of one sound
  playing at once and never how many start. WP18 proves both of the review's cases: forty plays of a
  one-second sound and of a 10 ms one.
- 2026-11-27 Finding 12 (the bolt example threw its contact away): `fly_bolt` ends at the pier with
  `end_at(hit)` and ends a spent fuse with `end()`, and the entry says what each leaves for later rules.
- 2026-11-27 Suggestion, taken: assets are what a screen shows and plays, in section 10 and in the
  glossary's `Content` entry, which is this planner's proposal. It costs a sentence and audio is in the
  first release.
- 2026-11-27 Items changed: WP11.2 and WP2.12, each by its statement. No check command changed. Planner's
  choices in this revision, each for the owner to overturn: a body's bent path is kept only where the game
  says `follow`; `respawn` replaces what stands of the prefabs a save names; a shape's edit waits for the
  next start in a 3D game; and a host with no window continues a run only through the standard lobby.
- 2026-11-28 Design review 10 asked for five changes to the motion contract, all settled in this revision,
  the last before building. [Review](../../../.orchestrate/evidence/GAME-SDK/design-review.md).
- 2026-11-28 Finding 1 (a straight move stopped early loses when it arrived): a leg takes the share of the
  tick it took of the step, and a stopped body rests for the remainder, so a followed path keeps each leg's
  times. What keeps nothing is now a path of one straight leg over the whole tick from the tick's start,
  in section 7, the cost table and the open question. WP12's handoff gains the step of (4, 0) stopped at
  (1, 0), told from the same end written by hand by a `fraction` of one eighth against one half.
- 2026-11-28 Finding 2 (a second followed move erased a ferry's carry): a move followed from where the
  entity stands adds to the path it has, the two over the one tick, so a carry and a step come to the
  straight line between the ends and a rider carried through a coin meets it. A move followed from another
  place still begins a new path. Section 7 has the two lines a rider writes, and WP12's handoff the proof.
- 2026-11-28 Finding 3 (`remaining_motion.dot(normal)` misread a moving target): a `Hit` has
  `approach_speed`, the speed the two were closing at along the normal, from the leg each was on. Section
  7, R3, and WP2.12's clause 7 with the review's hero and watch.
- 2026-11-28 Finding 4 (exact tangency cannot be held on a diagonal): touching means within the contact
  tolerance, either side, and `depth` is exactly zero inside it. In 2D it is a fraction of the numbers
  compared, at most 2⁻¹⁶, which WP2's builder measures and sets and `thrum-sim2d` holds; in 3D it is
  `SIM-3D`'s own constant. Overlap queries keep their exact test. WP2.12's clause 5 is the review's two
  circles at (0, 0) and (1, 1). The glossary gains the name.
- 2026-11-28 Finding 5 (the bolt gave `end_at` a ray's hit): `fly_bolt` sweeps the bolt's own sphere, and
  `end_at` keeps the hit's `fraction` as the moment the path ends, so a hull that crosses the bolt's last
  place later in the tick is not hit. The entry stays after the first release.
- 2026-11-28 Suggestion, taken: `.continues_with(..)` is in section 1's list of functions checked as rules,
  because section 11 and WP5 already refuse one defined outside the rules module.
- 2026-11-28 Item changed: WP2.12, by its statement, written again whole. No check command changed.
  Planner's choices in this revision, each for the owner to overturn: two moves followed in one tick run
  together and are not taken one after the other; `approach_speed` is in units a second; and the overlap
  queries do not use the tolerance.

## WP1 build

- Implemented the headless SDK over the registered simulation. The opt-in rule boundary applies staged
  structural edits within the engine transaction. The legacy schedule retains its tick-end edit timing.
  The SDK uses the runtime's revision, digest and history for rules, scene values, phases and their clocks.
- Mac verification covered written order, access diagnostics, transition requests, clock resume, allocation
  reuse, replay, and both returned errors and unwinding panics. The Rockfall and Swarm headless examples are
  compiled in the SDK integration tests. The public-surface guard uses resolved Rustdoc signatures and seven
  controls, and the repository check runs the guard and SDK doc tests.
- The worked examples' headless substitutes are tuples, a plain `Position`, and integer constructors for
  portable positions and speeds. Rockfall's headless finish rule uses a dodged rock in place of the later
  collision query. Its cadence and score transfer retain the design's form. The phase methods remain
  chainable after `then`, as in Swarm. Integration tests spell parameter lifetimes as `'_` for the workspace
  lint. `TestSetup::seed` provides the seeded harness path in R6 for the WP1 replay check.
- The evidence is under [WP1](../../../.orchestrate/evidence/GAME-SDK/WP1/). The allocation probe covers two
  small warmed CPU fixtures. Crowd-scale cost and cross-target rule parity are scheduled for WP2.
