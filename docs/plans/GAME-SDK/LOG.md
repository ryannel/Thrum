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
