# Thrum glossary

The agreed terms for planning and building Thrum.

### Goal

The one long-term aim Thrum is working toward, set in the vision and scope document.

- **Source** — orchestrate glossary.

### Vision and scope document

The document that sets Thrum's goal, what the current release covers, how it will be judged and the decisions
that guide the work. It lives in [VISION.md](VISION.md).

- **Source** — orchestrate glossary.

### Milestone

A significant point on the way to the goal that the user cares about. Thrum's [milestones](VISION.md#the-milestones)
include G0, G2S, G3a through G7 and the V1 exit. A milestone is reached when the work it depends on is complete.

- **Source** — orchestrate glossary.

### Work package

A piece of a module that one builder can finish in one session. Its done-when items specify what must be true
and how to prove it. Use the full phrase. Write the work package ID as `WP7` or qualify it by module,
as in `CLOUDS:WP7`. The third item in that work package is `WP7.3`.

- **Source** — orchestrate glossary.

### Task

What one agent session is asked to do. A builder's task names the work package to build. Planning and review
sessions also receive tasks.

- **Source** — orchestrate glossary.

### Game SDK

The public interface a game is written against: the crate `thrum`, its commands, guides and templates. A game
depends on it and on no other engine crate. Planned by the [GAME-SDK](GAME-SDK/DESIGN.md) module.

- **Source** — the owner's request of 25 November 2026; the field's term (software development kit).

Proposed: the planner chose the name and the crate's name, `thrum`.

### Scene

In a game written on the game SDK, one part of the game with its own setup, rules and look, such as a level, a
menu or a match. One scene runs at a time. Inside the engine, "scene" also names what the renderer draws (the 2D
and 3D scene kinds); say "render scene" for that when the two could be confused.

- **Source** — the field's term (Godot, Unity).

Proposed: the planner chose this meaning for game code.

### Rule

A function that moves a game's world on by one tick. Rules are the only code that changes the world, and a
scene's rules run in the order the scene lists them. Other engines call it a system. A rule crate is a crate
that holds only rules and passes the deterministic rules profile.

- **Source** — Thrum's own use since `SIM-2D` (rule package, rule set).

Proposed: the planner chose "rule" over "system" and "behaviour" for game code.

### Frame function

A function that runs once for each picture drawn. It reads the world as shown and sets what only this player
sees: the camera, the interface, effects. It cannot change the world.

- **Source** — coined for the game SDK; other engines split the same way (Godot's `_process`, Unity's `Update`).

Proposed: the planner chose the name.

### Action

A named thing a player can do, such as move or dash, bound to keys, the pointer and touch. Rules read actions
for each player and never read a key.

- **Source** — the field's term (Godot's input map, Unreal's input actions).

Proposed: the planner chose it for the game SDK.

### Command

A message a player sends once, such as a menu choice. It reaches the rules exactly once, in order. In a match
it is the session's ordered command. It is not a deferred spawn, which other engines call a command.

- **Source** — `SESSION`'s term.

Proposed: the planner carried it into the game SDK.

### Look

A component the engine draws: a sprite, a shape, a model, a light.

- **Source** — coined for the game SDK.

Proposed: the planner chose the name.

### Setting

A named value a game shows on the tuning panel, in the Mac window and on the Chrome page: a slider, a toggle, a
text or a readout. `CREATOR` calls the same thing a typed control.

- **Source** — the field's term.

Proposed: the planner chose "setting" for game code, and keeps "action" for what a player does.
