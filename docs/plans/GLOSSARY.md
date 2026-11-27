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
menu or a match. One scene runs at a time. A scene is a Rust type whose fields are what it is entered with, and a
function that describes it. Inside the engine, "scene" also names what the renderer draws (the 2D and 3D scene
kinds); say "render scene" for that when the two could be confused.

- **Source** — the field's term (Godot, Unity).

Proposed: the planner chose this meaning for game code.

### Rule

A function that moves a game's world on by one tick. Rules are the only code that changes the world. A scene's
rules run in the order the scene function writes them, and what one rule did is what the next one sees. Other
engines call it a system. A game's rules module (`src/rules.rs`) holds its rules and is checked against the
deterministic rules profile.

- **Source** — Thrum's own use since `SIM-2D` (rule package, rule set).

Proposed: the planner chose "rule" over "system" and "behaviour" for game code.

### Frame function

A function that runs once for each picture drawn. It reads the world as shown and sets what only this screen
shows: the camera, the interface, effects, how a look is dressed. It cannot change the world.

- **Source** — coined for the game SDK; other engines split the same way (Godot's `_process`, Unity's `Update`).

Proposed: the planner chose the name.

### Action

A named thing a player can do, such as move or dash: a field of an action map, bound to keys, the pointer and
touch. Rules read actions for each player and never read a key. A local action is one only the player's own
screen reads, such as pause.

- **Source** — the field's term (Godot's input map, Unreal's input actions).

Proposed: the planner chose it for the game SDK.

### Command

A message a player sends once, such as a menu choice. A command that is admitted reaches the rules once, after
the same player's earlier commands, and none reaches them twice. One on its way when a link drops may be lost.
In a match it is the session's ordered command. It is not a deferred spawn, which other engines call a
command.

- **Source** — `SESSION`'s term.

Proposed: the planner carried it into the game SDK.

### Look

A component the engine draws: a sprite, a shape, a model, a light.

- **Source** — coined for the game SDK.

Proposed: the planner chose the name.

### Placement

In a 3D game written on the game SDK, a model a scene stands somewhere once, with no entity, such as a valley or
a building. It has no gameplay identity and nothing moves it. It has the same model and material an entity's
look has, so giving the model to an entity is how it comes to move.

- **Source** — the field's term (static mesh in Unreal, static body in Godot), in plain words.

Proposed: the planner chose the name, in place of "scenery".

### Setting

A named value a game shows on the tuning panel, in the Mac window and on the Chrome page: a slider, a toggle, a
text or a readout. `CREATOR` calls the same thing a typed control. A shared setting is part of the world, read
by rules and changed by a command; a local setting belongs to one player's screen and no rule reads it.

- **Source** — the field's term.

Proposed: the planner chose "setting" for game code, and keeps "action" for what a player does.

### Control function

A function that is one player's character's whole movement step. A scene names it on the line that gives each
player a character, and it runs there in the scene's written order, once for each player's character, with that
player's input. In a match the engine runs it ahead for the local player when everything it reads can be
predicted, so their character answers before the host confirms the input. `SESSION` calls what it is handed to
a predicted controller.

- **Source** — coined for the game SDK; the field's term for the technique is client-side prediction.

Proposed: the planner chose the name, in place of "per-player rule".

### Phase

In a game written on the game SDK, one stage of a scene, with its own rules and its own enter and exit
functions, such as a countdown, a wave or an intermission. A scene is in one phase at a time.

- **Source** — the field's term (a state, in Bevy and in state machines), in plain words.

Proposed: the planner chose "phase" over "state", which here means everything the world holds.

### Prefab

A named, reusable thing a game spawns: its components with their defaults, its children and its variants.
Written in Rust, with no editor.

- **Source** — the field's term (Unity).

Proposed: the planner chose it for the game SDK.

### Content

Typed records a game's rules read and cannot change, such as waves, levels and enemies' numbers. Each is loaded
from a file and named by a stable ID. Assets are what is drawn; content is what the rules read.

- **Source** — the field's term (data tables in Unreal, resources in Godot).

Proposed: the planner chose the name and the split from assets.

### Offer

A question a game's rules put to one player or to all of them, answered once, such as a choice of upgrade or a
vote. It is world state, so it survives a lost connection.

- **Source** — coined for the game SDK.

Proposed: the planner chose the name.

### Input context

In a game written on the game SDK, a name for which action maps a player's keys, pointer and touches drive at
the moment, such as play or a shop. One key can then do one thing in play and another in a menu while the
world keeps ticking. A screen has one input context at a time, and it is that screen's own state: no rule
reads it.

- **Source** — the field's term (Unreal's input mapping context, Unity's action map switching).

Proposed: the planner chose it for the game SDK.

### Pause

In a game written on the game SDK, the game's clock standing for everyone in it, as world state: timers and
rules wait while ticks still commit. Asking for one and ending one are input, so a pause is the same alone and
in a match. It is not a menu: the key that asks for a pause is a local action, and a menu a player opens
without one is their own screen's and stops nothing.

- **Source** — the field's term, narrowed to the shared state.

Proposed: the planner chose this meaning, to keep a paused game apart from one player's open menu.

### Game's tick

In a game written on the game SDK, the count of ticks the game's rules have lived through, which is what `Tick`
gives a rule. It stands still through a [pause](#pause). The engine's committed tick goes on counting every tick
the world commits, and a digest and a recording go by that one.

- **Source** — the planner's term. The field says simulation time or game time for the same thing.

Proposed: the planner chose it, to keep the clock a rule reads apart from the engine's committed tick.

### Profile

In a game written on the game SDK, the saved value each player keeps on their own machine, such as a best
score or an unlock, beside whatever the match saves on the host's. In the world it is one value for each
player, which rules write and each machine saves for its own player alone. Defined in
[the game SDK's design](GAME-SDK/DESIGN.md#10-content-assets-saves-and-settings).

- **Source** — the field's term (a player profile, Unreal's local-player save).

Proposed: the planner chose the name, to say whose a save is.

### Extension map

The part of [the game SDK's design](GAME-SDK/DESIGN.md#the-extension-map) that says how each feature the SDK
does not build yet would be added: where it plugs in, the names it adds and the game code it allows. An
**extension point** is one of the eight kinds of place a feature may plug in, such as a parameter a rule may
take or a line of a scene. An entry adds names and changes none of the design's core.

- **Source** — coined for the game SDK; "extension point" is the field's term.

Proposed: the planner chose the names, in place of "reserved shapes".

### Output scale

At full screen on a fast display, the share of the display's pixel size, on each axis, at which the engine
reconstructs and presents the picture: one of 100, 85, 75, 67 and 50%. The display scales that picture up to
fill the panel. The **output extent** is the pixel size it gives. It is not the render scale, which is the
governor's and sets how many pixels are drawn inside the output extent before reconstruction. Defined in
[APPLE-PRESENTATION's design](APPLE-PRESENTATION/DESIGN.md#the-output-scale-wp18).

- **Source** — the owner's "the 120 upscale for full screen", 27 November 2026; the field's terms are output
  resolution and render resolution.

Proposed: the planner chose the name.
