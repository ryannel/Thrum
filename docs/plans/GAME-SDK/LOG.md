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
