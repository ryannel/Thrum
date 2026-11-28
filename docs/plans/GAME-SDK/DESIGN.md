# Game SDK: one crate a game is written against

27 November 2026. Revised that day so its core shape can be locked: the core is stated once, with its
invariants, [the reference](#the-reference) gives every public type its methods in one place, and
[an extension map](#the-extension-map) says how each feature it does not build would be added. It replaces the
design of 26 November, and was revised again that day from [a third round of games](#what-round-3-found).
**The owner approved it that day** ([O7](#decisions)) and answered
[the scope questions](#scope-questions-for-the-owner) (O8), and [a fourth round of games](#what-round-4-found)
added to the core without reshaping it. On 28 November, after three failed reviews of WP1,
[a rule's identity](#3-rules-and-the-tick) and [what a resumed phase restores](#4-scenes-phases-time-and-pause)
were settled, as the [log](LOG.md#wp1-diagnosis) records, and a design review that day closed seven places
in the two, which [the log](LOG.md#design-review-of-28-november) lists. [Plan](PLAN.md) · [done-when file](DONE.json) ·
[log](LOG.md) · [module list](../ARCHITECTURE.md#track-7-the-game-sdk) · [glossary](../GLOSSARY.md) ·
[the creator guide](../../design/ENGINE-EXPERIENCE.md), whose ambition this module builds ·
[the performance policy](../../design/AI-PERFORMANCE.md#performance-investment-from-the-first-design) ·
[the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md)

**The owner approved this design on 27 November 2026** (O7), so builders start, with WP1. The shape that is
locked is [the game at a glance](#the-game-at-a-glance) (one page), [the core](#the-core) and
[the extension map](#the-extension-map). [The worked examples](#worked-examples) are the evidence that the
core reads as a game, and [the scope answers](#scope-questions-for-the-owner) say what the first release
builds.

## The request

The owner, 25 November 2026: "What I would like you to plan is a high end, intuitive, expressive, readable, SDK
that game developers can use to build new games. this should be what V1 uses ideally and what we will use to
build the swarm game. It's VITAL that we get this right as it's the entire interface that developers and coding
agents will use to build a variety of games. It's the front of the product we're building. So it needs to be as
intuitive, well documented and easy to use as possible." And: "V1 first, Rust only, I'll approve the design."

The owner, 26 November 2026: "We're looking for something that's readable, expressive, intuitive and powerful.
Don't optimise at all for reducing the amount of re-work needed. Let's just find the best possible design."

The owner, 27 November 2026, after four more agents each wrote a whole game against the design: "Let's just be
clear on the dog fooding. It seems like it's complaining about missing features, that's fine. But what we
should be very confident in is our approach and shape of our core SDK design. If we're missing features, that's
fine, as long as we know how we would add them in a delightful way. So let's flesh the plan out enough so
that's clear. And let's bring multiplayer back into scope. Out goal is to lock in the shape of the SDK, not
design every feature."

The outcome: a developer or agent who has never seen Thrum builds a small 3D game, a small 2D game and a
networked game from the SDK's documentation alone, and the code reads as plainly as the game it describes. One
structure and one vocabulary across 2D, 3D, single-player and multiplayer; good-looking defaults; mistakes that
explain themselves; every public item documented with an example. And a reader can judge the SDK's shape with
confidence, because the design says how every feature it does not build would be added without bending it.

## Decisions

Each is the owner's. A builder builds what it says and nothing it rules out.

**25 November 2026.**

- **O1 V1 first.** The V1 exit does not wait for the SDK. V1 lands on today's API, SDK work packages then move
  the showcase onto the SDK, and [the swarm game](../../design/NEXT-GOAL-SWARM-GAME.md) is built on it from its
  first line. No work package on the road to the exit waits on this module.
- **O2 Rust only.** Games are written in Rust. There is no scripting layer.
- **O3 The owner approves the design** before any build. Met on 27 November 2026 (O7).
- **O4 First-release scope** ("Planner's default"): 2D queries and sprite animation are in; rigid bodies,
  skeletal animation and audio are out. The owner changed it on 27 November 2026 (O8): audio is in, in its
  small form, and so are 3D collision queries with a kinematic move. Rigid bodies and skeletal animation stay
  out.

**26 November 2026.** The owner answered "Yes I agree" to all four decisions and all ten recommendations of
[the review](../../../.orchestrate/evidence/GAME-SDK/owner-review-2026-11-26.md).

- **D1 Input-only replay, on every device.** A Thrum game's shared rules give the same result from the same
  admitted input on every device, a game played alone included. Each player's presentation uses ordinary
  floats. A state-replicated mode, for private information or an outside simulation, is not built.
- **D2 Authored objects are Rust.** A prefab with editable defaults, typed assets and optional children, and
  one model vocabulary whether a thing is placed or moves. No visual editor.
- **D3 The first character is a player's.** The first steerable thing in a first game belongs to a player,
  stays responsive in a match against fixed scenery, and enters a standard solo, host and join lobby with its
  movement function unchanged. Dynamic interactions use confirmed state.
- **D4 Typed content IDs from day one.** Gameplay content is typed, validated and named by stable IDs from the
  first release. Versioned saves that a later build loads, and data packs a game ships without recompiling,
  are year-one commitments, advertised when they hold.
- **R1** The first playable thing is a character you control. **R2** Written order tells the truth: a rule's
  finished edits, structural ones included, are what the next rule sees. **R3** A game defines a reusable
  thing, changes one detail and attaches another thing. **R4** Portable numbers only where the game's result
  depends on them. **R5** Phases, timers and resources have an owner and a lifetime. **R6** Static and moving
  objects share one visual vocabulary. **R7** Player input, local bindings and prediction eligibility are
  separate. **R8** A co-op choice and a lobby are ordinary things to author. **R9** It is easy to prove the
  game the author meant. **R10** Authored data and saved progress have stable identities.

**27 November 2026.** The owner's words are in [the request](#the-request).

- **O5 The shape is what is locked, not the feature list.** The core is settled and stated in one place. A
  feature may be missing from the first release; how it would be added may not be. Every feature a dogfood
  game missed, or the first release leaves out, has an entry in [the extension map](#the-extension-map) that
  adds names and changes none.
- **O6 Multiplayer is in the first release for 3D games as well as 2D.** Scope question 6 is answered yes.
  Predicted projectiles and contact, and hits judged as the shooter saw them, are answered in O8.
- **O7 The design is approved.** After [round 4](#what-round-4-found), in which the authors of all four games
  would lock the core in (confidence 7, 7, 8 and 8 out of 10), the lead asked: "Approve the core and take the
  recommended scope answers?" The owner, 27 November 2026: "I approve the sdk plan." So the core is locked,
  O3 is met, and builders start with WP1.
- **O8 What the first release builds**, which is the planner's recommendation on each of
  [the scope questions](#scope-questions-for-the-owner), as the owner approved them in O7:
  1. 3D collision queries and a kinematic character move are in, with no rigid bodies.
  2. Gamepads are in.
  3. Audio is in, in its small form.
  4. Predicted projectiles and predicted contact are not. They come first in the release after.
  5. Hits judged as the shooter saw them are not. They follow 4.
  6. Multiplayer for 3D games is in (O6).
  7. A host with no window is in.
  8. Skeletal animation stays out, and is the first engine module planned after the release.
  9. A text field and scrolling lists are not.
  10. Levels from an editor's file come after the first release.
  11. A profile for every player, kept on each player's own machine, is in.

**What seven games found.** On 26 November three agents each wrote a whole game against the design as the
review amended it, and the design was rewritten from their reports. On 27 November four more wrote a 2D
platformer, a 3D exploration game, an online 2D swarm survivor and an online 3D arena game against that
rewrite, with scope questions 1 to 3 taken as yes (reports and source in the main checkout under
`tmp/sdk-dogfood-2/`). The swarm survivor's author would choose the SDK (7 out of 10). The other three would
not yet: the platformer (7), the exploration game (5) and the arena game (2), which the first release could
not then put online at all. All seven praised the same core: rules in written order, the typed wall between
rules and the screen, phases, and tests with no window. Round 2 added a controlled character that is
predicted with no code, bots as players and content with validators. Most of what round 2 asked for is a
missing feature, and is in the extension map. Where a report showed the core itself fighting a genre, the
core is changed:

| What fought the game | The core now says | Where |
|---|---|---|
| A talk box that pauses the world could never close, a pause menu could not save, and a pause meant one thing alone and another in a match | A pause is one thing everywhere: the game's clock stands while ticks still commit, so commands, answers and requests are admitted, a save is captured, and a rule marked `.even_when_paused()` runs | [4](#4-scenes-phases-time-and-pause) |
| A save resumed into the scene's first phase, ran the saved phase's enter functions again and restarted its clock | A scene's enter function resumes a phase with its clock, and no phase is entered | [4](#4-scenes-phases-time-and-pause) |
| A ray saw through a hill, and a bolt needed every pier twice, because the level and the colliders were asked separately | Rays, sweeps and moves asked of a space meet the level too. The level alone is what a match runs ahead. One hit type, with normals and a sweep that starts in contact defined | [7](#7-space-and-the-level) |
| A shape written once for the collider and again for the move, with its dimensions guessed | One `move_shape`, in 2D and 3D, given the character's own collider | [7](#7-space-and-the-level) |
| A shot kept its pilot's seat, so whoever sat there next earned its point | A `Player` names one stay in a seat, as an `Entity` names one life of a slot | [5](#5-players-actions-and-control) |
| A spawned thing could not be attached, stored or parented by the rule that spawned it | `spawn` returns its `Entity` at once, and a spawn may say `.under(parent)` | [2](#2-the-world-entities-components-resources-and-prefabs) |
| A cooldown ring, a fade timed to a respawn and a clock on the status display each needed a rule that copied time into a resource | A frame function takes `Tick` and the level, read-only | [9](#9-frame-functions-looks-and-the-interface) |
| Nine parameters on every weapon rule, and ten arguments on the helper they shared | `#[derive(Params)]`: a struct of parameters is a parameter | [3](#3-rules-and-the-tick) |
| Cards chosen mid-fight stole the walk stick, and three level-ups in a second had to be queued by hand | Outside a modal the interface takes no direction input, and a player's open offers queue | [6](#6-commands-offers-and-requests), [9](#9-frame-functions-looks-and-the-interface) |
| Enemies meant to appear off screen appeared on a wide one, because rules cannot read a camera | `Camera2d::exactly(w, h)` shows that rectangle and no more, so a rule may assume it | [9](#9-frame-functions-looks-and-the-interface) |
| A 3D game could not be played online | Nothing in a match knows 2D from 3D, and the first release proves it (O6) | [11](#11-multiplayer-an-opt-in-over-input-only-networking) |

### What round 3 found

On 27 November four more agents each wrote a whole game and judged the core and the extension map apart: an
online 2D swarm survivor, a 2D platformer, an online 3D arena game and a 3D exploration game (reports and
source in the main checkout under `tmp/sdk-dogfood-3/`). Three would lock the core in, with confidence 7, 8
and 7 out of 10. The swarm survivor's author would not (7), because no save was a guest's to keep.

**They read the design of 26 November, not the rewrite above.** The main checkout they worked in had not
been brought up to it. So each friction they ranked was checked against the rewrite. Part was answered there
already, such as a frame function that takes `Tick`, a struct of parameters and a pause that still commits
ticks. The rest is changed here. And they guessed 10 to 55 names each, many of which the design gave in a
section they did not look in, so [the reference](#the-reference) now holds every type once, with its methods.

| What fought the game | The core now says | Where |
|---|---|---|
| A guest earned an unlock and no save was theirs to write | Every save has an owner. `saved` and `save` are the match's, on the host's machine. A **profile** is each player's, on their own machine, written from the result the rules confirm for that player | [10](#10-content-assets-saves-and-settings) |
| A ring could not fill between ticks, and a cooldown could not say how far along it was | A frame function reads the tick's clock two ways: `Tick`, exactly as a rule read it, and `Frame`, the same clock at the picture's own moment. `Timer` and `Cooldown` have `left` and `fraction` | [9](#9-frame-functions-looks-and-the-interface) |
| What a tile map placed had no values, so a prefab held a zero that a second function filled in | `.spawns(glyph, ..)` takes a spawn function given `Placed`, as a player's spawn is given `Joined`. An editor's object brings its fields the same way, and a level may be several maps | [7](#7-space-and-the-level) |
| A predicted shot had one line of syntax and no contract | The function that moves a launched thing has the control function's contract: the same table, its place on the control line, and how the thing ends | [The entry](#predicted-projectiles-and-contact) |
| A pause menu alone had to end the pause to save | A capture is read-only and needs no tick: it reads the world as last committed, and the menu stays open | [10](#10-content-assets-saves-and-settings) |
| One hero's death stopped every hero, because the platformer example used the scene's phase | A phase is for what happens to everyone. Example D keeps dying on the hero, so a second player changes no rule | [4](#4-scenes-phases-time-and-pause), [D](#d-a-precision-platformer) |
| A phase with a length could end only in another phase | `.then_run(rule)`: a rule run once when the time is up, which may ask for a scene | [4](#4-scenes-phases-time-and-pause) |
| A disabled thing could not be found by the rule that brings it back | The `Disabled` filter, and `disable` and `enable` return whether this call made the change | [2](#2-the-world-entities-components-resources-and-prefabs) |
| A ram needed two passes over one query | `Query::pair_mut(a, b)` | [R3](#r3-what-a-rule-may-take) |
| The lobby could not list content, and the host's setup was gone after the first scene | The host's panel and `starts_with` take read-only parameters, and the setup stays a game resource | [11](#11-multiplayer-an-opt-in-over-input-only-networking) |
| A respawn could not face the arena, because a heading is the player's machine's | `PlayersMut::face` | [5](#5-players-actions-and-control) |
| Nobody owned the captured pointer when a menu opened | The pointer follows the input context | [5](#5-players-actions-and-control) |
| An earned level-up ended with its phase | An offer with no expiry stays open until it is answered | [6](#6-commands-offers-and-requests) |
| The camera had no answer for a player with no character | A following rig holds its view, and what a frame function adds plays on top of it | [9](#9-frame-functions-looks-and-the-interface) |

### What round 4 found

On 27 November four more agents each wrote a whole game against this design and judged the core and the
extension map apart: a 3D exploration game, an online 2D swarm survivor, an online 3D arena game and a 2D
platformer, with scope questions 1 to 3 taken as yes (reports and source in the main checkout under
`tmp/sdk-dogfood-4/`). **All four would lock the core in**, with confidence 7, 7, 8 and 8 out of 10, and
they guessed 6 to 35 names each, where round 3's authors guessed 10 to 55. Each named the one change that
would raise its confidence most. **Each is an addition: it adds names, and changes no name and no invariant
of the core** (O5).

| What fought the game | The core now says | Where |
|---|---|---|
| A save was a hand-written mirror of the world: each kept thing was written as its component, again as a field of the save and again as the lines that respawn it, and a capture that left out the disabled wardens compiled and lost them (the exploration game) | A save keeps entities for the game. A field of the save, `Kept<N>`, holds the entities of the prefabs the capture names, each by its prefab's stable name with the components listed, and one call respawns them. The save is still the game's own versioned type | [10](#10-content-assets-saves-and-settings) |
| What a character owns was a hand-written walk of the tree in four places, and a keeper needed a flag and a second rule to be given its first weapon (the swarm survivor) | Ownership reads both ways. A spawn brings children chosen where it is made, so a spawn function returns a character whole; `CharacterSpawned` is the moment a character appears; and `query.under(parent)` reads a parent's children | [2](#2-the-world-entities-components-resources-and-prefabs), [5](#5-players-actions-and-control) |
| A rule that read `PlayerLeft` could no longer read the leaver's score, so each game kept its own table beside `PerPlayer` (the arena game, the swarm survivor) | `PerPlayer::departed(player)`: for the rest of the tick a player leaves in, what their seat held as they left | [5](#5-players-actions-and-control) |
| A stomp was easy to detect and hard to resolve: a sweep gave the point touched and a fraction, so the hero's place was worked out by hand and the rebound began a tick late (the platformer). A bolt's centre at a pier was worked out the same way (the arena game) | A hit says where the mover itself stood at first contact, and what motion it had left | [7](#7-space-and-the-level) |

**Smaller additions of the same kind**, each from a report's ranked friction or its list of guesses:

| What fought the game | The core now says | Where |
|---|---|---|
| A portcullis dropped and could never rise, because a level's recipe only grew (exploration) | What a rule adds to the level it can take away: an addition returns a `LevelPiece`, and `remove(piece)` takes it out | [7](#7-space-and-the-level) |
| Three hands of cards were drawn against one build, because an offer's choices were fixed when it was opened (swarm) | `.choices_with(f)`: a read-only function asked when the offer reaches the front of its player's queue | [6](#6-commands-offers-and-requests) |
| A rule could not reach disabled and enabled things together, a query had no optional part, and a sleeper took a second function to put to sleep (swarm, exploration) | The `WithDisabled` filter, a tuple of filters, `Option<&T>` in a query, and `.disabled()` on a spawn | [2](#2-the-world-entities-components-resources-and-prefabs), [R3](#r3-what-a-rule-may-take) |
| Rules two phases share sat under a condition written by hand (swarm, exploration) | `.during(phases)` on a rule or a group, with one phase or several | [3](#3-rules-and-the-tick) |
| A validator could not check a level against the enemies it names (swarm, exploration) | A validator may take `Content<U>` after its record | [10](#10-content-assets-saves-and-settings) |
| A scoreboard handed to the next scene was flattened to an array, and a wall's side was kept as a number (swarm, platformer) | A scene's fields hold a `List`, and a `Side` is world state as any name is | [2](#2-the-world-entities-components-resources-and-prefabs) |
| Whether two hands dealt in two paused ticks are the same hand was not said, and a game that forgot `.even_when_paused()` hung on its first level-up with no message (swarm) | What keys a draw is stated, and a waiting answer under a modal's pause is reported once | [3](#3-rules-and-the-tick), [12](#12-mistakes-that-explain-themselves) |
| What a sweep sees across a respawn was not said (platformer) | A `teleport` starts a new path | [7](#7-space-and-the-level) |
| A host could not continue a saved journey with a friend through the standard lobby (exploration) | `Lobby::standard().continues::<T>(slot)` | [11](#11-multiplayer-an-opt-in-over-input-only-networking) |
| A match in a test took no fixtures, a peer could name its craft and not its pilot, a test could not read content or name a fixture's ID, and a picture could not be let run (all four) | `start_match()`, `my_player()`, `content`, `test_id`, edits a whole table takes, and `advance` | [R6](#r6-tests-and-errors) |
| Names guessed that the reference should have given (all four) | A phase is `Data`; the data derives give `Clone`, `PartialEq` and `Debug`; `insert` replaces; a struct of frame parameters is a frame parameter; a lookup may be asked inside `par_each`; a part given `()` is a socket; `turn_toward`; how a test sets a `Heading` | [R1](#r1-describing-a-game) to [R6](#r6-tests-and-errors) |

**Asked for and not taken into the core**, because each would change its shape or its cost, with where it
went instead:

- **A `List` in a component** (swarm). A component stays `Copy` plain data of a fixed small size. What a
  character owns is its children, read with `under`.
- **A character's own children read by its control function and still run ahead** (exploration). It widens
  what a match runs ahead, so it is [an entry of the map](#smaller-entries), beside depth 2.
- **An event sent from inside `par_each`** (swarm), and **a lobby that takes the game's own rules and frame
  functions** (swarm). Each is [an entry of the map](#smaller-entries).
- **A second way into a phase that is entered again** (arena). Its author would keep the lifecycle as it is.
- **A scene added to the game from a test's own code** (swarm). A game with a `rules` module keeps every
  function that changes the world there, so a practice scene is part of the game, and a test that wants
  rules of its own builds a sketch.

The entries of the map that round 4 found thin are sharpened where they stand:
[3D space](#3d-space-and-a-kinematic-character) (one terrain that is drawn and stood on, what is placed and
not solid, what a contact's surface is), [gamepads](#gamepads) (a trigger, and which axis is inverted),
[audio](#audio) (its asset kinds, volume and pitch, the listener), [navigation](#navigation-and-path-queries)
(where a path's ends are put, and a path's length),
[predicted projectiles](#predicted-projectiles-and-contact) (a flight ended at a contact) and
[lobby choices for every player](#smaller-entries) (the panel's signature).

## Purpose and scope

Thrum's engine modules are built and proved, and each has its own public API: a frame host, a world lane, a
registered simulation, sprite rows, cooked rows, an interface tree, a session, a link. A game written against
them today is mostly plumbing. This module is the front door: one crate, `thrum`, that a game depends on and
nothing else. A game maker writes what the game *is* (its things, its rules, what the player sees) and the SDK
turns that into the engine's registrations, schedules, extraction and hosting. It adds no second engine
underneath: every kind of state keeps the owner it has ([owners](#state-and-owners)).

**Who it is for.** A developer who knows Rust and has never seen Thrum, and a coding agent in the same position.
Agents cannot watch a window, so testing a game without one, replaying a recorded run and getting a picture
from a command are part of the product.

**First release.** What is commissioned here:

1. The game structure: game, scenes with typed arguments, phases, entities, components, resources, prefabs,
   rules and frame functions.
2. Players, typed action maps with input contexts, bound to keys, the pointer, touch and gamepads (O8), a
   controlled character, bots, commands, offers and the standard lobby.
3. 2D space: colliders, queries on the world as it stands, a level's fixed collision loaded from content, tile
   maps, and a character move that reports its contacts.
4. 3D space (O8): the same colliders, queries, fixed collision and character move with a 3 on them, over a
   terrain and solid models, with a camera that pitches and a boom that avoids the level. No rigid bodies.
5. 2D and 3D looks under one vocabulary: sprites and clips cooked from source images, shapes and tile layers;
   cooked models with materials and textures whether placed or moving, sky, Sun, ground and lights.
6. Typed content with stable IDs, a typed asset catalog, saves into and out of a running game, and a profile
   for every player (O8).
7. Audio in its small form (O8): a sound that happens, a sound a thing makes, and music.
8. Settings on the tuning panel; the profiler; the interface.
9. Multiplayer as an opt-in that changes no rule, for 2D and 3D games (O6), with a host that needs no window
   (O8).
10. Tests with no window, recorded runs, pictures from a command, and errors with stable codes.
11. Reference documentation with an example on every public item, guides, templates and a new-game command.
12. The showcase moved onto the SDK, after the V1 exit.

**Not in the first release**, by the owner's scope (O4, O8) and the engine as built. Each has an entry in
[the extension map](#the-extension-map).

- Rigid-body physics and skeletal animation (O4, O8).
- Navigation, and a 3D ground's dressing and scatter.
- Text fields, scrolling lists and sliders (O8), which `GAME-UI` does not build.
- Predicted projectiles and contact, and hits judged as the shooter saw them, which come in the release after
  (O8); host migration.
- A visual editor (D2), levels from an editor's file (O8), a tile with a collision shape smaller than its cell,
  and hot reload of code. A changed rule rebuilds and restarts.
- Water, clouds and weather. `WATERS` and `CLOUDS` are still building.
- A crates.io release. A game reaches `thrum` by a path to an engine checkout, as
  [GAME-SPLIT](../GAME-SPLIT/DESIGN.md) settled.

### Scope questions for the owner

**All answered.** The owner took the planner's recommendation on each, on 27 November 2026 (O7, O8). A "yes"
added work packages, and for two of them an engine module, and changed no shape, which is what each entry of
the extension map shows. [The work packages](#the-work-packages) say where each is built.

1. **3D collision queries and the 3D character move in the first release?** Without them the 3D SDK serves
   showcases and not games, and a 3D match has nothing to run into. **Answered: yes, as queries and a
   kinematic move only, with no rigid bodies** ([entry](#3d-space-and-a-kinematic-character)). It needs a
   deterministic 3D index and fixed collision from meshes and a heightfield, as an engine module of its own:
   `SIM-3D`. Built in WP16 and WP17, with [camera pitch](#camera-pitch-and-rigs), which its entry gives to
   this question.
2. **Gamepads in the first release?** A platformer and a third-person game are played on one. **Answered:
   yes** ([entry](#gamepads)). It needs pad input in the shells, and focus by pad through `GAME-UI`'s
   bindings as built. Built in WP15.
3. **Audio in the first release?** Every dogfood game named it as the gap between a prototype and a game.
   **Answered: yes, the small form** ([entry](#audio)), behind an engine module: `AUDIO`. No agent can hear
   it, so how it sounds waits for the owner's ear, in WP19. Built in WP18.
4. **Predicted projectiles and predicted contact between players in the first release?** A fast
   player-against-player game feels a round trip late without them. The swarm game, a co-op crowd, needs
   neither. **Answered: no, first in the release after** ([entry](#predicted-projectiles-and-contact)):
   it is a declaration on a control line that changes no rule, and only a person playing such a game can
   judge it.
5. **Hits judged as the shooter saw them in the first release?** Only a competitive shooter needs it.
   **Answered: no, after question 4** ([entry](#hits-judged-as-the-shooter-saw-them)). Until then the
   design says plainly that a hit is decided at the shared tick.
6. **Multiplayer for 3D games in the first release? Answered: yes** (the owner, 27 November 2026, O6).
7. **A host with no window?** `SESSION` already hosts headless. **Answered: yes**, as one command in the
   online work package, WP5 ([entry](#smaller-entries)).
8. **Skeletal animation** stays out (O4). Both third-person games read as prototypes without it.
   **Answered: it stays out of the first release**, and is planned first among the engine modules that
   follow ([entry](#skeletal-animation)).
9. **A text field and scrolling lists?** A player's name comes from the system account until there is a field.
   **Answered: no for the first release** ([entry](#text-fields-lists-and-more-controls)).
10. **Levels from an editor's file (LDtk, Tiled, glTF placements)?** Hand-typed tile rows and placement tables
    do not scale past a demo, and D2 rules out an editor of Thrum's own, not reading someone else's.
    **Answered: after the first release, with data packs** ([entry](#levels-from-an-editors-file)).
11. **A profile for every player, kept on each player's own machine?**
    [The core](#10-content-assets-saves-and-settings) gives every save an owner, and a profile is the
    player's. Without it a match's saves are the host's and a guest keeps nothing. The swarm game's long game
    depends on it. **Answered: yes**, in WP13 for a game played alone and in WP5 for a guest, with
    [the entry](#saved-progress-for-every-player) that carries a guest's into a match.

### Physics, animation and audio

Decided by the owner on 25 November 2026 ("Planner's default"), and changed by the owner on 27 November 2026
(O8).

| | The first release |
|---|---|
| **Physics** | **2D and 3D queries in, rigid bodies out.** Within queries: rectangles and capsules beside the engine's circles, and in 3D spheres, capsules and cuboids; a level's fixed collision; and a character move that slides and reports its contacts. Nothing pushes anything |
| **Animation** | **Sprite animation in**: a clip a screen plays (`Shown<Sprite>`) and a clip that loops in step for everyone (`Sprite::looping`). **Skeletal animation out**, and first among the engine modules planned after the release |
| **Audio** | **In, in its small form** ([entry](#audio)): a sound that happens, a sound a thing makes, music and three volumes. No effects, no mixing graph |

## What a game maker must see

| Cue | Where it must hold | Bar |
|---|---|---|
| K1 A first game is one file and one command, and its first page ends with a character you steer | The 2D and the 3D starter, on the Mac and in Chrome | Godot's "Your first 2D game" and macroquad's first window: nothing to configure before something moves |
| K2 The code reads as the game | Every worked example and template: a scene function reads top to bottom as a description of play, and no line names a device, a frame host, a page, a permit or a schema number | Bevy's `breakout` example, with less ceremony |
| K3 One structure | 2D, 3D, alone and online: the same `Game`, scenes, rules and frame functions. Turning multiplayer on changes no rule | Unreal's gameplay framework, where one set of rules serves a listen server and a client |
| K4 Defaults look finished | A 3D scene that asks for `Sky::clear()` and nothing else gets the physical sky and Sun, shadows, exposure, tone mapping, reconstruction, EDR where the display has it and the governor. A 2D scene gets crisp sprites, drawn in order | The showcase's own look, with no setting touched |
| K5 Mistakes explain themselves | At compile time, at startup and while running, across [the listed mistakes](#12-mistakes-that-explain-themselves), each with a stable code | Rust's own compiler messages: the object, the cause, the next step |
| K6 Every public item is documented with an example that compiles | All of `thrum` | The Rust standard library's reference |
| K7 The docs are enough | A fresh agent builds a small 2D game, a small 3D game and a networked game from the guides alone, then makes a change the guides did not show | The first hour with Godot's or Bevy's getting-started guides |
| K8 The speed is kept | The reference crowd's behaviours at 10,000 bodies, written as SDK rules, beside the hand-written reference; a game links, compiles and prepares nothing it does not use | The engine's own figures, as tripwires |
| K9 A game is proved without a window | Rules stepped, input scripted, entities found and state read in an ordinary `#[test]`; a recorded run replayed in a test; one command writes a picture | An agent can prove a rule, keep a level beatable and look at a frame |
| K10 Nothing is guessed | Every public type a game touches has its methods in [the reference](#the-reference), one place, and every call a worked example makes is there | The dogfood reports' lists of guesses, each closed |
| K11 Nothing bends the shape | Every entry of [the extension map](#the-extension-map) adds names at a named extension point and changes no name or invariant of the core | A feature added a year on reads as if it had always been there |

## The game at a glance

Fifteen words carry the whole SDK. Each means the same thing in a 2D game, a 3D game, alone and online.

| Word | What it is | In code |
|---|---|---|
| **Game** | The program: its scenes and what holds for all of them | `Game::new("Rockfall").scene(play).run()` |
| **Scene** | One part of the game with its own setup, rules and look: a level, a menu, a match. One runs at a time. A scene is a type, and its fields are what it is entered with | `scenes.go_to(GameOver { dodged: 12 })` |
| **Phase** | One stage of a scene, with its own rules: a countdown, a wave, an intermission | `scene.phase(Stage::Wave).rules((..))` |
| **Entity** | One thing in the world | `entities.spawn(ROCK.at(x, 10.0))` |
| **Component** | A plain piece of data on an entity | `#[derive(Component)] struct Health(i32);` |
| **Resource** | A plain piece of data the world has one of | `#[derive(Resource)] struct Score { dodged: u32 }` |
| **Prefab** | A named, reusable thing to spawn, with its defaults, its variants and its children | `prefabs! { pub const BAT: Prefab = "bat"; }` |
| **Rule** | A function that moves the world on by one tick. Rules are the only code that changes the world | `fn fall(mut rocks: Query<(&Rock, &mut Transform2d)>, tick: Tick)` |
| **Frame function** | A function that runs once for each picture: it reads the world as shown and sets what only this screen shows | `fn status(mut ui: Ui, run: Res<Run>)` |
| **Action** | A named thing a player can do, a field of an action map, bound to keys, pointer, touch and a gamepad | `#[bind(Key::Space)] jump: Button` |
| **Command** | A message a player sends once, such as a menu choice | `outbox.send(PickArena(arena))` |
| **Player** | Who an input came from: a person at a device, or a bot. A game played alone has one | `inputs.of(player).jump.pressed` |
| **Look** | A component the engine draws: `Sprite`, `Shape2d`, `Model`, `Light` | `Sprite::new(art::BAT)` |
| **Content** | A typed record the game's rules read, loaded from a file and named by a stable ID | `waves.get(run.wave)` |
| **Setting** | A named value on the tuning panel | `#[slider(4.0..=14.0, default = 9.0)] run_speed: Num` |

Four sentences hold the model together, and the guides open with them:

1. **Rules change the world, at the tick.** The world is its entities and resources, and nothing else.
2. **Frame functions show the world, at the frame.** They cannot change it. They set what this screen shows,
   and they send input.
3. **Everything that changes the world comes in as input at a tick**: a held action, a command, a player
   joining, a pause, a save restored. That is why a game can be replayed, tested and played online without
   being rewritten.
4. **Everything happens in the order it is written.** A scene function reads top to bottom as one tick, and
   what a line did is what the next line sees.

A feature the SDK gains later arrives as more names at one of [eight extension points](#the-extension-map): a
parameter a rule may take, a parameter a frame function may take, a look, an action kind, a line of a game or
a scene, a kind of input, a kind of asset or content, a command of the tool. None of the four sentences
changes.

## The core

This is what the owner locks. Each section gives **what always holds** and points at the **names** a game
touches, which [the reference](#the-reference) lists once, with signatures. A name that is in neither the
reference nor a worked example is not in the first release.
Each invariant here is a mechanism a builder builds as written
([what a builder may change](#what-a-builder-may-change)).

### 1. A game: one crate, one package, one value

- A game is one Cargo package that depends on `thrum` and names no other engine crate.
  `use thrum::prelude::*` brings the fifteen words and the maths.
- **No type of an engine crate reaches a game through `thrum`.** Each type in a public signature of `thrum`
  is one of three kinds. It is defined in `thrum` or its derive crate, so its methods and its documentation
  are the SDK's. Or it is Rust's own, from `core`, `alloc` or `std`: an `Option`, a reference, a tuple, an
  iterator. Or it is the game's own, standing behind a generic parameter, as the `&T` that `Content<T>::get`
  returns. A type defined in any other crate, such as `wgpu`, `winit`, `thrum-core` or anything else `thrum`
  is built from, is refused wherever it stands: as a return type, an argument, a field, a bound, an
  associated type, a re-export or an alias, and as a type argument of another type. So `Option<Entity>`
  passes and an `Option` of a backend handle does not. The surface guard judges each type as it resolves,
  not as the item spells it.
- **Everything a game writes compiles with no window.** Scenes, rules, frame functions, the interface, cameras
  and looks are plain data and functions. That is `thrum` with its default features off, and what a test, a
  headless host and the rules check use. The default features add `run()`. The `multiplayer` feature, off by
  default, is the only way the network crates enter a game's closure.
- **`pub fn game() -> Game` is the one value `main` runs and a test steps.** The templates keep it in
  `src/lib.rs`, so tests under `tests/` reach it as well as tests beside the code.
- **A game's rules are its `rules` module**: `src/rules.rs`, or `src/rules/` and everything under it, or, in
  a game that is one file, a `mod rules { .. }` block in that file. It holds the game's components, scene
  types, action maps, prefab names and rules. [The check](#8-numbers) builds it on its own, a block lifted
  out of its file as it stands, beside the generated asset catalog and against `thrum` alone, and holds it
  to the portable profile. So a rule cannot call a helper elsewhere in the game.
- **Every function a game registers that runs in a tick, or whose answer enters the world, is defined in that
  module**, checked at startup from each function's path, in `test()`, `run()` and `cargo play --check`
  alike, a sketch in a test apart (below). Those are the functions registered by these routes and no
  others: a rule in `scene.rules` or a phase's `rules`; a control function; a spawn function given to
  `scene.players().spawn(..)` or to a tileset's `.spawns(..)`; an enter or exit function of a scene or a
  phase; a condition in `.when(..)`; the function a phase's `.lasts(..)` is given and the rule its
  `.then_run(..)` is given; a save's capture, its condition and its `validate`; the function an offer's `.choices_with(..)` is given;
  and the functions a lobby is given in `.starts_with(..)` and `.continues_with(..)`. A registration a
  later feature adds joins this list when it is added
  ([extension point 5](#the-extension-map)).
- **A game with a `rules` module registers each function as itself, because the check reads the function's
  own type.** A named function's type is its path, and a closure's is the path of the function it is
  written in, so each says its module. A function pointer's type says only what the function takes:
  `fn(ResMut<Score>)` is the type of a pointer to `rules::score` and of one to `screens::score` alike. So
  a pointer cannot be checked, and a game with a `rules` module refuses one at startup on every route of
  the list above, wherever the function it points to is defined, with the route, the pointer's type and
  the step: list the function by its name. A rule chosen while the game runs is each candidate listed
  under its own `when`. **A pointer is a rule in two places, which check no module:**
  [a sketch](#13-tests-without-a-window-recordings-and-pictures), which only `test()` steps, and a test's
  `game.run`, which registers nothing. In both it runs as a closure written there does, and
  [may not take `Random`](#3-rules-and-the-tick).
- **A `Save` type is defined there too, with the older types of its chain.** Every machine carries an old
  save forward when a restore lands, so the conversions are rules' code: the derive calls each one, and the
  module built on its own does not compile without them.
- A value the description gives the world directly (a resource's starting value, a prefab's definition, a
  shared setting's default) is part of
  [the rule-set identity](#11-multiplayer-an-opt-in-over-input-only-networking).
- **Every game that runs has a `rules` module, so every game played has checked rules behind it** (D1). The
  one-file starters keep theirs as a block in the one file, and the templates have `src/rules.rs` from the
  first line. So the first game a newcomer plays records, replays on another device and goes online as it
  stands, and there is no unchecked way to play.
- **`run()` plays only rules the check has passed as they were built.** The check works out **the rule
  sources' digest**: one value over every file of the `rules` module, or the block of a one-file game, and
  the profile they were held to. A pass is recorded against that digest in the game's build folder. The built
  program carries the digest of the rule sources it was compiled from, fixed when it is compiled, with no
  line of the game's code. `run()` refuses before the first tick unless a pass is recorded for that digest.
  `cargo play` checks and then builds, and builds the record into the program, which is how a browser build,
  a simulator build and a program that leaves the machine are made. A program Cargo built alone looks in the
  build folder as it starts. So `cargo run` plays the rules `cargo play --check` last passed. After any edit
  to them, a file read inside a correctly named `mod rules` included, the digest has no pass, and `run()`
  refuses and names the step: `cargo play`, or `cargo play --check`, whose refusal names the line. The same
  digest is the rules' part of [the rule-set identity](#11-multiplayer-an-opt-in-over-input-only-networking).
  `test()` asks for no record: a test is not play, and it steps a sketch too.
- **No build setting chooses a game's rules, so a pass holds for every way Cargo builds those sources.** The
  check refuses conditional compilation anywhere in the `rules` module: `#[cfg(..)]`, `#[cfg_attr(..)]` and
  `cfg!(..)`, whatever they test (a feature, a target, a profile, `test`), in a macro's body too. It refuses
  them from the sources as written, before it builds anything, so a line behind a feature that is off when
  the check runs is refused all the same. The module reaches nothing but `thrum` and Rust's own library, and
  everything it can name in `thrum` is there with `thrum`'s features off and means the same with them on.
  So the same rule sources compile to the same rules under every feature, target and profile, and the
  digest, which is over the sources, says which rules a program plays. **A pass still applies after any
  change of a build feature**, with no new check. What differs between two builds goes where a build may
  choose it: a value `game()` gives the world, such as a resource's starting value or a shared setting's
  default.
  `game()` and the scene functions are outside the module, so a feature may still choose what they
  describe. Every function they can register is in the checked module, and what they describe is in the
  rule-set identity as the program built it, so two peers built with different choices do not start a
  match. A test of the rules is written outside the module, beside `game()` or under `tests/`.
- **A game with no `rules` module is a sketch for a test.** `test()` steps it, which keeps a doc example and
  a unit test's own game a few lines long, and nothing else runs it: `run()` and every `cargo play` command
  refuse it at startup, and `Game::multiplayer` and `play_recording` refuse it in a test. Each names the
  functions that change the world and the one step: put them in `mod rules { .. }`.
- A game that outgrows one package may move the module to a package of its own, whose manifest says
  `[package.metadata.thrum] rules = true`: the same check on the same code, with the same refusals, and
  that package declares no feature.
- **The repository's guard** holds every package in a games repository, under the engine's `templates/` and
  under its `examples/` to `thrum` alone, from WP8
  ([ruling 5](../ARCHITECTURE.md#seam-rulings-for-game-sdk)), with the packages written on the engine's own
  API listed in the registry, each with its reason.

**Names:** `Game` and its lines, in [R1](#r1-describing-a-game).

### 2. The world: entities, components, resources and prefabs

- **The world is `thrum-core`'s registered simulation, and the SDK adds no storage.** The world is its
  entities and resources. Everything the SDK itself must remember about a run (the scene and phase, seats,
  open offers, the pause, the level's recipe) is a resource it registers there, so it is digested, rewound
  and sent to a late joiner with the rest.
- **A component is `Copy` plain data.** `#[derive(Component)]` writes its registration, its stable schema
  identity (from its name and fields, never from a Rust type ID, as C01 requires), its encoding and its
  finiteness check. A resource, an event, a command and a scene's fields hold the same kinds, and each
  such type is `Data` too, so a component sits inside another struct or a save with the one derive:

  | Kind | Types |
  |---|---|
  | Numbers | `bool`, the fixed-width integers, `Num`, `Vec2`, `Vec3`, `Angle`, `Rot3`, `Seconds` |
  | Time | `TickNumber`, `Timer`, `Cooldown`, `PhaseClock` |
  | Names | `Entity`, `Player`, `Id<T>`, `Prefab<N>`, `Layer`, `Mark`, `Part`, `OfferId`, `LevelPiece`, `Side` |
  | Text | `Text<N>`: up to `N` bytes of UTF-8, such as a player's name |
  | Shapes | `Option<T>`, a fixed array `[T; N]` and a tuple of these; a struct or an enum that derives `Data` |
  | In a resource only | `List<T, N>`, a list of at most `N`, which a scene's fields may hold too; `PerPlayer<T>`, one `T` for each seat |

  A `Vec`, a `String`, a reference, a raw float and `usize` are each a compile error that names what to use. A
  `List` is an ordinary value besides, so a rule may keep one as a local between two passes. Long text and
  tables belong in [content](#10-content-assets-saves-and-settings), which rules read and cannot change.
- **A resource has one of two lifetimes, said where it is registered.** `scene.resource(value)` is set to that
  value each time the scene is entered. `Game::resource(value)` keeps its value across scenes. The running
  scene's own value, such as `Level { def }`, is read as `Res<Level>` and cannot be written.
- **An entity is a handle, valid until the entity is despawned.** Entities belong to the scene they were
  spawned in and leave with it. The world holds a fixed number, `Game::max_entities(n)`, because the engine
  reserves its storage once, and a spawn past it is refused with a message.
- **`spawn` returns the entity's handle at once.** The entity itself appears when the rule finishes
  ([section 3](#3-rules-and-the-tick)), so until then the handle can be stored in a component, attached under
  a parent and given to a later spawn as its parent, and nothing else.
- **What the world holds is fixed before the first tick**, from what the game's description shows at startup:
  the signatures of every function the scenes list; every prefab's bundle and every spawn function's return
  type; the SDK's own components; and `Game::component::<T>()` for the rare one none of those shows. Spawning
  a component the list lacks is refused, naming the rule, the component and the line to add.
- **A prefab is a named thing to spawn** (D2, R3). Its name is a typed constant made in a `prefabs!` block and
  nowhere else, and the game gives each name one definition before the first tick: a bundle, or `.variant()`
  of another prefab with changes. A variant follows its base and is resolved at startup.
- **`Prefab<N>` says in its type what a spawn must bring.** `SHOT.at(..)` without its `Shot` and `Velocity` is
  a compile error, so no prefab holds a made-up owner or a zero that means "not set". `.with(c)` means one
  thing everywhere: this spawn, or this variant, has `c`. It brings what the prefab needs, replaces a
  default, or adds.
- **The world remembers which prefab an entity was spawned from**, by the prefab's stable name, and whether a
  definition made it as a child or a part. That is the SDK's own component, so it is digested and rewound
  with the rest. `entities.part` reads it, and [a save keeps entities by it](#10-content-assets-saves-and-settings).
  An entity spawned from a bare tuple has none.
- **No name an author wrote waits for its use to be checked.** Every `prefabs!` block the game's code refers
  to registers itself with the built program at link time (the technique of Rust's `inventory` crate), so the
  SDK knows every declared name before play on every target, with no line of the game listing the blocks. At
  startup each name must have a definition, and each definition's components, assets, children and parts are
  checked, with [the access check](#3-rules-and-the-tick) against every rule. A prefab a content record
  names is checked with the content.
- A bare tuple is still a bundle, and a function that returns a bundle or a spawn is still a good way to
  compute one. A prefab is for what is spawned by name, varied or checked early. A look that needs a
  transform brings one at the origin when the bundle has none.
- **Children.** A child stores a local transform, is drawn and queried at its parent's world transform, and
  is despawned with its parent. A **part** is a child with a name, declared in the prefab: a warden's eye, a
  place on a back. A part with no look is a socket. Entities with no parent pay for no hierarchy. A cycle is
  refused. A reference that is not ownership, such as a target, stays an ordinary `Entity` field.
- **Ownership is written in one line and read in one line.** A definition's children are the same for every
  spawn. A spawn adds its own with `.child(spawn)`, chosen where the spawn is made, from content or from who
  it is for: `calling.prefab.with(hero).at_point(p).child(wand.prefab.with(Weapon::new(id)))`. They land with
  the entity, under it, after the definition's own. So a spawn function returns a character whole, and no flag
  and no second rule arms it. **`query.under(parent)` reads a parent's children**: those the query matches,
  in the order attached, read and written as the query reads and writes, so "this keeper's weapons" is
  `for mut weapon in weapons.under(hero)`. It reads the children the tree holds for that parent, and
  searches nothing else. A part is a child, so `under` finds it.
- **Attaching says where the child ends up.** `keep_local(pose)` cannot fail. `keep_world()` stores the local
  transform that recomposes nearest to where the child stood and **promises a bound, not exactness**
  ([C01](../../design/CONTRACTS.md)): one part in 2^17, exact only where every number on the way composes
  exactly. It is refused, with the tick and with nothing changed, under a parent whose scale through its
  parents is zero or outside 1/10,000 to 10,000, where the result would need shear, or past the bound, which
  is tested on every call. `detach` is held to the same rule.
- **A disabled entity** keeps its components and its place and is skipped by queries, space queries and
  drawing, with its children, until it is enabled. The rule that brings it back finds it with the `Disabled`
  filter, which gives the disabled entities and no others, so a pick-up's restock `Timer` lives on the
  pick-up. `disable` and `enable` return whether this call made the change, as `despawn` does, so two heroes
  on one pick-up take it once. A spawn may say `.disabled()`, so a sleeper is made asleep. The `WithDisabled`
  filter gives the entities a query matches whether they are disabled or not, for the rule that must reach
  both, such as a card answered while its keeper is down.

**Names:** the derives, `prefabs!` and the typed constants in [R1](#r1-describing-a-game); `Prefab`, `PrefabDef`, the transforms, `List`
and `PerPlayer` in [R2](#r2-what-world-state-holds); `Query` with `under` and its filters, `Entities` and `Family` in
[R3](#r3-what-a-rule-may-take).

### 3. Rules, and the tick

- **A rule is a function, and its parameters say what it reads and writes.** A rule is a function of its
  parameters (D1): the same world, input and tick give the same result on every machine and in every replay.
  Nothing a rule can be handed leads to a clock, a file, the network, the window or one player's screen, and
  [the check](#8-numbers) refuses the ordinary ways of reaching round that.
- **Written order (R2).** A scene function is one list. Each tick the SDK walks it top to bottom and runs each
  entry once: a `scene.rules(..)` block's rules in turn, a `scene.players()..control(f)` line for each
  player's character in seat order, and a `scene.phase(p)` block's rules when `p` is the scene's phase.
  `rules` may be called any number of times and tuples nest, so no list is too long to write.
- **While a rule runs, the world stands still around it.** Its own writes to components and resources are in
  place at once. Its spawns, despawns, inserted and removed components, attachments, disabling and changes to
  the level are held until it finishes, so nothing appears or vanishes under a loop.
- **When a rule finishes, everything it did has happened.** The next rule's queries visit an entity the last
  one spawned and not one it despawned, and space queries answer from where things are now. The engine
  resolves structural edits when a tick ends today. The SDK asks core for a barrier after each registered
  system, inside the tick's one transaction
  ([ruling 2](../ARCHITECTURE.md#seam-rulings-for-game-sdk)), and WP1's first item proves it before anything
  is built on it.
- `entities.despawn(e)` returns whether this call is the one that despawns it, so two heroes on one gem count
  it once.
- An event a rule sends is read by later rules of the tick, by frame functions, and by no rule of the next
  tick.
- **The whole tick commits or rolls back as one.** A rule returns nothing or a `thrum::Result`. A returned
  error refuses the tick on every target: the world is as it was before the tick, the game stops with the
  rule's name, the tick and the error, and in a test `step()` returns it and the game can be stepped again.
  A panic is a bug in the game. On the Mac and the simulator it is caught and reported the same way. On the
  browser target, which cannot unwind, the program stops after the SDK's hook has put the rule, the tick and
  the message on the page.
- **What a rule may touch comes from its signature alone.** A signature names components in accesses: each
  `&T` and `&mut T` in a `Query` or a `Single`, and each a control function takes for its character. It names
  resources in `Res<T>` and `ResMut<T>`. **Two accesses of one function conflict when they name the same
  component, at least one writes it, and one entity is reached by both.**
  - **A resource has no entity to tell two accesses apart**, so two accesses to one resource conflict
    whenever one of them writes it. `ResMut<Ledger>` beside `Res<Ledger>`, or twice, in one function, written
    there or inside a `Params` struct it takes, is refused at startup, before any rule is handed either, with
    the function, both parameters and the resource. The SDK's other writing parameters (`Entities`, `Emit<T>`,
    `Scenes`, `PhaseMut<P>`, `PlayersMut`, `LevelMut2d`, `Offers<T>`, `BotInputs<A>`, `Saving`) are each taken
    once by a function, by the same check.
  - Inside one parameter both always reach the same entity, so `Query<(&mut Health, &Health)>` is refused at
    startup whatever entities exist. A writer reads too.
  - Between two parameters the SDK decides from **the sets of components that exist**, not from filters: every
    prefab's set at startup, and each new set when the rule that made it finishes, by a bare spawn, an
    `insert` or a `remove`. A set two parameters of some rule conflict over refuses that tick and names the
    rule, the parameters, the component and the entity. So parameters that match nothing in common need no
    `Without`, and no rule ever starts with an entity two of its parameters could both reach.
  - A control function's own accesses reach one entity, the character its player has, and its queries are
    tested against that entity: at startup, when a rule changes the character's components, and when
    `possess` gives the player another.
  - What a rule looks things up through (`Space2d`, `Level2d`, `Family`, the tree on `Entities`) reads the
    world as the rule found it, so it never reads a value the rule is writing.
- **A struct of parameters is a parameter.** `#[derive(Params)]` on a struct whose fields are rule parameters
  makes one a rule may take, with methods of the game's own, so rules that share work share one line. The
  access check and the read-only bound see through it to its fields.
- **A read-only parameter** cannot change the world: a `Query` or a `Single` with no `&mut`, `Res<T>`,
  `Family`, `Inputs<A>`, `Players`, `Events<T>`, `Tick`, `Random`, `Space2d`, `Level2d`, `Phase<P>`,
  `Content<T>`, `Restored<T>` and `Keep`. A spawn function, a condition and a capture take read-only parameters and
  nothing else, by a bound on the type: what a function may do is fixed by the type it asks for and never by
  how it binds the value.
- **Parallel work is inside a rule.** `query.par_each(..)` runs the closure over the engine's chunks. It
  writes only the entity it is given, and the result is the serial loop's by construction. What a rule looks
  things up through, such as `Space2d` and `Level2d`, may be asked inside it.
- **Cadence and conditions.** `rule.every(seconds(1))` runs a rule once each interval, first at the end of the
  first interval. A rule written in a phase's block counts its interval on
  [that phase's clock](#4-scenes-phases-time-and-pause): from when the phase was entered, and in a resumed
  phase from where the clock had got to. Every other rule, one under `during` included, counts from when
  the scene was entered. `rules.when(condition)` runs a rule or a
  group only in ticks where a read-only function returns true. `rules.during(phase)`, with one phase or a
  tuple of them, runs a rule or a group only while the scene is in one of those phases, where it is written:
  it is for rules that two phases share. A phase block is still where a phase's enter and exit functions and
  its length are said. A cadence that belongs to one thing is a `Cooldown` on that thing.
- **A rule's identity is its function's name**: the path Rust gives a named function, as
  `swarm::rules::spawn_enemies`, which is the path [section 1's check](#1-a-game-one-crate-one-package-one-value)
  reads and the name an error gives the rule. It belongs to the function, not to where or how the function
  is listed. So it is the same in a scene's `rules`, alone or inside a tuple, in a phase's block, under
  `every`, `when` and `during`, as a condition, as an enter or an exit function, as the function a `lasts` is
  given, as a `then_run` rule, and in a test's `game.run`. **None of it is state.** No part of a draw's key
  comes from a table that fills as functions are registered or called, an address, a Rust type ID, the
  order of registration or a hasher of Rust's library, because none of those is the same in every build.
  A rewind, a replay and a game built again have nothing of it to restore.
- **The name, and so every draw, is the same in every build of the same sources by one version of the
  compiler**: a debug build and a release build, every feature set, every target. That is the whole
  promise. Another version of the compiler may spell a path differently, so its program is another build
  ([risk 20](#risks)). Two games made in one program cannot show this, because one program has one build's
  answers. WP1 shows it with draws held as literal numbers in a test that passes in a debug build and in a
  release build, and WP2 with its digests on the three targets.
- **A draw from `Random` is keyed by the game's seed, the game's tick, the name of the function that asks
  and how many times that function has already run inside a tick at that game's tick**, and by nothing
  else. So a function draws the same wherever it is listed, two functions draw apart, a function listed
  twice in a scene draws apart in its two runs of a tick, and a rule added to a scene changes no other
  rule's draws. Across ticks the count is more than nothing only for
  [a rule that runs through a pause](#4-scenes-phases-time-and-pause). So two hands dealt in two paused ticks
  are two hands, and no draw depends on how many empty paused ticks a driver committed. A control function,
  which runs once for each player, is keyed by its player in place of the count, so a match runs it ahead
  for one player alone.
- **Only a run inside a tick is counted.** The count is world state, and only a tick writes it, with the
  entry of a scene that starts a game or that a test's `go_to` makes, which counts as a tick's landing
  does. A function is also run outside any tick, on the world as last committed, in three ways: a save's
  capture and its condition, when a frame function asks through `saves.save` or `saves.can_save`; the
  same capture after the tick of a rule that asked with `Saving`; and what a test runs with `game.run` or
  `TestGame::save`.
  Each such run draws as that function's first run at that world's game's tick, and counts nothing. So
  every read of one committed world draws the same and answers the same, on whichever machine and however
  often. A pause menu that asks `can_save` twice in one picture changes no digest and no later draw, and
  the host whose menu asked stays equal to a peer whose menu never opened. A test's calls, rewound or
  not, leave every later draw as it was.
- **Two functions that take `Random` never share a name.** Rust gives one path to two functions declared
  under one name in two blocks of one function's body, so the key above could not tell their draws apart.
  One program can tell them apart: two registrations are one function exactly when they have one Rust
  type. The SDK asks that question, same or different, which every build answers alike, and nothing of
  the type enters a key. At startup, in `test()`, `run()` and the check alike, it sets beside each other
  all the functions the game's description registers that take `Random`, by whichever route and in
  whichever scene. Two of one name that are not one function refuse the start, before either runs, with
  the path, where each is listed and the step: give one of them another name. A test's `game.run` holds
  the function it is given to the same test before running it, against the game's functions and against
  those that test game has already run: the second of a pair is refused as the error `game.run` returns,
  and the world is unchanged. What a test game has run is the harness's own note, in no digest, and no
  draw reads it. So in one game no two functions ever draw under one name. Two functions of one name
  where either takes no `Random` are left alone, because only one of them draws under it.
- **Only a named function may take `Random`.** A closure has the name of the function it is written in,
  which every closure written there shares, and a function pointer has no name: its type says only what
  it takes, and two functions with one body are one address in an optimised build. Either is a rule like
  any other, [where it is a rule at all](#1-a-game-one-crate-one-package-one-value), until it asks for
  `Random`. Then it is refused, at startup where a scene lists it and as the error `game.run` returns,
  with the callable and the step: write a named function.
- **The tick** is 60 Hz unless `Game::tick_rate` says otherwise. Speeds are written per second and multiplied
  by `tick.dt()`.

**Names:** every parameter a rule may take, with its methods, in [R3](#r3-what-a-rule-may-take); `every`, `when` and `Params` in [R1](#r1-describing-a-game).

### 4. Scenes, phases, time and pause

**Scenes.**

- **A scene is a type and a function.** The type derives `Scene`, and its fields are what the scene is entered
  with. The function describes it: `fn level(scene: &mut Scene<Level>)`. `Scene` alone is `Scene<()>`, for a
  game with one scene. `Game::scene` takes one function or a tuple of functions that describe the same scene,
  which is how a game keeps a scene's rules in `rules.rs` and its screen in `screens.rs`.
- The game starts in the first scene listed, entered with its `Default` value. A first scene with no default
  is a compile error. `game.test_in(scene)` and `cargo play --scene <name>` start elsewhere.
- **Which scene is running, and its value, are world state.** A match's players are always in the same scene,
  and a late joiner is sent it.
- **A tick makes one transition at most**: a change of scene or a change of phase, never both. The tick's
  requests are taken in this order, and the first stands: one an enter or exit function made while the last
  tick's transition was landing; a `request.scene` admitted at this tick; the rules' own, in written order,
  `scenes.go_to`, `scenes.restart` and `PhaseMut::go_to` alike; a phase's `lasts`, which counts as asked after
  every rule. A later request returns `false` and changes nothing, and its caller may ask again in a later
  tick. The rest of the tick's rules still run, in the scene and phase the tick began in, and the transition
  lands when the tick ends. So the rule written first decides, whichever kinds the two requests are.
- **`go_to` returns whether its request stands**, so a rule does once what must happen with the change:
  `if scenes.go_to(Cleared { .. }) { records.keep(&run) }`. `go_to` with the running scene's type and a new
  value leaves the scene and enters it again, which is how one round follows another.
- **A request made while a transition is landing waits a tick.** A `go_to` made by an enter or exit function
  is held, and is the next tick's first request. A held request for a phase lives no longer than its scene:
  asked by an exit function of a scene that is being left, it is refused and nothing is held.
- **Leaving:** the phase's exit functions, then the scene's, then every entity of the scene is despawned.
  **Entering:** scene resources are set, then the scene's `on_enter` functions and its `players()` lines run
  in the order written, then the first phase's clock is set and its enter functions run. All of it happens
  in the tick that asked, so
  the next tick's first rule finds the scene whole.
- A scene's kind (2D or 3D) follows from its camera. A scene needs no camera: without one it shows its
  background and its interface, which is what a menu and a result page are.
- **A frame function asks for a scene with `request.scene(..)`**, honoured only where the running scene says
  `scene.accepts::<Target>()`. In a match the host's request is the one admitted, unless the scene says
  `.accepts_from_anyone::<Target>()`. Anything more particular is a command and a rule.

**Phases.**

- A phase is a value of an enum that derives `Phase`. `scene.phases(first)` gives the scene phases and names
  the one it starts in. A `scene.phase(p)` block is a place in the scene's written order.
- A change of phase is one of the tick's requests and lands when the tick ends: the old phase's exit
  functions, then the new one's enter functions. Going to the phase the scene is in leaves and enters it.
- Rules outside any phase block run in every phase. A frame function and a control function read `Phase<P>`.
  Only a rule takes `PhaseMut<P>`.
- `lasts` takes a time, or a read-only function that returns one, asked once, when the phase is entered, so
  a night's length can come from its content file or the host's choice. Its answer is world state, so the
  function is one of [the checked routes](#1-a-game-one-crate-one-package-one-value). A `lasts` is never
  less than one tick.
- **A phase's clock is world state: two counts of whole ticks.** The first is how many ticks the phase has
  run. The second, kept while the phase has a length still to act on, is how many ticks are left until its
  time is up. Everything that counts from the phase's entry reads the first: `phase.time()`,
  `Tick::in_phase()`, and the interval of each `every` rule in the phase's block. `left()` reads the second.
- **The clock is set as the phase is entered, before the phase's first enter function runs**: no ticks run,
  and the whole length left. A function given to `lasts` is asked at that point. It runs after the exit
  functions of what is being left and, where a scene is being entered, after the scene's own enter
  functions, so it reads what they wrote. It runs before the phase's enter functions, so nothing they
  write changes the length. So an enter function that reads `phase.clock()` gets `time()` of nothing and
  `left()` of the whole length, which is the clock a test reads before the next tick, whether the length
  was written as a time or came from a function. Nothing asks for the length again, so a setting that
  changes while a phase runs changes the next phase entered and not this one.
- **Each tick the phase runs moves the clock once, before the tick's first rule**: one more tick run and,
  where any are left, one fewer left. **The time is up in the first tick whose clock reads none left.** In
  a phase entered with a length of `n` ticks that is the `n`-th tick after the entry, and its rules read
  `left()` as zero.
- **When the time is up, what follows the length happens once, after every other rule of that tick**: the
  phase goes on to `.then(phase)`, or runs `.then_run(rule)`, an ordinary rule which may ask for a scene
  made from the run, such as the next round or a result page. Its request is the `lasts` request, last in
  the tick's order. **From then the clock has no length**: where the rule asks for nothing the phase goes
  on, `left()` is `None`, and the rule does not run again. So `left()` says which of three things is true.
  A time above zero is a length still running. Zero is a time that is up with what follows still to
  come, which is read only inside that one tick. `None` is no length, or one already acted on.
- **A scene has one phase machine**, and a second `scene.phases(..)` is refused at startup. So the tick's one
  transition and `Tick::in_phase()` each mean one thing. A mode that runs beside the phase, such as walking
  and fallen, is a resource, or a component where each character has its own, with its own `Timer` and rules
  under `.when(..)`.
- **A phase is the scene's, so it is for what happens to everyone**: a countdown, a wave, an intermission.
  What happens to one character, such as dying, is that character's own, as
  [example D](#d-a-precision-platformer) keeps it. So a game written for one player gains a second with no
  rule changed (K3).
- **A scene entered from a save resumes its phase with the clock it was saved with, and enters none.**
  `Phase::clock()` gives the phase's clock as one value, a `PhaseClock`, which holds the clock's counts of
  ticks and which a save keeps beside the phase.
  An `on_enter` function of the scene may call `PhaseMut::resume(phase, clock)`: the scene then starts in
  that phase with the clock's two counts in place of a clock set fresh, and the clock moves from there as
  any clock does. So **the phase carries on tick for tick as the phase the clock was taken from would
  have**. The first tick after the scene is entered is the tick that would have come next where the clock
  was taken: the phase's first tick where one of its enter functions took it, the tick after the one a
  rule took it in, or the tick after the last committed where a test or a save's capture took it between
  ticks. In that tick and in each one after, `phase.time()`, `Tick::in_phase()` and the time left read
  what they would have read, an `every` rule of the phase's block runs if it would have run, and the
  phase's time is up if it would have been. So a rule that runs every five seconds, in a phase saved four
  seconds in, next runs one second after the restore, and no reading is a tick late, because none is the
  sum of two rounded times.
  **What follows the length is neither lost nor repeated, and the clock's time left says which.** A clock
  taken with time left, by an enter function included, has it happen in the tick it would have. A clock
  taken by a rule in the tick the time was up holds a time left of zero, because what follows had not yet
  happened when the rule read it. Resumed, its first tick after the entry is a tick whose time is up:
  that tick's rules read `left()` as zero again, and after them what follows happens, once. A clock taken
  after that, between ticks or in any later tick, has no time left, and nothing follows a second time.
  A save's capture reads the world as committed, so the clock it keeps is never the middle one.
  No function is asked again: no phase's enter function, because nothing was entered, and not
  the function `lasts` was given, because the length was fixed on entry. So a phase entered with ten
  seconds and saved four seconds in ends six seconds after the restore, though the setting its length
  came from says twenty by then.
  **`resume` restores the phase's clock and nothing else.** What counts from the scene's entry, which is
  `Tick::in_scene()` and the interval of an `every` rule outside the phase's block, counts from this
  entry, and a `Timer` and a `Cooldown` are the game's own values, which its save brings back.
  Called anywhere else `resume` returns `false` and changes nothing.

**Time belongs to the thing being timed** (R5). All of it is counted in ticks and is world state, so it
rewinds, replays and saves. Coyote time, a jump buffer and a dash are `Timer` windows. A weapon's rate is a
`Cooldown` on the hero that holds it. A phase with a length uses `lasts`. Each says how long it has left and
how far along it is, to a rule and to [a frame function](#9-frame-functions-looks-and-the-interface) alike.
**A time is a whole number of ticks wherever the SDK counts one, and there is one rule each way.** A count
of ticks reads as the nearest time a `Seconds` holds. A length of time `d` (a `lasts`, an `every`, a
`Timer`'s or a `Cooldown`'s wait, `tick.after(d)`, a time a `PhaseClock` is made from by hand) is the
fewest ticks whose time reads at least `d`, and an `every` and a `lasts` are never less than one tick. So
a wait never ends early, a time read from a count gives that count back, and `phase.time() >= d` first
holds in the tick a `lasts(d)` runs out, at 60 Hz as at any rate, whether the phase ran straight through
or was resumed on the way.
**A clock made by hand has no tick rate until a game takes it.** `PhaseClock::at(time).ending_in(left)`
can be written before any game exists. It holds the two times it was given, and `time()` and `left()`
give them back as given. `resume` turns each into ticks at its game's rate, by the rule for a length:
a tenth of a second is one tick at 5 Hz and six at 60 Hz. A clock `phase.clock()` gave holds its game's
counts, so the times it reads give those counts back. A clock made with a time left of zero is one whose
time is up with what follows still to come.
**A save keeps both.** A `Timer` or a `Cooldown` in a save is written as its whole wait and the time it has
left, and comes back with both, so `left` and `fraction` read after a restore what they read at the capture.
A wait of four seconds saved two seconds in comes back half done, and one of eight saved six seconds in
comes back three quarters done, each with two seconds left. One that is idle, ready or finished comes back
so.

**Pause.** A pause is one thing, alone and in a match.

- **Whether the game is paused, and by whom, is world state**, and asking for a pause and ending one are input
  admitted at a tick. Alone the one player may always pause. In a match it is the game's choice:
  `Multiplayer::players(..).pause(Pausing::Nobody)`, the default, `Host` or `Anyone`.
- **While paused, ticks still commit and the game's clock stands.** Two counts are kept apart. **The
  committed tick** is the engine's: it counts every tick the world commits, paused or not, a match is
  numbered by it, and the world's digest covers it. **The game's tick** is what `Tick` gives a rule: world
  state that moves on by one in each tick that is not paused. So through a pause `Tick` gives the same
  number, and every timer, cadence, phase clock and tick-timed animation waits. No held action is read and
  no control function runs. There is nothing to catch up afterwards.
- **A paused tick changes the world only by the input it admits**: a command, an answer, a request, a change
  to the roster, a shared setting's edit, a piece of a save, or the pause's end. One that admits none of them
  runs no rule, marked or not, and leaves every entity, component and resource bit for bit as it was. Only
  the committed tick moves on. So nothing a rule or a frame function reads can tell how many such ticks
  were committed, and the number is the driver's: a match's host commits one each tick, because the match's
  clock is the committed tick, and a game alone commits none and steps a paused tick when input arrives.
- **Two runs are compared at the same committed tick.** The digest covers the committed tick, so it differs
  between a run that committed ten empty paused ticks and one that committed none, while every value in the
  two worlds is equal. [A recording](#13-tests-without-a-window-recordings-and-pictures) holds each input at
  the committed tick that admitted it, which keeps both how many paused ticks its run committed and which
  inputs were admitted together. A replay commits the same ticks, whichever driver made the recording and
  whichever replays it, and its digest equals the run's at every committed tick.
- **What a paused tick does with what it admits:**
  - The roster changes at once. A join's event and character come at the first tick after the pause.
  - **Commands, answers, scene requests and a shared setting's edits are admitted.** A rule marked
    `.even_when_paused()` runs once, in its written place, in each paused tick that admits input, and in no
    other. It reads the commands and answers its signature names. So which inputs share a tick shows: two
    commands admitted in one tick are read by one run, and two admitted in successive ticks by two runs, at
    the same game's tick. A command or an answer no such rule takes waits in the world for the pause's end
    and is read once then.
  - A scene request lands, and ends the pause.
- **A save is no input, so a pause never stands in its way.**
  [Its capture](#10-content-assets-saves-and-settings) reads the world as last committed, and no tick is
  stepped for it.
- Frame functions run in every picture, so a menu works, and anything that should move while the world waits
  is theirs.
- So what a menu does under its own pause works: quit to the title, save here, restart the level, or end the
  talk whose box holds the pause.
- **A press made while the presser's own screen shows the pause is never read as play**, and one made in play
  is read once ([section 5](#5-players-actions-and-control)).
- `ui.modal(..).pausing()` asks for the pause while the modal is open, where this player may pause, and is an
  ordinary modal where they may not. `scene.unpausable()` is for a scene where a pause makes no sense.

**Names:** a scene's and a phase's lines in [R1](#r1-describing-a-game); `Timer`, `Cooldown` and `PhaseClock` in [R2](#r2-what-world-state-holds); `Scenes`, `Phase` and `PhaseMut`
in [R3](#r3-what-a-rule-may-take); `Pause` and `Request` in [R4](#r4-what-a-frame-function-may-take).

### 5. Players, actions and control

**Actions.**

- **An action map is a struct whose fields are the things a player can do** (R7), and a rule reads it as plain
  fields. `#[derive(Actions)]` also writes a name for each field, `Controls::JUMP`, for the places that name
  an action and do not read it: a test, a key prompt, a camera rig, a remap screen.
- **The bindings on the map are the defaults.** Each player can change theirs, and a player's bindings and
  local settings are kept on their own machine. A field with no `#[bind]` and no `#[sent]` is a compile error.
- **The action maps a game's rules read are its action set**, known from their signatures. It is the same on
  every peer, fixes the layout of the input frame, and is part of the rule-set identity. It must fit
  `SESSION`'s frame: three 16-bit axes, into which the SDK packs every button, `Axis` and `Stick`, and up to
  eight actions of 256 bytes. Every `Heading` and `#[sent]` value of the set travel together, in one action
  that carries the latest of each, which a late or repeated frame leaves standing. So a sampled frame fills
  one of its eight actions at most, and the host, which admits a frame's own actions first, always has
  seven left for that player's commands. Startup refuses a set that does not fit, with the counts and the
  limit.
- **A press is read once: never twice, and never lost while no more presses fall between two admitted frames
  than the count holds**, which is at least three and which the reference states. A button travels as whether
  it is held and a small running count of presses, beside a bit that says the frame was sampled, and the
  world keeps the last count it read. So a frame the session repeats presses nothing; the neutral frame for
  an absent player releases everything and keeps the count; a press on a late frame is read from the next
  that arrives; and a joiner starts its counts from the world's.
- **A pause and play are kept apart where the press is made.** While a player's screen shows the pause, their
  machine counts no press, and a key that goes down then is ignored until it is let go. A key held since
  before stays held. A press made while their screen still showed play is read once, in the first tick after.
- **A `Heading` belongs to the player's machine.** Bound to the mouse, it turns at the frame rate with the
  pointer captured, so a camera that reads it has no delay, and its latest value travels with each tick's
  input, so "forward" in a rule is the way the player was looking. `#[sent]` is for a value a frame function
  works out, such as the direction from a craft to the pointer.
- **A rule may turn a player to face a way, and their machine follows.** `players.face(p, Walk::LOOK, angle)`
  is how a respawn faces the arena. The world keeps a turn for each player's `Heading`, added to what their
  machine sends, and `face` sets it so that the heading reads as `angle` from that rule on. Their own screen
  adds the same turn from the picture that shows that tick, as a cut. So the camera and the rules agree, and
  a frame sampled before the turn moves the heading only by what the mouse did since.
- **The pointer follows the input context.** It is captured while this screen's context has a `Heading`
  bound to the mouse, and free otherwise. A modal turns the context off, so it frees the pointer for its
  buttons and gives it back when it closes, in a match where the world keeps running too. Cards outside a
  modal are worked by local actions while the pointer stays captured. Where a browser asks for a gesture,
  the click that captures is not read as play.
- **Local actions never leave the machine.** `#[derive(LocalActions)]` is for keys only the screen cares
  about: pause, a scoreboard, a card's hotkey. A frame function reads them with `MyInput<Menu>`. A rule cannot
  take them, they are in no input frame, digest or replay, and they are still read while a modal is open.
- **An input context** says which action maps this screen's keys, pointer and touches drive now, so one key
  can jump in play and buy in a shop while the world keeps ticking. A screen has one at a time,
  `Screen<InputContext>`. A shared map with no `#[context(NAME)]` is in `InputContext::PLAY`, and a local map
  with none is always on. A map whose context is off is sampled as idle, and a shared map stays in every input
  frame with the same layout, so a context never changes what a match agrees on. A key that went down while a
  context was off is ignored by it until the key is let go.

**Players.**

- **A player is who an input came from, and who is in the game is world state.** `Players` only looks, and is
  what a frame function, a spawn function and a condition are given. `PlayersMut` changes the roster and is a
  rule's alone.
- **A `Player` names one stay in a seat.** It is a seat and a count of the joins that seat has seen, as an
  `Entity` is a slot and a generation. A value kept after its player left, such as a shot's `pilot`, never
  names whoever sits there next: `players.status(p)` is `Gone`, `PerPlayer::get(p)` is `None` and
  `Query::of_player(p)` finds nothing. `player.index()` is the seat, for a place in a line or a colour.
- `PlayerJoined` is sent once for a player, at the tick they enter the game, to whichever scene is running. A
  later scene finds who is here with `Players`. `PlayerLeft` carries the player and their name, so a rule can
  keep what should outlive them. `PlayerAway` and `PlayerBack` come at the tick of each.
- **`CharacterSpawned { player, character }` is the moment a character appears.** It is sent each time a
  scene's spawn makes a character: for every player here as the scene is entered, for a player who arrives
  later, and for a `respawn`, a bot's as a person's. So what happens once for each character that appears,
  such as opening its line on a scoreboard or granting the levels a late joiner missed, hangs on one event in
  whichever scene the character appears, where `PlayerJoined` comes once for a player. It is read from when
  the character lands to the end of that tick, and one sent as a scene is entered is read through that
  scene's first tick. `possess` sends none: it makes no character.
- `PerPlayer<T>` keeps one value for each seat, such as a score that outlives a character. It returns to its
  default when the seat's player leaves.
- **A rule that reads `PlayerLeft` still finds what the leaver had.** `PerPlayer::departed(player)` gives, for
  the rest of the tick a player leaves in, the value their seat held as they left, read-only. `get(player)` is
  `None` from the moment they go and the seat is at its default for whoever sits there next, so the two never
  mix: a bot's score is filed in the very tick a person takes its seat. From the next tick `departed` gives
  `None`. The world holds the value for that one tick, so every machine and a replay read the same.
- **Seats.** A world has four seats, `SESSION`'s four slots, and `Multiplayer::players` says how many a match
  uses. Which are taken, by a person or a bot, is world state, and `SESSION`'s slot numbers never reach a
  game. A person takes the lowest free seat and a bot the highest. **A person is never turned away for a
  bot**: where bots hold every free seat the person takes the lowest a bot holds, and in that tick the bot
  leaves as any player does. When people hold every seat a newcomer is refused as full, by the SDK: it hands
  `SESSION` no link while no seat is free or away, and gives no seat to a slot the world has no room for. An
  away player's seat is theirs for the grace.

**A controlled character** (D3, R1). One line of a scene gives every player a character and says how it moves.

- **A player has one character at a time, and which entity it is, is world state**: the one the scene's
  `spawn` made for them, until `possess` gives them another. `Players::character(p)`, `Me::character()`,
  `Query::of_player(p)`, a camera `following_my_character`, `respawn(p)` and what a match runs ahead all mean
  that one entity. Anything else a player owns, such as a drone, a turret or a shot, is an ordinary entity
  that holds their `Player` in a component of the game's own and is moved by a rule that reads
  `inputs.of(player)`.
- **An entity is one player's character at most, and the `Player` component says whose.** The SDK alone
  gives and takes that component: a character holds its player's for as long as it is their character, and
  a game reads it and cannot spawn, insert, change or remove it. So the component on an entity and the
  answer of `character(p)` never disagree.
- **`possess(p, entity)` moves both or neither**, and returns whether this call made the entity `p`'s
  character, as `despawn` does. Where the entity is alive and is nobody's character, it is `p`'s from the
  next rule on: the SDK takes the `Player` component off the one they had, which stays in the world as an
  ordinary entity, and puts it on this one. Where the entity is another player's character, is `p`'s
  already or is gone, it returns `false` and nothing changes. So when two players reach for one turret in
  a tick the first asked has it, and when one reaches for the other's craft both keep the character, the
  component, the camera and the control they had. An entity that lacks a component the control function
  takes is a mistake in the game, and refuses the tick.
- **`spawn`** takes a prefab or a function. The function may take `Joined` (who it is for) and any read-only
  parameter, so it can place the character at a level's start or in a line. It runs for each player already
  here when the scene is entered, in seat order, each finding the characters spawned before it, and for each
  who arrives later. The character is given that `Player` as a
  component. It makes world state, so it is defined in the `rules` module. A scene has one.
- **`control`** takes a function whose `&T` and `&mut T` parameters are components of that one character, with
  `&A` for that player's action map. It is the character's whole movement: it writes the pose itself. It runs
  where its line is written, once for each player's character.
- A character is despawned when its player leaves. While a player is away their character stays under idle
  input, unless `when_away` says otherwise. `players.respawn(player)` runs the scene's spawn for them again:
  it brings back a character a rule despawned, and takes the place of one that still lives.
- **A scene may have further `players().control(..)` lines, each `.during` another phase**, for a character
  that moves one way on foot and another at the helm. One control function moves a character in any phase:
  two lines that could run in the same phase are refused at startup.
- **A control function may take any parameter a rule may, and a table says which of them a match runs
  ahead.** In a match the local player's own character answers at once when everything its function reads is
  in the first three rows:

  | Parameter | In a match, for the local player's character |
  |---|---|
  | Its own components, `&A`, `Tick`, `Entity`, `Random`, `Content<T>` | Run ahead exactly. Content and the seed never change, and everything `Tick` gives counts from entries the step's state carries |
  | `Res<T>`, `Phase<P>`, `Level2d`, `Level3d` | Run ahead with the value at the last confirmed tick. A change arrives a moment later, as a correction. Free for settings, the phase, the level and anything that changes rarely |
  | `Emit<E>` | The event reaches this screen's frame functions when it is predicted, once, so a jump's dust is not late. Rules read it at the confirmed tick |
  | `Query`, `Single`, `Space2d`, `Space3d`, `Inputs`, `Players`, `Family`, `Events<E>`, `Commands<T>`, `Restored<T>`, `Keep` | **Not run ahead.** They read what other machines move or send |
  | `ResMut`, `Entities`, `PhaseMut`, `Scenes`, `Offers`, `LevelMut2d`, `LevelMut3d`, `PlayersMut`, `BotInputs`, `Saving` | **Not run ahead.** They change more than the character |

  A function that takes anything from the last two rows still works everywhere, with its character shown as
  the host confirms it, a round trip late. So that nobody pays that by accident, a game with multiplayer on
  refuses to start until that line says `.unpredicted()`, and the message names the parameter that costs
  prediction. Alone, nothing is asked. A new rule parameter joins one row of this table when it is added, and
  [predicting more than the character](#predicted-projectiles-and-contact) moves named things up a row.
- **A bot is a player whose input a rule writes.** `PlayersMut::add_bot` takes a seat and is announced as any
  player is. A rule written before the control line sets the bot's map with `BotInputs<A>`: a button is set
  as held or not, and the world derives `pressed` and `released` from one tick to the next as it does for a
  person. From there nothing tells a bot from a person: the same spawn, control function, score table and
  offers, which a rule answers for it.

**Names:** an action map and the players' line in [R1](#r1-describing-a-game); `Button`, `Axis`, `Stick`, `Heading`, `Player` and
`PerPlayer` in [R2](#r2-what-world-state-holds); `Inputs`, `BotInputs`, `Players` and `PlayersMut` in [R3](#r3-what-a-rule-may-take); `MyInput`, `Pointer`, `Bindings`
and `Me` in [R4](#r4-what-a-frame-function-may-take).

### 6. Commands, offers and requests

- **A command is a message a player sends once**, a value of a type that derives `Command`, sent from a frame
  function with `Outbox`. The SDK keeps commands that cannot go yet and sends them in order, so a menu never
  handles a full queue.
- **An admitted command is read once**, by `Commands<T>`, with who sent it, after the same player's earlier
  commands, on every machine: in the tick it is admitted, or, where a pause holds that tick and no rule that
  runs through the pause takes it, in the first tick after the pause.
- If the link drops while a command is on its way, the game cannot know whether it was admitted, and the
  player comes back to a fresh copy of the world. So an interface is drawn from what the world says, and a
  repeated command is made harmless. An offer does both for a choice.
- A rule that refuses a command says so with an event, which the sender's frame functions read. Startup
  checks that each command type's encoding fits one action of an input frame.
- **An offer is a question the rules put to players, answered once** (R8). Open offers are world state: a late
  joiner and a reconnecting player are shown theirs, and the cards stay until the answer has landed.
- `accept` gives the answers admitted this tick: one for each player and offer, from a player it was put to,
  for a choice it held, while it was open. Anything else is dropped.
- **An offer stays open until it is answered**, unless it says `.expires(..)` or `.until_phase_ends()`. It
  ends with its scene, and a player's part in it when they leave. So a level-up earned in one wave is still
  there in the next, and an answer in flight across a change of phase lands.
- **A player's open offers of one type queue.** Their screen is shown the oldest, and the next when its
  answer has landed, so three level-ups in a second are three questions in turn with nothing to keep by hand.
- **An offer's choices may be drawn when it is shown.** `.choices(..)` fixes them when the offer is opened.
  `.choices_with(f)` takes a read-only function that is asked once, in the tick the offer becomes its
  player's oldest open one, and what it returns is the offer's choices from then, as world state. So each of
  three hands is dealt against the build the last answer left: a weapon at its top level is not offered
  again. The function is given `Asked`, which says for whom and for which offer, and it makes world state,
  so it is one of [the checked routes](#1-a-game-one-crate-one-package-one-value). It is for an offer put to
  a player or to each player. A vote's choices are the same for everyone and are given when it opens.
- A player who joins while `to_each_player` or `to_all` is open is asked. A player's part in an offer ends
  when they leave. A person answers from their screen and a bot through a rule, both are taken by `accept` in
  seat order, and nothing downstream can tell them apart. An `OfferId` holds the scene's entry and a count,
  so a second run cannot answer the first run's offer.

**Names:** `Commands`, `Offers`, `Offer`, `Pending` and `Tally` in [R3](#r3-what-a-rule-may-take); `Outbox`, `Request` and `MyOffer` in [R4](#r4-what-a-frame-function-may-take).

### 7. Space and the level

- **Colliders detect and never push.** There are no rigid bodies (O4). A collider is on one layer. Its sizes
  are full widths and heights, a capsule's length includes its caps, and it is centred on its entity's
  position unless `.offset(..)` says otherwise. In a 3D scene a `Collider2d` lies on the ground plane, x and
  z, so triggers, pick-ups and catch radii work in a 3D game with the first release's queries.
- **`Space2d` answers from where things are now** (R2): after every rule before this one, the last rule's
  spawns and despawns included. `at_tick_start()` is for a rule that wants the older view on purpose.
- **One index for each layer.** `.on(LAYER)`, with one layer or a tuple, searches those layers and nothing
  else, so an enemy that asks for the nearest of four heroes looks at four colliders with 10,000 enemies
  around it. A layer's index is rebuilt only when a rule has changed that layer since it was last built.
- **Two orders, by what was asked.** An overlap (`touching`, `within`, `in_rect`, `nearest`) answers nearest
  first. A ray or a sweep answers earliest contact first, by `fraction` along the path. Ties go to the
  earlier spawned.
- **`motion_of(entity).sweep()`** sweeps the entity's own collider along the path it took this tick, from
  where it stood at the tick's start or was spawned, against the paths the others took. It is how a fast shot
  hits and how a stomp knows it came from above. **A `teleport` starts a new path**: an entity teleported
  this tick is swept from where the teleport put it, so a respawn across a patrol touches nothing between
  the two places.
- **A written position is a straight path, and a move that bent or stopped short says so.** Where a function
  writes a position, the entity's path this tick is the straight line from where its path began to where it
  now stands, over the whole tick. That is right for a shot and for a walker that met nothing. It is wrong
  for a body that slid: a courier whose step of (3, 3) from (0, 0) met a wall at (1, 1) and slid up it to
  (1, 3) passed through a watch at (1, 2) that the line between its two ends misses by 0.316. It is wrong
  too for a body that was stopped: a hero whose step of (4, 0) from (0, 0) is stopped by a wall when its
  centre reaches (1, 0) was at (0.5, 0) one eighth of the way through the tick and at rest from one quarter
  of it, where the line between its two ends has it at (0.5, 0) half-way through.
  So **a transform follows a move**. `at.follow(&moved)` puts the entity where the move ended and makes the
  move's own path the entity's path for this tick: each leg in order, from where the move began, with the
  share of the tick the move spent on it.
  - **A leg takes the share of the tick that it took of the step.** The move spends its step as it goes: a
    leg that ends at a contact met a fraction of the way through the step that was left takes that fraction
    of the tick that was left, and a shape stopped before its step was spent rests where it stopped for the
    rest of the tick. So the courier's first leg took a third of the tick and its second two thirds, and the
    stopped hero's one leg took a quarter of the tick and it rested for three quarters. A path keeps when
    each leg began and ended, a rest included.
  - **`motion_of(entity)` is then those legs, by time.** Its sweep meets the courier's watch on the second
    leg, against where the watch itself stood at that moment of the tick. `fraction` is the time along the
    whole tick, `mover_position` is on the leg the contact was found on, or where the body rests, and
    `remaining_motion` is from there to where the path ended, which for a body at rest is nothing. A small
    thing that crosses the stopped hero's line at (0.5, 0) half-way through the tick is not met, and one that
    comes against the hero at (1, 0) in the tick's last quarter is.
  - **A followed path lasts while the entity stands where the move ended.** A position written after it
    makes the path a straight line again, from the same beginning to the new place.
  - **A move followed from where the entity stands adds to the path the entity already has this tick, and
    the two run together.** Both took the same tick, so at each moment the entity is where the earlier path
    had it, moved on by as far as the new move had gone by that moment. The earlier path is whatever the
    entity has: a followed move's legs, the line a written position gave, or standing still, for which the
    sum is the new move's own legs. So a rider whose ferry carried it by (2, 0) and whose own move then took
    it up by (0, 1) crossed the straight line from where it began to (2, 1), and was at (1, 0.5) half-way
    through the tick. A rider carried through a coin, who then stands still, meets the coin at the moment
    the ferry brought it there: its own move, of nothing, adds nothing, and takes nothing away.
    `fraction` is the time along the tick, as on any path. This sum is what the entity passed, for a
    pick-up, a trigger and a stomp. What stopped it is in each move's own `contacts()`.

    ```rust
    let carried = tiles.move_shape(at.position, body, space.motion_of(ferry).delta()).on(TERRAIN);
    at.follow(&carried);    // the ferry's step this tick, stopped by the level if a ceiling is in the way
    let moved = space.move_shape(at.position, body, hero.velocity * tick.dt()).except(me).on((TERRAIN, PLATFORMS));
    at.follow(&moved);      // from where the hero now stands: added to the carry, both over the one tick
    ```
  - **A move followed from any other place begins a new path where that move began**, as a `teleport`'s
    begins where it put the entity. What moved the entity earlier in the tick is then not swept, so a jump
    between two places that should touch nothing between them is a `teleport` or a move begun at the new
    place.
  - **It is the tick's working memory**, as the place each entity stood at the tick's start is. It is made
    again when a tick is stepped again, and is gone when the tick ends, so it is in no digest. **A path that
    is one straight leg taking the whole tick, from where the entity stood as the tick began, is what a
    written position gives, so an entity whose followed moves come to that keeps nothing more**: a move that
    met nothing, a walker's step along the floor it stands on, a rider's carry and its own step across open
    ground. A path that bent, that came to rest before the tick was over or that began somewhere else keeps
    its legs and their times.
  - A control function and a rule follow a move the same way, and `Level2d::move_shape` and
    `Space2d::move_shape` both give one. Every guide and example moves a body with `follow`.
- **The level is a scene's fixed collision** (R7). It does not move, so it is not made of entities. What the
  world holds is **the level's recipe**: each tile map loaded, by that map's identity in the game's content,
  its origin and the tile sheet it is drawn with where its load named one, and each fixed shape a rule has
  added since, in order. The recipe is digested, rewound and sent in a baseline, and the collision and the
  tile layer are built from it, the same on every machine. The SDK keeps the built collision of every recipe
  still inside the rewind and prediction span, by the identity of what is solid in it, so two recipes that
  differ only in a sheet share one, and a joiner, a rewind and a step run ahead each get the level of their
  own tick.
- **A change to the level is a rule's act.** It lands when the rule finishes and is undone with a refused
  tick. The recipe holds a fixed number of added maps and shapes, 256 unless `scene.level_shapes(n)` says otherwise.
- **What a rule adds, it can take away.** `add_rect`, `add_segment` and `add_tiles` each return a
  `LevelPiece`: a name for that piece of the recipe, plain data a resource or a component keeps.
  `remove(piece)` takes the piece out and returns whether this call did, as `despawn` does. The recipe is the
  pieces that stand, in the order they were added, and the number it holds is of those. Everything built from
  the recipe follows, after a rewind and for a late joiner too: the collision, the tile layer, and paths
  where [navigation](#navigation-and-path-queries) is built. So a gate that drops and rises is one piece
  added and removed, and nothing is cleared and built again.
- **A `Tilemap` is content**: rows of glyphs in a record's file. Each has an identity of its own, its record
  and the path of fields to it, so a record may hold several. A tile is one unit square unless
  `Tileset::tile_size` says otherwise, and a glyph's spawn and its mark stand at the centre of its cell. The
  tiles are drawn as one tile layer, not an entity each. A validator and a test read a map's cells, and a
  test makes one from rows. A glyph the tileset does not name is refused when the content is checked, a
  space apart, which is nothing.
- **What a level places is spawned by a spawn function, as a player's character is.** `.spawns(glyph, ..)`
  takes a prefab, or a read-only function that returns what to spawn. The function is given `Placed`, which
  says where, in which cell and by which glyph, and any read-only parameter. So a moth keeps the tile it
  sways about, a spitter times its first shot from where it stands, a `Prefab<N>` is given its `N`, and no
  prefab holds a zero for a second function to fill in. It runs once for each such cell when the load
  lands, in row order. It makes world state, so it is defined in the `rules` module. An object in
  [an editor's file](#levels-from-an-editors-file) brings its fields through the same function. A load
  places everything its map names every time, in a scene entered from a save too: there
  [`entities.respawn(save)`](#10-content-assets-saves-and-settings), called after the load, puts what the
  save kept in place of what the load placed from the same prefabs.
- **A level may be several maps.** `load` replaces the level, and `add_tiles(&map, origin)` adds a map
  beside what is loaded, so a level of three screens is three small maps in a record that lists them.
  Either may say `.drawn_with(sheet)`, for a record that names its own tile sheet: what a glyph means is the
  scene's, and how it is drawn may be the record's. **The sheet a load named stays in the recipe with its
  map**, by the asset's stable name. So a rule that chose between two sheets and kept no note of its choice
  is drawn the same after a rewind and on the screen of a player who joined later, who never saw the load.
- **A scene's description is fixed at startup and cannot see the scene's value.** What depends on the value
  or on content is done by an enter function, which reads `Res<Level>` and `Content<T>` and builds the
  level, and by `scene.on_show(f)` for what this screen shows, such as a backdrop.
- **Rays, sweeps and moves asked of a space meet everything on the layers asked: entities' colliders and the
  level's fixed collision.** One `Hit` says which: its `entity` is `None` for the level. So one ray tells
  whether a guard sees past a hill, and one sweep whether a bolt reached the pier or the hull first. Overlaps
  find entities. **`Level2d` asks the fixed collision alone, and that is the part a match can run ahead.**
- **A hit's normal points out of what was struck, toward the mover**, and is never zero.
- **A hit says where the mover was, and what motion it had left.** `mover_position` is where the moving shape
  itself stood at first contact: its own position, and not the point touched. `remaining_motion` is the way
  from there to where its path ended, so the two together end where the path ended, always. For
  `motion_of(entity).sweep()` `mover_position` lies on the path the entity took this tick. A stomp sets the
  hero at the contact and spends the rest of the tick going up, and a bolt stops with its centre clear of
  the pier, each with no second query. For a ray the mover is the ray's own point.
- **Touching is said at the profile's precision: within the contact tolerance.** The profile's numbers
  cannot hold most places of exact contact. Two circles of radius 1, one at (0, 0) and one on the diagonal,
  touch with the second at (√2, √2), and the two numbers nearest √2 put the centres 3.99999976 and
  4.00000048 apart when squared, either side of the 4 that contact asks for. So wherever this design says a
  mover's shape *touches* a thing, or *touches it and does not overlap it*, it means this: **the gap between
  the two shapes, or the depth of one inside the other, is no more than the contact tolerance.**
  - **The tolerance is one stated value of the rule set**, the same on every machine and in the reference,
    and small beside anything a game measures. In 2D it is a fixed fraction, a power of two, of the largest
    coordinate or size of the two shapes, and of one unit where those are smaller, so it grows with the
    numbers as their rounding does. WP2's builder measures what its proof's cases leave and sets the
    fraction no smaller than that and no larger than 2⁻¹⁶. The sweep and move tests this module adds to
    `thrum-sim2d` hold it as that crate's public constant, so the SDK and the crate under it mean one
    thing by a contact. In 3D it is
    [`SIM-3D`](../SIM-3D/DESIGN.md#2-shapes-and-the-pair-tests)'s contact tolerance, a public constant, which
    its own sweeps already keep: the place one reports is clear of what was struck and within the tolerance
    of it.
  - **Every answer that speaks of contact uses it the same way**: the `mover_position` of a sweep's and of a
    move's hit; `depth`, which is exactly zero for a contact within the tolerance and more than the
    tolerance otherwise; which of the three ways below a path meets a thing; and a move that begins touching
    a wall. A path that begins within the tolerance of a thing begins touching
    it. One that only grazes a thing, passing within the tolerance of it and no nearer, may be reported or
    not, the same on every machine.
  - **The overlaps keep their exact test.** `touching`, `within`, `in_rect` and `Level2d::touching` answer
    by the profile's own arithmetic, so a pair that stands within the tolerance may be found or not, the
    same on every machine. A rule that needs a shape clear of what it struck moves it on, as a stomp does,
    or sets it out by more than the tolerance.
- **A hit says how deep, so a contact that was there from the start means one thing.** `depth` is how far the
  mover's shape stood inside what it struck at first contact, measured along the normal, which is then the
  shallowest way out. A path meets a thing in one of three ways:

  | The path | `fraction` | `mover_position` | `depth` | `remaining_motion` |
  |---|---|---|---|---|
  | Meets it on the way | Where the two first touch | Touching it | Zero | The rest of the path |
  | Begins touching it | Zero | Where the path began, which touches it | Zero | The whole path |
  | Begins inside it, deeper than the tolerance | Zero | Where the path began, which overlaps it: where the mover in truth stood | More than the tolerance | The whole path |

  In all three a shape set at `mover_position + normal * depth`, worked out in a rule's own numbers as
  written, touches what it struck: it stands within the contact tolerance of it. Where the numbers can hold
  the place, it is exact: two circles of radius 1 whose centres begin one apart along an axis report
  fraction zero, the place the mover began and a depth of exactly 1 along the line between the centres, and
  the mover set one further out stands exactly two from the other. Where they cannot, it is within the
  tolerance: with the struck circle at (0, 0) and the mover beginning at (1, 1), the depth is 2 − √2 as
  nearly as the numbers hold it, the mover set out by it stands within the tolerance of (√2, √2), and a
  sweep that begins there reports the same circle at fraction zero with a depth of zero, as a path that
  begins touching. A contact there from the start is reported whichever way the path leads, in, along or
  away. Where no way out is the shallowest, as for a shape met with itself, the normal is a fixed direction
  the reference states.
- **A hit says how fast the two were closing.** `approach_speed` is the speed, in units a second, at which
  the mover and what it struck were coming together along the normal at first contact: what the struck
  thing's motion gave along the normal, less what the mover's gave. Each motion is taken from the leg of its
  own path that the thing was on at that moment, and is nothing for the level, for a thing that did not
  move and for a body at rest after a move that stopped. It is above zero where the two were closing, zero
  where neither gained on the other, and below zero where they were parting. A contact met on the way was
  closing. For a contact there from the start it is what tells a second stomp from the tick after the
  first: a hero that begins a tick touching the top of a watch, and rises one unit in it while the watch
  rises two, has an `approach_speed` above zero, because the watch is gaining on it. With the watch still
  it is below zero. `remaining_motion.dot(hit.normal)` is above zero in both, and says only which way the
  mover itself went, which answers the question where what was struck stood still. For a `ray` and a
  `sweep_circle`, which are asked of things where they stand, the asked path is taken as crossed in one
  tick.
- **Moving a character is one call that slides along what it meets and reports it**: both axes in one step,
  so no game moves an axis at a time or compares positions to learn what happened. It is given the
  character's own collider, which a control function takes as a component, so a shape is written once.
  `Level2d::move_shape` meets the level and can be run ahead. `Space2d::move_shape` also meets the colliders
  on the layers it is given, such as a moving platform, reports the entity it stands on, and is
  `.unpredicted()` in a match. One-way tiles stop a shape that comes from above.
- **A move is told who is moving.** A shape does not say whose it is, so a move asked on its mover's own
  layer says `.except(me)`, with the `Entity` a control function may take, as an overlap and a sweep do.
  Without it the first thing such a move meets is its own collider, at fraction zero and at its whole depth.
- **A move that begins inside something is not pushed out of it and goes no deeper.** Colliders never push,
  and a move does not either. The contact is the first of `contacts()`, at fraction zero with its `depth`.
  The part of the step that leads further in is taken out, as a wall takes it, and the rest is travelled,
  so a shape leaves what it began inside by its own step or by the game's line,
  `at.position += hit.normal * hit.depth`. A shape that begins touching a wall slides along it as any does.
- **A move reports what the shape rests against when it ends, moved into or not**, within a skin the
  reference states. A hero falling beside a wall with the stick at rest still has `wall()`, so a wall kick
  needs no second probe.

**Names:** `Tileset` in [R1](#r1-describing-a-game); `Collider2d` and `Layers` in [R2](#r2-what-world-state-holds); `Space2d`, `Level2d`, `LevelMut2d`, `Tilemap`, a `Hit`
and a move in [R3](#r3-what-a-rule-may-take).

In 3D the same names carry a 3, and the first release builds them (O8):
[the entry](#3d-space-and-a-kinematic-character) says what differs.

### 8. Numbers

Two kinds of number, and one question tells them apart: **can this value affect play?** (D1, R4)

- If it can, it is world state. Rules compute it, with `Num`, `Vec2`, `Vec3`, `Angle`, `Rot3` and `Seconds`:
  [the deterministic rules profile](../SIM-2D/DESIGN.md#mechanisms)'s numbers, where each operation is rounded
  once, so one input trace gives one result on the Mac, in Chrome and on the iPhone simulator.
- If it cannot, it is presentation. Frame functions compute it, with `f32`, `Vec2f`, `Vec3f` and `Rot3f`, and
  any maths library.

**In a rule.**

| You write | Is it allowed | Why |
|---|---|---|
| `speed * 0.9`, `x + 1.5`, `x / 2.0` | Yes | A float literal beside a `Num`, a vector or an `Angle` is a `Num` |
| `x > 0.0`, `x == 0.0`, `x.max(0.0)`, `x.clamp(-12.0, 12.0)` | Yes | The same, for comparisons and methods |
| `x * 2`, `count as i32 * 3`, `Num::from(count)` | Yes | An integer of size up to 16,777,216 converts exactly. A larger one is rounded once, as the profile rounds every operation: 16,777,217 becomes 16,777,216 |
| `vec2(1.5, 0.0)`, `vec2(x, 0.0)`, `vec3(wish.x, fall, wish.z)` | Yes | Each argument is a `Num`, a literal or an integer |
| `seconds(0.4)`, `Angle::degrees(35.0)`, `Angle::turns(0.1)` | Yes | Constructors take a literal |
| `Rock { fall_speed: 6.5 }`, `const GRAVITY: Num = -30.0;` | **No** | A literal alone, where a `Num` is expected, is an `f32` to Rust. Write `num(6.5)` and `num(-30.0)` |
| `const GRAVITY: Num = num(-30.0);`, `Vec2::lit(0.8, 1.4)`, `Seconds::lit(0.4)` | Yes | `num` and the `lit` constructors are `const`, for constants and tables |
| `let k = 0.5_f32.sqrt();`, `a_float * another_float`, `x.to_f32().sin()` | **No**, refused by the check | A float computed by the platform can differ between devices |
| `use std::collections::HashMap`, a file, a clock, a thread, an outside crate | **No**, refused by the check | Each reaches outside the tick |

**Names:** [R2](#r2-what-world-state-holds). `Angle`'s functions are built from exactly rounded operations with no platform maths
library, with conformance vectors on the three targets
([ruling 1](../ARCHITECTURE.md#seam-rulings-for-game-sdk)). `normalized_or_zero` of a zero vector is zero, so a
bat on top of its target does not make a tick fail. An `Axis` is used as a `Num` and a `Stick` as a `Vec2`.
`tick.dt()` is a `Num`, a tick's length in seconds, and a `Seconds` becomes one with `.num()`. An integer indexes
a table with `as usize`.

**In a frame function.**

- A pose is read as `Pose2d` or `Pose3d`: the transform as this picture shows it, between ticks and predicted,
  in floats. `Query<&Pose2d>` is the screen's way to ask where something is. A frame function that asks for
  `&Transform2d` is told to ask for `&Pose2d`.
- Every other component is read as it is stored. A `Num` in one becomes a float with `.f32()`, a vector with
  `.f32()` and an `Angle` with `.radians()`. Anything that takes a place to draw at accepts either kind, so
  `draw.burst(art::POP, death.at)` needs no conversion.
- Floats go back into the world only as input: an `Axis`, a `Stick` or a `Heading` a frame function sets is
  put on the wire's grid, and a command's fields are plain data.

**Directions.** In 2D, x is right, y is up, and an angle of zero points along x and grows anticlockwise. In
3D, x is right, y is up and −z is forward. `direction_xz()` of a zero heading is forward and it grows to the
left. A `Stick` pushed up is forward: `stick.y` is along the heading and `stick.x` is to its right.

**The check.** `cargo play` runs it before it builds, and `cargo play --check` runs it alone. It builds the
game's [`rules` module](#1-a-game-one-crate-one-package-one-value) on its own and holds it to the profile, as the profile holds
a rule package today: no raw-float arithmetic or methods, no unordered containers, clocks, threads,
environment, files, sockets, processes or standard input, and no outside crate. Before it builds, it reads
the module's sources and refuses [conditional compilation](#1-a-game-one-crate-one-package-one-value) in
them, which is the game check's own refusal: the profile's lints and the check of the engine's registered
rule packages are as they were. Each refusal names the line,
the reason and what to write. A pass is recorded against the digest of the sources it built, and
[`run()` asks for that record](#1-a-game-one-crate-one-package-one-value), so a game built by Cargo alone
is held to the check too, with whatever features Cargo was given. It is a guard against mistakes, not a sandbox: `unsafe` code can still reach out,
and what gets past is caught, where it shows, by the digests a match compares.

### 9. Frame functions, looks and the interface

**The frame.**

- **A frame function runs once for each picture the engine encodes**, on the pair of adjacent published ticks
  the engine chose for that picture and the fraction between them
  ([C02](../../design/CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world)), and the draw that
  follows uses the same. That is **the world as shown**: a pose is where the picture has it, between the two
  ticks; everything else is as of the newer tick; and in a match the local player's own character is at its
  predicted pose, in a query and in the draw alike.
- **A frame function writes only what belongs to this screen**, and reaches the world only by sending input.
  A struct of frame parameters is a frame parameter, by the derive that makes
  [a struct of rule parameters](#3-rules-and-the-tick) one.
- **A frame function reads the tick's clock, two ways, and never needs it copied.** `Tick` is the newer
  tick of the pair, exactly as a rule read it, and every component is read as of that tick. So
  `boost.ready(tick)`, `shield.left(tick)`, `tick.in_scene()` and `phase.time()` say on a status display what
  they said in the rule. `Frame` gives the same clock at the picture's own moment between the two ticks, in
  floats, for what must move smoothly: `frame.fraction(&boost)` fills a ring, and `frame.in_scene()` times a
  ghost beside a hero drawn between ticks. Both stand through a pause.
- **`Screen<T>` is one value for this screen, whichever frame function asks.** The SDK's own (`Camera2d`,
  `Camera3d`, `Sky`) are set from the scene's description each time the scene is entered. A local setting is
  the panel's value. A game's own type starts as its `Default` and lasts for the whole game, unless a scene
  says `scene.screen(value)`. `scene.on_show(f)` runs a frame function once, when this screen first shows the
  scene.
- **`Shown<L>` is how one screen dresses a look** (R6). The look on the entity is world state: which sprite or
  model it is. Which clip plays, a flash, a decorative turn and hiding it on this screen are not.
- **What a frame function sets or draws lasts for that picture, unless the call names a clip or a time.** A
  tint, an offset, a drawn line and the interface are described again by the next run. A clip asked for with
  `play` goes on until another is asked for, and asking again for the one playing does not restart it. A
  `flash(0.3)`, a `burst` and anything drawn `.lasting(seconds)` are kept by the SDK as this screen's state and
  played out, so one started in a picture that is never shown appears in the next.
- A clip chosen with `Shown` is timed by the frame. A look every player must see at the same moment, such as
  a coin that spins in step, is `Sprite::new(..).looping(clip)` on the entity, timed from the tick. A rule
  that depends on a moment of an animation owns that moment as a `Timer`.
- **A cut.** Every pose is drawn between its last two ticks, so a move that is not a movement must say so.
  `at.teleport(point)` sets the position and marks the tick: the entity is drawn at the new place at once, a
  camera following it cuts with it, and a predicted character is not eased toward it. `pose.cut` says so to
  a camera of the game's own.
- **Cameras.** The scene describes its camera, and a rig keeps it there with no frame function. The rig is
  this screen's state: a frame function changes its numbers through `Screen<Camera2d>` or `Screen<Camera3d>`
  (a boom's length, who it follows, the field of view) and the rig goes on working. The camera, sky and
  background belong to the screen, so two players of one match can look at different places. A rule cannot
  read a camera. What rules may assume of a view is a constant the game gives both: `Camera2d::exactly(w, h)`
  shows that rectangle and no more on every window. A rig that follows a character holds its last view
  while that player has none, and goes to the next one at a cut. A frame function points it elsewhere
  meanwhile with `camera.rig().follow(entity)`. A shake, and anything else a frame function adds, plays out
  on top of the rig and does not replace it.
- **Events reach the frame through a bounded cursor.** Each `Events<T>` of a frame function is given the
  events of every tick since it last ran, each once, in tick order, from a ring of at least a second of
  ticks. `missed()` counts any a hitch dropped: events are for show, and nothing that must happen hangs on
  one. An event the local player's own control function sends while it is run ahead is shown then, and once:
  an emission is named by the player whose character sent it, the tick, the type and its ordinal in that
  player's step. So when two players jump in one tick, this screen shows its own player's dust when it is
  predicted and the other's when it is confirmed, each once and each at its own place. A confirmed tick that
  sends one more for this player than prediction did shows only that one, and one that sends fewer takes
  nothing back.
- **Everything a frame function does happens once, in the run where it does it**, whether or not the display
  takes that picture: an event is delivered to one run, a click is handed to one run, and a command sent
  takes effect when it is sent.
- Interpolation between ticks is the engine's, on the GPU, for every look. A game writes none. A component is
  published to frame functions only if one names it.

**Looks.**

- **A look is a component, and an entity with a look and a transform is drawn. A thing has the same look
  whether it is placed or moves** (D2, R6). `Model::new(mesh).material(..)` is one vocabulary on an entity
  and on a placement.
- `scene.place(model)` stands a model in the scene with no entity, for what has no gameplay identity. It is
  the cheaper path, the engine's cooked placement. Giving the same model to an entity is how it comes to
  move.
- An entity's `Model` is drawn as an entity row of the 3D scene, with its placement at two ticks, culled on
  the GPU and interpolated: position and scale linearly, the turn by the shorter way, its lit faces and its
  shadow together ([ruling 8](../ARCHITECTURE.md#seam-rulings-for-game-sdk)).
- Draw order in 2D is layer, then lower on the screen in front, then spawn order.
- Extraction is the SDK's, written once, over the rows that changed. The engine chooses the recipe for the
  device, as [the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) says, and a game
  never names one.

**The interface.**

- **A frame function describes the interface each frame with `Ui`, and the SDK keeps
  [GAME-UI](../GAME-UI/DESIGN.md)'s tree**, changing only what differs, so an interface that did not change
  costs one draw. A control is the same control from one frame to the next by its place in the description,
  and controls made in a loop take a `.key(..)`.
- **A modal is an exclusive scope.** While it is open this screen's
  [input context](#5-players-actions-and-control) is off, by the rule for any change of context, and
  direction keys move its focus. Local actions with no context are still read, so the key that opened a pause
  menu closes it. `.pausing()` asks for [the pause](#4-scenes-phases-time-and-pause).
- **Outside a modal the interface takes no direction input.** A status display, a row of cards dealt
  mid-fight and a prompt are worked by the pointer, by touch, and by the game's own keys:
  `card.pressed_by(Cards::FIRST)` presses it with a local action. So the stick that walks a hero is never the
  stick that picks a card.
- Focus, touch sizes, safe areas and density are `GAME-UI`'s. The SDK's default font and sizes ship with it,
  so a first game needs no font.

**Names:** every parameter a frame function may take in [R4](#r4-what-a-frame-function-may-take); a screen's lines in [R1](#r1-describing-a-game); looks, cameras, `Ui`
and the asset catalog in [R5](#r5-looks-cameras-the-interface-and-assets).

### 10. Content, assets, saves and settings

Three kinds of data, each with a stable identity (D4, R10), and the tuning panel.

**Assets are what a screen shows and plays**: what is drawn, and [what is heard](#audio).

- A game keeps them under `assets/`, and `cargo play` cooks them, watches them and writes **the asset
  catalog**, `src/art.rs`: one typed constant for each asset, in a module for each folder. The kind is in the
  type, so `Sprite::new(art::meshes::ROCK)` does not compile, and an editor completes `art::` to what exists.
  The catalog is committed with the game, and `cargo play --check` fails when it is stale.
- A saved asset is redrawn in the running game without a stall, as [CONTENT-LOOP](../CONTENT-LOOP/DESIGN.md)
  built.
- **A shape the rules stand on is fixed for a run.** In a game with
  [3D space](#3d-space-and-a-kinematic-character) a mesh and a terrain are each cooked into a pair, what is
  drawn and what is stood on, and the collision half is in the rule-set identity. A running game keeps the
  pair it started with, both halves, so the surface drawn is always the surface stood on.
  - **An edit that changes the collision waits for the next start.** `cargo play` cooks the new pair, names
    the asset that is waiting on its console and on the panel, and gives the running game neither half.
    The game takes the new pair when it is next started. So a rock made taller under a character is not
    redrawn over the old surface, and no machine of a match changes what a move meets at a tick of its own.
  - **An edit that leaves the collision as it was is redrawn at once**: a material, a texture, a normal. The
    cooked collision is compared, so a mesh saved again with its triangles where they were waits for
    nothing.
  - **Every run uses the pair it started with.** A test and a replay take the pair cooked when they start.
    Every machine of a match started with the pair its rule-set identity names, and a machine whose own
    differs does not join, as with any other difference of content. A late joiner steps the same pair as
    the host from its first tick.
  - A game with no 3D space cooks no collision from a mesh, so every edit to one is redrawn at once. In 2D
    what is solid is the scene's `Tileset` and a content record's map, and a sheet only draws.
- `Sprite::named("enemies/bat")` and its like take a name found at run time. A name that matches nothing draws
  a visible placeholder and is reported once, with the nearest names that exist.
- How a sheet, a clip and a tile get from a source image into the catalog is
  [an entry of the extension map](#sprites-clips-and-tiles-from-source-images) that the first release builds.

**Content is what the rules read**: waves, levels, enemies' numbers.

- A record's **ID is its file's name**. `Id<T>` is `Copy` plain data: it goes in a component, a resource, a
  scene's fields, a command and a save, and it means the same record in a later build.
- **An ID written in Rust is declared in an `ids!` block**, which registers itself with the built program as
  [a `prefabs!` block does](#2-the-world-entities-components-resources-and-prefabs). So startup knows every
  one, wherever a rule uses it, and refuses one that names no file, with the nearest name that exists. There
  is no other way to write one: a name found at run time goes through `Content::find`, which returns an
  `Option`.
- Files are RON. A decimal in a file becomes a `Num` when the content is cooked, rounded once, the same on
  every host.
- **A record holds what world state may hold**, except `Entity` and `Player`; and, because rules cannot change
  it, `String`, `Vec<T>`, a `Tilemap`, a typed asset name, a `Prefab<N>` checked against its declaration, and
  another struct or enum that derives `Content`. **A record holds no raw float**, whoever reads it.
- **Everything is checked before play**, in `test()`, `run()` and `cargo play --check`, in this order: each
  file against its type; each `Id` and `Prefab` it names, and each `Id` an `ids!` block declares; each
  asset's kind; then the game's own validators, the function each `#[content(validate = ..)]` names and
  nothing else. A validator is given its record, and any `Content<U>` it asks for after it, so a level's
  file is checked against the enemies it names and a broken level is found at startup and not by a test.
  Every file has passed its type and its names before any validator runs. A mistake names the file, the
  field and the nearest name that exists.
- The game's content, with its prefabs, is part of the rule-set identity, so a match agrees on it before it
  starts. A test reads the game's real content, and `with_content(name, value)` adds a record with no file.
- **Data packs** are the year-one commitment (D4), and [an entry of the map](#smaller-entries). The IDs, the
  checks and the identity above are what a pack needs, and they are in the first release.

**Saves are what a player keeps.** A save is a type the game owns, never an image of the world, so it can
outlive the build that wrote it.

- **A saved resource** is the short way: `Game::saved::<Records>(slot)` makes `Records` a game resource that
  is read before the first tick and written whenever a tick commits a change to it. Rules never touch a file.
  **The value it starts with is part of how a run starts**, with the seed and the first scene: a recording
  holds it, and a replay starts from it on any device and writes no slot.
- **A run saved and resumed** is the long way. `Game::save::<T>(capture)` registers the one capture for `T`,
  a read-only function that returns the save. Every request names its type, and a slot's file records the
  type and version it holds.
- **Out: a capture needs no tick.** It is read-only. Asked for by a frame function, it reads the world as
  last committed, at once: no tick is stepped, nothing is admitted, and the digest and both tick counts
  stand. So a pause menu saves a jump in mid-air, stays open and shows `saves.status(slot)` go from `Writing`
  to `Written`, alone and in a match alike. Asked for by a rule, with `Saving`, it reads the tick that rule
  ran in, once it commits. The file is written away from the tick. `save_if(capture, condition)` gives the
  one place that says when a save is allowed, and both routes and `saves.can_save::<T>()` follow it.
  **A capture and its condition are runs outside a tick, so they count nothing.** Either may take `Random`,
  and [draws as its function's first run](#3-rules-and-the-tick) at the game's tick of the world it reads.
  So `can_save` asked twice of one committed world answers the same twice, and a host whose menu asks it
  draws afterwards what a peer whose menu never opened draws.
- **In: a save enters the world as admitted input, in pieces.** `request.scene(..).restoring::<T>(slot)` reads
  the file on the machine that asks and sends its bytes ahead of the request, each piece one action of an
  input frame, budgeted after that tick's commands against the frame `SESSION` encodes whole (about 850
  bytes a tick). The world gathers them as world state, under a limit of 64 KiB unless `#[save(max_kib = n)]`
  says otherwise, and at the tick the last is admitted every machine checks it, decodes it, carries an older
  version forward through its chain and enters the scene with it. The scene's enter functions take `Restored<T>`, and
  [resume the phase](#4-scenes-phases-time-and-pause) the save names. A recording holds the pieces.
- **A restore lands whole or not at all, and its failure never leaves the SDK's step.** When the gathered save
  fails its check, is of a newer version, or an enter function that takes it returns an error, the world is
  left as it was and the tick is stepped again without the restore, inside the one world step the driver
  asked for. A match is not ended by a save that will not load.
- **A save keeps entities for the game, so it is not a second copy of the world.** A field of type
  `Kept<N>` holds the prefab names it was filled from, and entities: for each, the stable name of the prefab
  it was spawned from, whether it is disabled, and the components `N` lists. The capture fills it with
  `Keep`, a read-only parameter: `keep.all(WARDEN)`, or `keep.all((BAT, ELITE_BAT))`, takes every entity a
  rule or a spawn function spawned from those names, the disabled ones too, in the order they were spawned.
  One call brings a save's kept entities back, `entities.respawn(save)`: each is spawned from its prefab,
  with the kept components in place of the prefab's defaults, disabled where it was disabled, in the order
  kept, field after field. So no component is written again as a field of the save and again as the lines
  that respawn it, and a capture cannot quietly leave out what a query skips.
  - **What a spawn must bring, a save must keep.** A [`Prefab<N>`](#2-the-world-entities-components-resources-and-prefabs)
    says in its type what every spawn of it brings, and the definition has no value for those. So
    `keep.all` of a prefab compiles only into a `Kept` that lists all of them, for each prefab named:
    `keep.all(ROCK)` of `ROCK: Prefab<Rock>` into a `Kept<Transform2d>` is a compile error that names
    `Rock`, as a spawn without it is. Into a `Kept<(Rock, Transform2d)>` two rocks spawned with different
    fall speeds come back each with its own. What a file holds of an entity is always enough to spawn it.
  - **After `respawn`, the things of the prefabs a save names are the save's and no others.** `respawn`
    first takes out every entity that `keep.all` of each field's names would take now: those in the world,
    and those the function that calls it has itself spawned or loaded before the call. Then it spawns the
    kept ones. So a scene entered from a save builds its level as it always does and calls `respawn`
    after: the floor, the marks and the things no field names are the load's, and what a field names is the
    save's. Of two coins a map places, with one collected before the save, the restored level has the map's
    floor and exactly the surviving coin. A field filled from a name and holding none of it, because every
    coin was collected, leaves no coin. What is spawned from those prefabs after the call, by a rule or by
    a later map's load, is an ordinary spawn. A game mirrors no list of what it placed.
  - **The tree comes back with them.** A kept entity that hung under another entity the same save keeps
    comes back under it, at its local place, whichever field each is in: a parent is spawned before what
    hangs under it. One whose parent the save does not
    keep, such as a relic on the back of a player's character, comes back with no parent, where it stood in
    the world, and hanging it again is the game's line. What a definition made as a child or a part comes
    back with its parent's prefab and is not kept twice.
  - **A player's character is the scene's to spawn**, by its players' line, and never a save's: what a save
    keeps of it is the game's own fields.
  - **A mistake is found at the capture or at the load, never as a thing missing later.** An entity of a
    named prefab that lacks a listed component fails the capture, with the prefab, the entity and the
    component, and nothing is written. A kept name the game no longer declares makes the file
    `Loaded::Failed`, with the field and the name, unless the save's chain carries it forward: a conversion
    renames a prefab with `kept.renamed("husk", HUSK)` and changes what is kept with `kept.map(..)`. A
    rename is held to what the new prefab's spawn must bring, as `keep.all` is. A file that names a prefab
    whose spawn must bring a component its field does not hold is `Loaded::Failed` too, with the field, the
    name and the component, so no entity is ever spawned short. A conversion from a version that had no
    such field starts it as `Kept::none()`, which names no prefab and so takes nothing out.
  - A `Kept` is in a save and nowhere else. It is not world state, so the save's size limit is its only
    bound.
- **What a save may hold** is what a resource may hold, except `Entity` and `Player`, which name nothing in
  another run, and beside that a `Kept<N>`. A `Timer` and a `Cooldown` are written as the whole wait and the time left, and a phase's
  clock is kept as a `PhaseClock`, so [each comes back as it stood](#4-scenes-phases-time-and-pause). An
  `Id` whose record a later build removed makes the file `Loaded::Failed`, with the field named.
- Storage is each target's own: a folder on the Mac, the browser's storage in Chrome, the app's folder on the
  simulator. Chrome's is asynchronous, which is why reading is `Loaded` and not a `Result`.
- **Every save has an owner, said where it is registered**, as a resource's lifetime is (R5).
  - **The match's**: `Game::saved::<T>(slot)` and `Game::save::<T>(capture)`. One value for the game, kept on
    the machine that runs it. In a match that is the host's: only the host's machine reads and writes a
    slot, and only the host's own screen may ask for a save or a restore. A guest's `saves.can_save` is
    false. [A host with no window](#11-multiplayer-an-opt-in-over-input-only-networking) has no screen:
    its process reads the slot its lobby continues from before the first tick, and a rule's `Saving` still
    writes there.
  - **A player's: a profile.** `Game::profile::<T>(slot)` gives every player a `T` of their own, kept on that
    player's own machine beside whatever the match saves: a best haul, an unlock, a logbook. In the world it
    is one game resource, `PerPlayer<T>`. A rule writes `books[player].best = haul` in the tick that decides
    the result, for the player it names, who is never whoever sits in that seat next. Any rule may read it,
    and a frame function reads its own player's with `Me`.
- **A profile comes in before its player.** A player's value is in the world before their `PlayerJoined`:
  alone it is read before the first tick, as a saved resource is, and a guest's is carried in from the
  guest's machine by [the entry that builds it](#saved-progress-for-every-player). It is a game resource, so
  it stands across scenes and a rematch.
- **Each machine keeps its own slot equal to what the world confirms for its own player.** Whenever its
  confirmed world changes, a machine compares its own player's value there with what its slot last held, and
  writes where they differ, and no other seat's. The confirmed world changes by a tick the machine commits,
  and by a baseline it installs: on joining, on returning within the grace, and in a repair. So a best the
  host committed while a guest's link was down is on the guest's disk when the baseline that holds it is
  installed, though no tick the guest stepped changed it. A step run ahead writes nothing, a replay writes
  no slot, and a bot has the default and no file.
- **"Its own player" is a stay, not a seat.** A machine keeps the `Player` its value landed for, seat and
  count of joins together, and writes for that `Player` alone. A return within the grace is the same
  `Player`, so what the baseline holds for it is reconciled. When the grace has run out the stay is over:
  whoever a later baseline shows in that seat is another `Player`, whose value this machine never writes,
  and the returning person joins as a new stay, whose value comes in from their slot again. What the world
  confirmed for a stay after its machine last heard of it is lost with the stay. Before a stay's value has
  landed, and after the stay is over, its machine writes nothing, so neither a seat still at its default
  nor a seat's return to the default as its player leaves is ever written.
- **A profile is its player's word.** `#[save(validate = f)]` runs on every machine as the value lands, and
  one that fails it, or its check, lands as the default and the player joins. A write that fails leaves the
  slot behind the world, so the next comparison writes it again, and it shows in `saves.status(slot)` on
  that machine. The first release builds it (O8).
- **Versions.** A save's version and its chain of older types are declared from the first release, in
  [the `rules` module](#1-a-game-one-crate-one-package-one-value) because every machine runs the conversion,
  and a test restores an old file. The promise that a released save loads in a later build is year one's (D4).

**Settings.** `#[derive(Settings)]` on a struct makes each field a control on the panel, with its default
beside its range. They are `thrum-controls`' typed controls ([CREATOR](../CREATOR/DESIGN.md)), so the Mac
panel, the Chrome page and parameter files work as they do today, and the profiler is in every game's panel.

| | A shared setting: `Game::settings::<T>()` | A local setting: `Game::local_settings::<T>()` |
|---|---|---|
| Owner | The world. `T` is a game resource | This screen, kept on this machine |
| Read by | Rules, control functions and frame functions, as `Res<T>` | Frame functions, as `Screen<T>`. A rule cannot take it |
| Fields | What a resource may hold. A raw float is a compile error | Those, and `f32` |
| A panel edit | Enters as a command and changes the world at a tick. A replay holds it | Changes this screen at once |
| A frame function's edit | None: it sends a command, and a rule writes the setting | Through `Screen<T>`: it changes this screen at once and is kept on this machine, as a panel edit is. So an options menu is a frame function |
| In a match | The host's edits are the ones admitted | Each player has their own |

**Names:** the `Content`, `Save` and `Settings` derives, `ids!` and `Slot` in [R1](#r1-describing-a-game); `Kept` in
[R2](#r2-what-world-state-holds); `Content`, `Saving`, `Keep` and `Restored` in [R3](#r3-what-a-rule-may-take); `Saves` in
[R4](#r4-what-a-frame-function-may-take); the catalog in [R5](#r5-looks-cameras-the-interface-and-assets).

### 11. Multiplayer, an opt-in over input-only networking

- **Multiplayer is one line and changes no rule** (K3, D3): `Game::multiplayer(Multiplayer::players(1..=4))`
  with the `multiplayer` feature. A game played alone is the same game with one player.
- **Only input travels** (D1). Every machine steps the same rules from the same admitted input: held actions,
  commands, answers, the roster's changes, a pause, a scene request, the pieces of a save. Nothing of the
  world is replicated while a match runs, so 700 enemies cost the link what none do. A joiner is sent the
  world once, as a baseline, and then steps it. Digests are compared, and a difference is repaired.
- **The SDK implements `SESSION`'s world for the game**, and a game writes none of the steps of
  [the session guide](../../SESSION.md): the rule-set identity, from the game's rules, content and build and
  what its description registers as the program built it, which is each scene's entries in their order and
  every starting value; the world image and digest, from the registered state; the
  action wire, from the action maps and commands; the roster, as [seats](#5-players-actions-and-control); and
  the driven world. Who does what is `SESSION`'s and unchanged: a native process hosts, browsers and the
  simulator join, the world is shown at the confirmed tick, and a match ends with its host.
- **Nothing in a match knows 2D from 3D** (O6). What travels is input, what is compared is a digest of
  registered state, and what runs ahead is a control function. `Transform3d`, `Vec3`, `Rot3` and `Angle` are
  the profile's numbers, with conformance vectors on the three targets, so a 3D world digests, rewinds and
  joins as a 2D one does. A 3D control function runs ahead on the same rows of
  [the table](#5-players-actions-and-control): its components, its action map with its `Heading`, the tick,
  and the level's fixed collision, which is `Level3d` where the scene has 3D space and `Level2d` on the
  ground plane where it has not. The local player's character is shown at its predicted `Pose3d`, and a camera rig that
  follows it or reads its `Heading` has no delay. The first release proves it twice: a rehearsed 3D match
  with no window in WP5, and the 3D starter hosted on the Mac and joined from Chrome in WP14.
- **What a player's own machine runs ahead has three depths.** Each is a declaration, and the first release
  builds the first. Depth 1 changes no rule. Depth 2 predicts what the control function itself launches, so
  firing written as a rule of its own moves into that function, and no rule that judges a hit changes.
  Depth 3 is asked for by the rule that judges the hit, in the one line that names the space, and changes
  no other rule:

  | Depth | What answers at once | How a game says it | First release |
  |---|---|---|---|
  | 1 | The player's own character, against fixed collision | A control function that reads only what can be run ahead | Yes |
  | 2 | What that character launches, and the other players it runs into | `.predicts(..)` and `.against(..)` on the control line, with the launch made by the control function ([entry](#predicted-projectiles-and-contact)) | No: first in the release after (O8) |
  | 3 | A hit judged against what the shooter's screen showed | `space.as_seen_by(player)` in the rule that judges it, and `#[seen]` on a button whose press decides a shot ([entry](#hits-judged-as-the-shooter-saw-them)) | No: after depth 2 (O8) |

  Until depth 2 is built a player's shots and the players they touch are shown at the confirmed tick, and
  until depth 3 a hit is decided at the shared tick. The guides say both plainly.
- **How depth 1 runs.** The SDK gives the control function to `SESSION`'s predicted controller as its step.
  The controller's state is everything the next step depends on: the character's entity and the components
  the function names; the clock it can read, which is the game's tick and the ticks at which the running
  scene and its phase were entered; this player's input state; whether the match is paused; and what the
  function reads of the confirmed world, which is the resources it names, the phase and the level's identity.
  Content and the seed are the rule set's constants.
- **The whole state is compared with the host's, bit for bit**, so a change to anything the step reads
  corrects it even where the character has not moved. **A step run ahead equals the host's step of that tick
  when the host admitted the frame this machine stepped with and nothing the step reads had changed on the
  host that this machine had not yet been sent.** Where either fails, the tick is corrected when its
  confirmation arrives: the controller returns to the confirmed state and replays its unconfirmed input, and
  the SDK eases the shown pose to the corrected one over a few pictures, except across a `teleport`.
- **Under a bad link.** A player whose input is late keeps their last axes for a moment and then idles, and
  their `PlayerStatus` says so. Nobody else waits for them. A player whose link drops is `Away` for
  `Multiplayer::grace(seconds(30))`, keeps their seat, score and character, and has left after it.
- **The lobby is an ordinary scene on the match's clock** (R8), of type `Lobby`, and the first scene of a game
  that has one. When a player chooses *Host*, the SDK starts `SESSION`'s host and its ticks at once with the
  lobby running, and a peer who joins is sent it as it stands. So a ready mark, a host's panel choice and
  *Start* are input at a tick and world state. *Play alone* is the same lobby with no session.
  `scenes.go_to(Lobby)` returns everyone to it. The host's panel is a frame function of the lobby, so it
  reads content to list what there is to pick. What the host chose stays a game resource, so every later
  round and a rematch read the same plan with nothing copied from scene to scene.
- **The standard lobby continues a saved run.** `Lobby::standard().continues::<T>(slot)` shows the host
  *Continue*, beside *Start*, where that slot on the host's machine holds a save of the match's. *Start*
  enters a scene afresh. *Continue* enters one with
  [the save restored](#10-content-assets-saves-and-settings), as `request.scene(..).restoring` does. So a
  host and a friend take up a journey through the standard lobby.
- **The save says which scene it continues in, where a game needs it to.** `continues` enters the scene
  `starts` names, which is right for a game whose every run begins in one place. `.continues_with(slot, f)`
  takes a read-only function of the save that returns the scene to enter,
  `fn fell_from(journey: &Journey) -> Fell`, as `starts_with` takes one of the setup. Every machine asks it
  at the tick the save lands, once the save has passed its check and been carried forward through its
  chain, so it is defined in the `rules` module. So a journey saved in region B is continued in region B
  from a lobby whose *Start* begins in region A, and a scene that checks its save against its own value
  passes. *Start* is not changed by it.
- **A host needs no window** (O8). `cargo play --host` runs the game with `thrum`'s default features off and
  prints the ticket to join by ([entry](#smaller-entries)). No line of the game changes. What the host's
  screen would do in the lobby falls to the lowest seat's player: the panel, *Start* and *Continue*.
- **A host with no window brings its save in before the first tick.** It has no screen to find a file from,
  and `SESSION`'s host with no window has no slot of its own to send one from. So nothing of the save
  travels as input:
  - **The host's process reads the slot**, once, before the first tick, as it reads a saved resource: the
    slot its lobby's `continues` or `continues_with` line names. Where that holds a save of the line's
    type that passes its check and is of no newer version, **the world the host seeds holds the save's
    bytes**, in the state a restore's pieces gather into. The host prints which it found: a save held, an
    empty slot, or a file it could not take, with the reason.
  - **Whether there is a run to continue is read from the world**, the same on every machine. The lobby
    shows *Continue* to the lowest seat's player where the world holds a save, whatever that player's own
    storage holds, and shows *Start* alone where it holds none.
  - **Their press carries no bytes.** It is a request admitted at a tick, as *Start* is. At that tick every
    machine decodes the save the world holds, asks the `continues_with` function and enters the scene, as
    for a restore whose last piece had just landed, and one that fails leaves the world as it was. A peer
    who joined the lobby was sent the bytes in its baseline, and a recording holds them with the rest of
    how the run started. No player's machine reads or sends the host's file.
  - **The world lets the bytes go when the lobby is left**, by *Start* or by *Continue*, so no tick of
    play digests them. The file is read once: a run a rule saves later in that match is on the host's
    disk for the next time it hosts, and a lobby the same match returns to offers *Start* alone.
  - **What only a host's own screen can ask stays with a host that has one**: a save asked for by a frame
    function, and a game's own `request.scene(..).restoring`. On a host with no window `saves.can_save` is
    false on every screen, a `restoring` request is refused with a message that names the lobby's
    `continues`, and a rule's `Saving` writes the host's slot as it always does.
- **`Session`**, in a frame function, is for a game with its own screens, and for a screen that asks whether
  it is alone.
- **Not offered in the first release**: depths 2 and 3 above, and
  [host migration](#smaller-entries).

**Names:** `Multiplayer` and `Lobby` in [R1](#r1-describing-a-game); `Session` in [R4](#r4-what-a-frame-function-may-take); `TestMatch` in [R6](#r6-tests-and-errors).

### 12. Mistakes that explain themselves

- **Every message names the object, the cause and the next step, and has a stable code** (`T0412`) that the
  guide *When something goes wrong* explains with a failing example and its fix. `cargo play --check
  --message-format=json` gives each as a record with its code, source span and suggestion, for an agent.
- **A mistake is caught at the earliest moment that can know it**: by the compiler where a type can say it,
  on the SDK's own traits so the words are the SDK's; by the rules check; at startup, in `test()` and `run()`
  alike, as the error they return, before the first tick; and otherwise in the tick, which is refused with
  the world unchanged.
- A message says only what its check knows. `thrum::Error` is one enum with a variant for each kind of remedy,
  in [the adopted error shape](../../../.agents/skills/thrum-engineering/references/patterns.md).

| When | Mistake | The message says |
|---|---|---|
| Compile | A rule takes `Ui`, `Screen<T>`, `Shown<L>`, `Draw2d`, `Outbox`, `Me`, `Pause` or another frame parameter | That a rule must give the same result on every machine, and to move this to a frame function |
| Compile | A frame function takes `ResMut<T>`, `Entities`, `Emit<T>`, `PhaseMut<P>`, `PlayersMut` or a `&mut` query | That frame functions cannot change the world, and to send a command or write a rule |
| Compile | A spawn function, a condition or a capture takes a parameter that writes | That it may only read, and which parameter to change |
| Compile | A bare `Player` spawned, inserted or removed as a component, or a query that asks for `&mut Player` | That the SDK gives a character its `Player`, and to keep one in a component of the game's own |
| Compile | A frame function asks for `&Transform2d` or `&Transform3d` | To ask for `&Pose2d` or `&Pose3d` |
| Compile | World state holds a `Vec`, a `String`, a reference, a raw float or `usize`; a content record holds a raw float | What to use: `Num`, `Text<N>`, `List<T, N>`, content, or a local setting |
| Compile | A spawn of a `Prefab<N>` without its `N`; `keep.all` of a `Prefab<N>` into a `Kept` that does not list its `N`, or a `renamed` to such a prefab; `invert_y` on an action that is not an `Aim`; `Sprite::new` given a mesh; a field of an action map with no binding; a first scene with no default | Rust's own type error, with the SDK's note on what is missing |
| Check | Raw-float arithmetic, an unordered container, a file, a clock or an outside crate in the `rules` module; a path from it to the rest of the game | The line, why it can differ between machines, and what to write |
| Check | `#[cfg(..)]`, `#[cfg_attr(..)]` or `cfg!(..)` in the `rules` module | The line; that a build setting would choose the rules, so a pass could not say which rules are played; and to give the value from `game()` or a shared setting, or for a test to move it out of the module |
| Startup | One parameter names a component twice and writes it; two parameters of one rule, or a control function's query and its character, conflict over a prefab | The function, the parameters, the component and the prefab, and what to drop |
| Startup | Two accesses of one function reach one resource and one of them writes it, through a `Params` struct included; a writing parameter of the SDK's taken twice | The function, both parameters and the resource, and which to drop |
| Startup | A scene calls `phases` twice, has two `players().spawn(..)` lines, or has two control lines that could run in one phase | The scene and the lines, and what to write instead |
| Startup | A rule reads a resource nothing provides | The rule, the resource, and `scene.resource(..)` or `Game::resource(..)` |
| Startup, and from `game.run` | A closure or a function pointer takes `Random` | The callable; that draws are keyed by a function's name, which a closure and a pointer do not have; and to write a named function |
| Startup, and from `game.run` | Two functions that take `Random` have one name and are not one function | The path they share; where each is listed, or that a test ran the first; that a draw is keyed by its function's name; and to give one of them another name |
| Startup | A function that makes or changes the world is defined outside the `rules` module, in a game that has one | The function and the module |
| Startup | A function pointer is registered in a game that has a `rules` module | The route and the pointer's type; that a pointer's type does not say where its function is defined; and to list the function by its name |
| Startup | The action maps do not fit an input frame; a command type is too large | The counts or the size, and the limit |
| Startup | A declared prefab name has no definition; a definition names an asset that does not exist; an `ids!` block names a record that does not exist; a content file fails its type, names a record that does not exist or fails the game's validator | The prefab, the constant or the file, the field, and the nearest name |
| Startup, in `run()`, with multiplayer on or given a recording | The game has no `rules` module | The functions that change the world, and `mod rules { .. }` to put them in |
| Startup, in `run()` | The check has recorded no pass for the rule sources the program was built from: they changed since it last passed, or were never checked | That the rules are not the ones the check passed, and `cargo play` or `cargo play --check` |
| Startup | With multiplayer on: a control function reads what cannot be run ahead | The parameter and `.unpredicted()` |
| Check, with multiplayer on | A component a control function writes is also written by a rule, such as a `Diver` that holds both a kick and a cargo | A note, not a refusal: the control line, the component and the rules that write it; that each such write reaches the player as a correction; and to split the component |
| Startup, in `run()` | No asset store, a stale asset catalog, or a missing prerequisite | What is missing and the command that provides it |
| Running | A spawn of a component the world does not know; a spawn past the world's size; more fixed shapes than the level's recipe holds | The rule, what ran out or is unknown, and the line to add |
| Running | `Single` finds none or two; `entities.part` on a prefab with no such part; `level.mark` finds none or two | The rule and what was found |
| Running | An entity, or a player's character, comes to be reached by two parameters of one function over a component one of them writes | The rule, the parameters, the component and the entity, and for a character its player |
| Running | `keep_world` where it is refused | The child, the parent and the cause, with the residual and the bound; the child is where it was |
| At a capture | An entity of a prefab `keep.all` names lacks a component the `Kept` lists | The prefab, the entity and the component, and to list what every such entity has. No file is written, and `saves.status(slot)` says so |
| Running, on screen | An answer or a command is admitted while a modal holds the pause, and no rule marked `.even_when_paused()` takes it | Once: its type, that it waits for the pause to end, and to mark the rule that takes it |
| Running | `go_to` a scene the game was not given; a `request.scene` the running scene does not accept; `possess` of an entity that lacks a component the control function takes | The scene, or the entity and the component, and what to add |
| Running | A rule returns an error, or panics | The rule, the tick and the error, [as each target allows](#3-rules-and-the-tick) |
| Running, on screen | An entity with a look in a scene with no camera, or a 3D look under a 2D camera; a named asset that matches nothing | Once: the scene and the look, or the name and the nearest names |
| Running, on screen | A session is refused or closed, or the match has no seat for this player | `SESSION`'s reason, or that the match is full, in words a player can read |
| Running, on screen | In a match, a save or a restore asked for by a screen that is not the host's own, a host with no window's lowest seat included | Once: that the match's saves are on the host's machine; for a guest, that the host asks; for a host with no window, the lobby's `continues` line and a rule's `Saving` |

### 13. Tests without a window, recordings and pictures

- **`game().test()` is the game with no window, device or clock, on the same rules and the same engine path
  as `run()`** (R9). A game played alone has its one player from the start, with their character.
- **Starting is fallible, and fixtures go in before it.** `test()` is `test_setup().start()`, which in order
  installs the fixtures; checks the content; runs every startup check; gives each saved resource its fixture
  or its `Default`, since a test reads and writes no real storage; and enters the first scene. The first
  failure is the error it returns. No tick has been stepped when it returns. A match takes the same
  fixtures: `test_setup()` ends in `start()` or in `start_match()`.
- **A test can tell "not there" from "not yet".** Lookups return a `thrum::Result` whose error says which: a
  join still waiting for its tick, a player with no character, an entity that is gone.
- **What a test changes, it changes between ticks**, and it has landed when the call returns. `game.run(rule)`
  runs any rule function once the same way, so a game's own helpers that take `Entities` or a `Query` set a
  test up. A named function run this way keeps [its name](#3-rules-and-the-tick), so it draws from `Random`
  what a scene's first run of it draws at that tick, however often the test calls it, and the call is not
  counted. The test game notes which function it has run under each name, so that a second function of
  that name which takes `Random` is refused before it runs. The note is the harness's own and is in no
  digest. `game.run` registers nothing, so [it takes a function pointer](#1-a-game-one-crate-one-package-one-value)
  in a test of any game.
- **`TestScreen`** runs the scene's frame functions for one player against the world as it stands, with no
  renderer, so a test can open a pause menu, pick an upgrade card and see `offer.status()` change. Each call
  on it is one picture.
- **`game().test_match()`** is a host and its peers in one process, each with a world of its own, stepped in
  simulated time over `SESSION`'s rehearsal. It proves a late join, a slow peer, a choice made twice and a
  dropped link, in 2D and in 3D.
- **A recording** is the input a run admitted, with the game's seed, the scene it started in and the value
  each saved resource started with. A profile read before the first tick, which is a lone player's and a
  host's, is such a value and is held the same way, so a run replays alike on a device with another unlock
  history. A guest's is in the input, as the pieces it was admitted in. Every game that runs has checked rules
  ([section 1](#1-a-game-one-crate-one-package-one-value)), so every recording has them behind it, the
  one-file starter's included, and a recording made on one device replays on another (D1).
- **A recording holds each input at the committed tick that admitted it**
  ([the two counts](#4-scenes-phases-time-and-pause)). That keeps which inputs were admitted together, and
  how many paused ticks the run committed, as a gap in the numbers and never as a wait. A replay commits
  the same ticks, the empty ones without waiting on a clock, so it ends at the run's committed tick with
  the run's digest, and equals it at every committed tick between. After a change to the rules it replays
  the same input against the new rules, which is what makes it a test that a level is still beatable.
- **A picture.** `cargo play --shot <file>` runs a game offscreen for a number of ticks, under a recording if
  one is given, and writes the frame the engine submitted, through [T2 capture](../T2-CAPTURE/DESIGN.md). It
  needs no screen and no permission.

**Names:** the harness, with what each call returns, in [R6](#r6-tests-and-errors).

### 14. Running a game, documentation and templates

One command for each target, the same for every game, from the game's folder. The template ships them as
Cargo aliases onto the engine's tool. The game's `main` is the same file on every target.

| Command | What it does |
|---|---|
| `cargo play` | Cooks and watches `assets/`, writes the asset catalog, checks the rules, builds, opens the Mac window with its panel |
| `cargo play --web` | The same for Chrome: builds for wasm32, serves the page and the store on loopback, prints the URL |
| `cargo play --ios` | Builds and starts the game on the iPhone simulator, through [GAME-UI](../GAME-UI/DESIGN.md)'s one shell |
| `cargo play --check` | The rules check and every startup check, with no window. A pass is recorded, so a build Cargo makes of the same rule sources runs. `--message-format=json` for an agent |
| `cargo play --record <file>`, `--replay <file>` | Records a run while a person plays, and shows one. The one-file starter records as it stands |
| `cargo play --shot <file>` | Writes a picture |
| `cargo play --scene <name>` | Starts in another scene, with its default value, or with the value given in RON where it has none: `--scene 'Level(def: "cellar")'` |
| `cargo play --host [--port <n>]` | Hosts a match with no window, and prints the ticket to join by and what it found in the slot its lobby continues from (O8) |

Every one of these refuses a game with no `rules` module, with the functions to move and the step to take
([section 1](#1-a-game-one-crate-one-package-one-value)). A game started any other way, such as `cargo run`
or an editor's run button, goes through the same `run()`, which plays only rules the check has passed as
they were built.

- **Reference.** Every public item of `thrum` has a doc comment with an example, and every example is a doc
  test. `./dev check` fails a public item without one.
- **Guides**, in `docs/sdk/`: *Start here* (the four sentences and the fifteen words); *Your first 2D game*,
  whose first page ends with a ship you steer; *Your first 3D game*; *Make it multiplayer*; *Numbers*; *How do
  I…* (short answers by task); *When something goes wrong* (by code). A guide teaches only what is built, and
  each code block in a guide is compiled by the docs check.
- **One file for agents.** `docs/sdk/REFERENCE.md` is generated from the reference and the guides, and the
  docs check fails when it is stale.
- **Templates**: `2d`, `3d` and `multiplayer`. `./dev new <folder> --template <name>` writes a game that
  builds, runs, records and passes its check as written, with `src/lib.rs` holding `game()`, `src/rules.rs`,
  a test, and an `AGENTS.md` that points an agent at the one file. A game made this way lives outside the engine
  checkout and reaches `thrum` by path.
- **Judged by a change the guides did not show**, as well as by a clean start: a second enemy variant, a
  reconnect during a choice.

## The reference

Every name a game touches in the first release, once, with its methods, so that a reader finds a name and
does not guess one (K10). [The core](#the-core) says what each means and what always holds. An entry of
[the extension map](#the-extension-map) lists its own names, and the entries the first release builds
([3D space](#3d-space-and-a-kinematic-character), [camera pitch](#camera-pitch-and-rigs),
[gamepads](#gamepads) and [audio](#audio)) are part of this reference. A spelling here is
[a builder's to change](#what-a-builder-may-change), in a line of the log.

**Index.** The group that holds each type:

`Actions` R1 · `Angle` R2 · `Answer` R3 · `Asked` R3 · `Away` R1 · `Axis` R2 · `Backdrop` R5 · `Bindings` R4 ·
`BotInputs` R3 · `Bundle` R3 · `Button` R2 · `Camera2d` R5 · `Camera3d` R5 · `Collider2d` R2 · `Color` R5 · `Commands` R3 ·
`Content` R3 · `Cooldown` R2 · `Delivery` R4 · `Disabled` R3 · `Draw2d` R4 · `Draw3d` R4 · `Emit` R3 ·
`Entities` R3 · `Entity` R2 · `Error` R6 · `Events` R3 · `Family` R3 · `Found` R3 · `FoundTiles` R3 ·
`Frame` R4 · `Game` R1 · `Ground` R5 · `Heading` R2 · `Hit` R3 · `Id` R2 · `InputContext` R1 · `Inputs` R3 ·
`Joined` R3 · `Keep` R3 · `Kept` R2 · `Key` R1 · `Keys` R1 · `Layer` R2 · `Layers` R2 · `Level2d` R3 · `LevelMut2d` R3 ·
`LevelPiece` R2 · `Light` R5 · `Link` R6 · `List` R2 ·
`Loaded` R4 · `Lobby` R1 · `LocalActions` R1 · `Mark` R1 · `Material` R5 · `Me` R4 · `Model` R5 · `Mouse` R1 · `Move2d` R3 ·
`Moved2d` R3 · `Multiplayer` R1 · `MyInput` R4 · `MyOffer` R4 · `Near` R3 · `Num` R2 · `Offer` R3 ·
`OfferId` R2 · `Offers` R3 · `Outbox` R4 · `Params` R1 · `Part` R1 · `Pause` R4 · `Pausing` R1 · `Peer` R6 ·
`Pending` R3 · `PerPlayer` R2 · `Phase` R3 · `PhaseClock` R2 · `PhaseMut` R3 · `Placed` R3 · `Player` R2 · `Players` R3 ·
`PlayersMut` R3 · `PlayerStatus` R3 · `Pointer` R4 · `Pose2d` R4 · `Pose3d` R4 · `Prefab` R2 · `PrefabDef` R2 ·
`Query` R3 · `Random` R3 · `Rect` R2 · `Request` R4 · `Res` R3 · `ResMut` R3 · `Restored` R3 · `Result` R6 ·
`Rot3` R2 · `Save` R1 · `Saves` R4 · `SaveStatus` R4 · `Saving` R3 · `Scene` R1 · `Scenes` R3 · `Screen` R4 ·
`Seconds` R2 · `Session` R4 · `Settings` R1 · `Shape2d` R5 · `Shown` R4 · `Side` R2 · `Single` R3 · `Sky` R5 ·
`Slot` R1 · `Space2d` R3 · `Spawn` R3 · `Sprite` R5 · `Stick` R2 · `SunAngle` R5 · `Swept` R3 · `Tally` R3 · `TestGame` R6 · `TestMatch` R6 ·
`TestScreen` R6 · `TestSetup` R6 · `Text` R2 · `Tick` R3 · `TickNumber` R2 · `Tile` R3 · `Tilemap` R3 ·
`Tileset` R1 · `Timer` R2 · `Touch` R1 · `Transform2d` R2 · `Transform3d` R2 · `Ui` R5 · `Vec2` R2 · `Vec3` R2 · `With` R3 ·
`WithDisabled` R3 · `Without` R3

### R1. Describing a game

```rust
Game::new("Skylark")
    .scene(f) | .scene((rules::level, screens::level))   // one function, or several that describe one scene
    .add(rules::prefabs)                        // fn(Game) -> Game, to keep a part of the description elsewhere
    .resource(value)  .component::<T>()  .prefab(NAME, definition)
    .settings::<T>()  .local_settings::<T>()  .content::<T>("folder")
    .saved::<T>(slot)  .save::<T>(capture)  .save_if::<T>(capture, condition)    // the match's saves
    .profile::<T>(slot)                         // each player's own, on their own machine
    .multiplayer(Multiplayer::players(1..=4))  .lobby(Lobby::standard().starts(Arena))
    .max_entities(n)  .tick_rate(hz)
    .run() -> thrum::Result
    .test() | .test_in(scene) | .test_setup() | .test_match()      // R6

// A scene function: fn level(scene: &mut Scene<Level>). Each line is a place in the written order.
scene.resource(value)  .on_enter(f)  .on_exit(f)  .rules((..))  .add(part)
scene.players()
    .spawn(hero_at_start)       // a prefab, or a read-only function that returns what to spawn
    .control(run_and_jump)      // runs here in the written order, once for each player's character
    .during(Stage::Playing)     // optional: only in this phase
    .when_away(Away::Idle)      // optional: Idle (the default); Remove; Bot: the seat counts as a bot while away,
                                // so `BotInputs::is_bot` is true of it and a rule writes its input
    .unpredicted();             // required in a match when `control` reads what cannot be run ahead
scene.phases(Stage::Playing)
scene.phase(Stage::Wave)
    .on_enter(next_wave)  .rules((steer, fire, hit))  .on_exit(clear_enemies)
    .lasts(seconds(60)) | .lasts(round_length)             // a time, or a read-only function asked once, on entry
    .then(Stage::Intermission) | .then_run(next_round)     // a phase, or a rule run once when the time is up
scene.accepts::<Level>()  .accepts_from_anyone::<Level>()  .unpausable()
scene.space2d(Space2d::cells(4.0))  .level_shapes(n)       // part of the rules, so the same for every player
scene.tiles(Tileset::new(art::TILES)                       // how this scene reads a tile map
    .solid('#', TERRAIN)  .one_way('=', TERRAIN)  .marks('@', START)
    .spawns('o', COIN)  .spawns('m', moth_at)              // a prefab, or a read-only function given `Placed`
    .draws(':', art::tiles::VINE)                          // drawn and not solid; a space is nothing
    .tile_size(1.0))
// The lines that describe a screen:
scene.camera(..)  .background(colour)  .backdrop(Backdrop::new(art::SKYLINE).parallax(0.3))
scene.sky(Sky::clear())  .ground(Ground::flat())  .place(model).at(x, y, z).turned(angle).scaled(s)
scene.each_frame((animate_hero, status, pause_menu))  .on_show(f)  .screen(value)  .input_context(NAME)

// On a rule or a group, where a scene lists it:
scene.rules((steer, fire.every(seconds(0.4)), (hit, die).when(wave_is_on), end_talk.even_when_paused()));
scene.rules((foes_think, foes_move).during((Stage::Dark, Stage::Reckoning)));   // one phase, or a tuple of them

fn hero_at_start(joined: Joined, tiles: Level2d) -> impl Spawn      // joined.player; any read-only parameter
fn moth_at(placed: Placed) -> impl Spawn                            // placed.at .cell .glyph; any read-only parameter
// A spawn function returns `impl Spawn`: a prefab at a place, `HERO.at_point(start)`, or a tuple of components,
// `(Shape2d::triangle(1.0), Transform2d::at(x, -7))`, as `entities.spawn` takes either (R3)
fn run_and_jump(hero: &mut Hero, at: &mut Transform2d, body: &Collider2d, controls: &Controls, tick: Tick)

Multiplayer:  Multiplayer::players(1..=4)  .pause(Pausing::Nobody | Host | Anyone)  .grace(seconds(30))
Lobby:        Lobby::standard()      // alone, host or join by a link, a paste or a launch argument; who is here; why a match closed
              .ready_up()            // a ready mark each player sets for themselves
              .host_panel(pick_arena)                       // each change is sent as the host's command
              .starts(Arena) | .starts_with(arena_from)     // asked at the tick *Start* is admitted
              .continues::<Journey>(Slot::AUTO)             // *Continue* for the host, where the slot holds a save:
                | .continues_with(Slot::AUTO, fell_from)    // into the scene `starts` names, or the one the save says
// fn pick_arena(ui: &mut Ui, setup: &mut Setup, arenas: Content<ArenaDef>): any read-only frame parameter after the two
// fn arena_from(setup: &Setup, arenas: Content<ArenaDef>) -> Arena: any read-only parameter after the first
// fn fell_from(journey: &Journey) -> Fell: the scene a save continues in; any read-only parameter after the first
// `Setup` is the game's own `Data` type. An `Id` has no default, so the game writes one that names a declared record:
//   impl Default for Setup { fn default() -> Self { Setup { arena: HARBOUR } } }
// The lobby keeps it in the world as a game resource: every screen shows the host's choice, a late joiner is sent
// it, and every later scene reads it as `Res<Setup>`.

prefabs! {                                              // a game's names, declared together
    pub const BAT: Prefab = "bat";
    pub const SHOT: Prefab<(Shot, Velocity)> = "shot";  // needs these from whoever spawns it
}
ids! { pub const FIRST_WAVE: Id<WaveDef> = "first"; }   // an ID the code names; checked at startup
Layer::new("enemies")  Mark::new("start")  Part::new("back")  InputContext::new("shopping")   // typed constants
Slot::new("records")   Slot::AUTO                       // InputContext::PLAY is the default context
```

| Derive | Its attributes | What it gives |
|---|---|---|
| `Component`, `Resource`, `Event`, `Command`, `Data` | — | Plain data of [the kinds world state holds](#2-the-world-entities-components-resources-and-prefabs). Each is `Data` too, so one derive lets a type nest in another and in a save. Each also gives `Clone`, `PartialEq` and `Debug`, as `Save` does, so a test compares two values |
| `Scene`, `Phase` | — | `SceneType` on a struct whose fields the scene is entered with; `PhaseType` on an enum, which is `Data` too, so a save keeps a phase. Bounds are written on those traits, and a game writes the derive and never the trait |
| `Actions`, `LocalActions` | `#[bind(..)]` or `#[sent]` on each field, `#[context(NAME)]` on the struct | A map of `Button`, `Axis`, `Stick` and `Heading` fields; a name for each field, `Controls::JUMP`; and `Controls::idle()`, nothing held and every value at zero |
| `Content` | `#[content(validate = f)]`, with `fn(&T) -> thrum::Result`, which may take `Content<U>` parameters after the record | `ContentType`. A record may also hold `String`, `Vec<T>`, a `Tilemap`, a typed asset name and a `Prefab<N>` |
| `Save` | `#[save(version = 2, from = RecordsV1)]`, `max_kib = n`, `validate = f` | A versioned save, which may hold `Kept<N>` fields. `impl From<RecordsV1> for Records` carries an old file forward |
| `Settings` | `#[slider(4.0..=14.0, default = 9.0, label = "Run speed")]`, which also takes `step = 0.5` and `unit = "°"`; `#[toggle]`; `#[readout]` | A control on the panel for each field |
| `Params` | — | A struct of rule parameters, or of frame parameters, that is itself a parameter, with methods of the game's own |

```rust
#[derive(Actions)]
pub struct Controls {
    #[bind(Keys::A_D, Keys::LEFT_RIGHT, Touch::stick_x())]                  pub run: Axis,
    #[bind(Keys::WASD, Keys::ARROWS, Touch::stick())]                       pub walk: Stick,
    #[bind(Key::Space, Mouse::LEFT, Touch::button("Jump"), Touch::tap())]   pub jump: Button,
    #[bind(Mouse::look_x(), Touch::look_x())]                               pub look: Heading,
    #[sent]                              pub aim: Heading,   // no device: a frame function sets it through `MyInput`
}
#[derive(LocalActions)] #[context(SHOPPING)] pub struct ShopKeys { /* the same kinds of field */ }
// What `#[bind(..)]` takes, by the kind of field it fills:
//   Button:   Key::Space, Key::Escape, Key::E and every other key by its name;  Mouse::LEFT, RIGHT, MIDDLE;
//             Touch::button("Jump"), a button drawn on a touch screen;  Touch::tap(), a tap anywhere else
//   Axis:     Keys::A_D, Keys::LEFT_RIGHT, Keys::W_S, Keys::UP_DOWN, a pair of keys;  Touch::stick_x(), one
//             axis of the drawn stick;  Touch::drag_x(), a finger dragged left and right anywhere on the screen
//   Stick:    Keys::WASD, Keys::ARROWS;  Touch::stick(), a stick drawn on a touch screen
//   Heading:  Mouse::look_x();  Touch::look_x(), a drag on the half of the screen with no stick
//   A pad:    for each kind, the names the entry "Gamepads" lists: Pad::SOUTH, Pad::LEFT_STICK_X, Pad::LEFT_STICK

#[derive(Content)] #[content(validate = WaveDef::validate)]            // thrum::Error::invalid(field, why)
pub struct WaveDef { pub enemy: Prefab, pub count: u32, pub speed: Num, pub next: Option<Id<WaveDef>>, pub taunts: Vec<String> }
// In a RON file a `Num` and a `Seconds` are decimals, a vector is `(1.5, 0.0)` and an `Angle` is `Degrees(35)`.

#[derive(Save)] #[save(version = 2, from = RecordsV1)]
pub struct Records { pub best: List<(Id<LevelDef>, Seconds), 64>, pub coins: u32 }

#[derive(Params)]                           // 't is the tick the rule runs in
pub struct Strikes<'t> { shades: Query<'t, &'t mut Health, With<Shade>>, ledger: ResMut<'t, Ledger>, struck: Emit<'t, Struck> }
impl Strikes<'_> { pub fn strike(&mut self, shade: Entity, damage: i32, by: Player) { /* .. */ } }
fn pulse_halos(halos: Query<(&Halo, &Transform2d)>, space: Space2d, mut strikes: Strikes) { /* .. */ }
```

### R2. What world state holds

```rust
// Numbers (section 8). In a frame function a Num or a vector becomes a float with .f32(), an Angle with .radians().
Num:          num(6.5), a const;  Num::from(integer);  sqrt abs floor ceil round min max clamp lerp sign;  to_i32()
Vec2, Vec3:   vec2(x, y), vec3(x, y, z);  Vec2::lit(0.8, 1.4), a const;  ZERO;  fields x, y and z;
              length  length_squared  normalized_or_zero  clamp_length  clamp_to(rect)  distance  dot  lerp;  in 3D, right()
Angle:        Angle::degrees(35.0), ::turns(0.1), ::of(vector) (the way a vector points), ::between(a, b);
              sin  cos  direction  direction_xz  toward(target, at_most) (a turn limited to a rate)
Rot3:         Rot3::IDENTITY, ::about_y(angle), ::facing(direction);  forward() -> Vec3,  heading() -> Angle
Seconds:      seconds(0.4);  Seconds::lit(0.4), a const;  ZERO;  adds, subtracts and scales by a Num;
              one divided by another is a Num;  num() -> Num, to multiply a speed by
Rect:         Rect::new(min, max), ::centered(point, w, h);  min, max;  center()  contains(point)

// Time, counted in the game's ticks (section 4).
TickNumber:   a tick to keep or compare: what `tick.number()` and `tick.after(d)` give
Timer:        a deadline that may not be set.  Timer::IDLE,  Timer::after(tick, wait),  start(tick, wait),  clear();
              running(tick): set and not yet reached, a window;  finished(tick): set and reached;
              left(tick) -> Seconds;  fraction(tick) -> Num, from 0 when it was started to 1 when it is reached
Cooldown:     something that may happen again after a wait.  Cooldown::READY,  Cooldown::ready_after(tick, wait);
              ready(tick);  take(tick, every) -> bool: if ready, starts the wait and returns true;
              left(tick) -> Seconds;  fraction(tick) -> Num, 1 when it is ready
              In a save a Timer and a Cooldown keep the whole wait and the time left, so `left` and `fraction`
              read after a restore what they read at the capture
PhaseClock:   a phase's whole clock, in ticks, to keep in a save: what `phase.clock()` gives and `resume` takes.
              time() -> Seconds, how long the phase has run;  left() -> Option<Seconds>: a time above zero
              while its length runs, zero in the tick the time is up until what follows has happened, None
              where it has no length or what follows has happened.  A clock an enter function of the phase
              takes reads time() of nothing and left() of the whole length.
              PhaseClock::at(time), with no length, and .ending_in(left): one made by hand, for a test and
              for a save's conversion from a version that kept only the time.  It holds the times given,
              and `resume` turns each into ticks at its game's rate as every length is turned (section 4),
              so a clock made from the `time()` and `left()` another gave resumes as that one does

// Names. Each is Copy and compared.
Entity:       one life of a slot.  `spawn` returns it, a query names it, and a component may keep it
Player:       one stay in a seat.  index() -> u32, the seat, for a place in a line or a colour;  Player::FIRST
Id<T>:        a content record's stable name, declared in `ids!` or found with `Content::find`
Prefab<N>:    a spawn is a prefab at a place, with this spawn's own differences:
              BAT.at(3.0, 0.0)  .at(x, y, z)  .at_point(p)  .at_local(x, y)  .turned(angle)  .scaled(s)
              .with(component)  .tweak(|c: &mut C| ..)  .under(parent);   BAT.variant() -> PrefabDef
              .child(spawn): a child of this spawn's own, beside the definition's;  .disabled(): it lands disabled
PrefabDef:    PrefabDef::new(bundle), or BAT.variant(), a definition that starts as another prefab's;
              .with(Health(8))  .tweak(|sprite: &mut Sprite| ..)  .child(AURA.at_local(0.0, 0.2))  .part(EYE, spawn)
              .part(BACK, ()): a part with nothing on it, a socket
Layer, Layers:  `.on(..)` takes a Layer, a tuple of them, or a Layers, a set chosen first:
              Layers::of((TERRAIN, LEDGES)),  .with(LAYER),  .without(LEDGES)
Mark, Part, Slot, InputContext:  typed constants (R1)
OfferId:      one open offer, to keep in a resource
LevelPiece:   one thing a rule added to the level: what `add_rect`, `add_segment` and `add_tiles` return and
              `remove` takes

// Text and containers.
Text<N>:      up to N bytes of UTF-8, such as a player's name.  Text::from(".."), cut at N;  as_str()  len()  is_empty()
List<T, N>:   at most N values, in order.  In a resource, a scene's fields, a save or a `Data` struct kept in one,
              and as a rule's local.
              List::new(), List::from(array);  push(v) -> bool (false when full),  pop(),  insert(i, v) -> bool,
              remove(i) -> T,  clear(),  retain(|v| ..);  len()  is_empty()  is_full()  get(i) -> Option<&T>  list[i]
              first()  last()  contains(&v)  iter()  iter_mut()
PerPlayer<T>: one T for each seat, the default again when the seat's player leaves.  In a resource or a save.
              per[player], which refuses the tick for a player who has gone;  get(player) -> Option<&T>,
              get_mut(player);  iter() -> (Player, &T) and iter_mut(), for the players here, in seat order;
              departed(player) -> Option<&T>: what a leaver's seat held, for the rest of the tick they leave in
Kept<N>:      entities a save keeps, with the prefab names it was filled from (section 10).  In a save only.
              What `keep.all(..)` returns and `entities.respawn(save)` brings back.  N lists at least what each
              named prefab's spawn must bring.  len()  is_empty()  iter() -> (&str, &N), each entity's prefab
              name and what was kept of it;  in a save's conversion, renamed(old_name, PREFAB),  map(|old| new)
              and Kept::none(), which names no prefab

// Places.
Transform2d:  Transform2d::at(x, y) .turned(angle) .scaled(s);  fields position, turn, scale;  teleport(point);
              follow(&moved): stands where the move ended.  The move's legs, with the time each took, are this
              tick's path, added to the path the entity has where the move began where it stands (section 7)
Transform3d:  Transform3d::at(x, y, z) .turned_y(angle) .facing(direction) .scaled(s);  fields position, rotation,
              scale;  teleport(point)  follow(&moved)  face(direction)  turn_y(angle)  forward() -> Vec3
              heading() -> Angle;  turn_toward(direction, at_most): a turn about y toward that way, limited to a rate
Collider2d:   Collider2d::circle(r), ::rect(w, h), ::capsule(r, length)   .offset(x, y)  .on(LAYER)
Side:         Left | Right.  Kept in a component as any name is

// Action kinds: the fields of an action map (section 5).
Button:       .held .pressed .released: bool;  .presses: how many since the last tick;  Button::held(bool), for a bot
Axis:         a Num from -1 to 1, used as one;  Axis::of(n)
Stick:        a Vec2 no longer than 1, used as one;  Stick::toward(v);  Stick::ZERO, RIGHT, LEFT, UP, DOWN
Heading:      an Angle the player's own machine keeps.  angle()  direction()  direction_xz();
              Heading::of(angle), Heading::toward(direction), for a bot and for a `#[sent]` field
```

### R3. What a rule may take

A control function, a spawn function, a condition and a capture take the read-only ones among these
([section 3](#3-rules-and-the-tick)), and [the table of section 5](#5-players-actions-and-control) says which a
match runs ahead.

```rust
Query<D, F>:  the entities that have D, in the order they were spawned.  In D, &T reads, &mut T writes and Entity names;
              Option<&T> and Option<&mut T> are Some where the entity has T, and the query visits it either way.
              F filters: With<T>, Without<T>;  Disabled, which gives the disabled entities and no others;
              WithDisabled, which gives the disabled with the rest;  and a tuple of filters, which gives what
              every one of them gives.
              iter(), or `for item in &query`;  get(e),  get_mut(e): each takes an Entity or an Option<Entity>;
              under(parent): the children of `parent` that the query matches, in the order attached;
              pair_mut(a, b) -> Option<(item, item)>: two different entities at once, None when one is missing;
              of_player(p): that player's character, where the query matches it;  count() -> u32,  is_empty(),
              par_each(|item| ..)
Single<D, F>: the one entity that matches, used as its item;  into_inner() to take a tuple apart.  None or two
              refuses the tick and says which.  Option<Single<..>> is none or one
Res<T>, ResMut<T>:  a resource, used as its value.  `Res<Level>` of the running scene's type is the scene's value
Entities:     spawn(spawn_or_tuple) -> Entity;  despawn(e) -> bool, true for the call that despawns it;
              insert(e, component), which replaces one of that type already there,  remove::<C>(e);
              respawn(save): every entity the save's `Kept` fields hold, each from its prefab, in place of
              whatever stands of the prefabs those fields name (section 10);
              attach(child).to(parent).keep_world() | .keep_local(pose);
              detach(child).keep_world() | .keep_local() | .at(pose);
              disable(e) -> bool,  enable(e) -> bool, each true for the call that changes it;
              and everything Family has.  Every edit lands when the rule finishes
Family:       the tree, read as the rule found it, for a function that changes nothing.  alive(e) -> bool,
              parent(child) -> Option<Entity>,  children(parent), in the order attached,
              part(parent, PART) -> Entity, which refuses the tick if its prefab has no such part,
              world2d(e) -> Transform2d,  world3d(e) -> Transform3d: its place in the world, through its parents
Events<T>:    read(): this tick's events, in the order sent.  The game's own, and PlayerJoined { player },
              PlayerLeft { player, name },  PlayerAway { player },  PlayerBack { player },
              CharacterSpawned { player, character }
Emit<T>:      send(event)
Tick:         number() -> TickNumber;  dt() -> Num, a tick's length in seconds, to multiply a speed by;
              seconds(),  in_scene(),  in_phase() -> Seconds;  every(d) -> bool,  after(d) -> TickNumber,
              reached(t) -> bool;  Tick::test(number), for a unit test of a pure function
Random:       draws keyed by the game's seed, the game's tick, the name of the function that asks and how many
              times it has run inside a tick at that tick (section 3).  A run outside a tick, which is a
              save's capture, its condition and a test's `game.run`, draws as the first run and is not
              counted.  A named function takes it;  a closure or a function pointer that does is refused,
              and so are two functions of one name that both do.  between(a, b),  below(n),  chance(p),
              angle(),  direction(),  pick(items) -> Option<&T>,  and for_entity(e) inside par_each
Scenes:       go_to(scene) -> bool,  restart() -> bool: false when the tick's one transition is taken.
              `go_to` of the running scene's type leaves it and enters it with the new value
Phase<P>:     get() -> P,  is(p) -> bool,  time() -> Seconds, how long it has run;  clock() -> PhaseClock
PhaseMut<P>:  everything Phase<P> has, and go_to(p) -> bool;  resume(p, clock) -> bool, in a scene's enter
              function: the phase carries on with that clock tick for tick, its `every` rules with it.  No
              enter function and no `lasts` function is asked.  What follows the length (`then`, `then_run`)
              happens once: when the time left runs out, in the first tick where the clock's left() was
              zero, and never where it was None

Inputs<A>:    of(player) -> &A,  iter() -> (Player, &A),  any(|a| ..)
BotInputs<A>: set(player, A),  is_bot(player) -> bool
Players:      iter(): each Player here, by value and in seat order, so the loop may change the roster;
              count(),  host() -> Player,  name(p) -> Text<24>,  character(p) -> Option<Entity>,
              status(p) -> PlayerStatus: Here | Away { since } | Bot | Gone
PlayersMut:   everything Players has, and add_bot() -> Option<Player>, None when every seat is taken;  remove_bot(bot);
              respawn(p): the scene's spawn, in place of the character they have;
              possess(p, entity) -> bool: theirs from the next rule on, false for another player's character;
              face(p, Walk::LOOK, angle): that heading reads as `angle` from this rule on, and their screen cuts to it
Joined, Placed:  what a spawn function is given.  joined.player;  placed.at: Vec2, placed.cell, placed.glyph
Spawn, Bundle:   a Bundle is a tuple of components, and what `PrefabDef::new` takes.  A Spawn is what
              `entities.spawn` takes and a spawn function returns: a prefab at a place, or a Bundle.  A game
              writes `-> impl Spawn` and never implements either

Commands<T>:  read() -> (Player, &T): each admitted command with who sent it, a player's own in the order sent
Offers<T>:    open(offer) -> OfferId;  accept() -> each Answer { player, choice, offer } admitted this tick;
              tally(offer) -> Tally<T>;  close(offer);  pending(player) -> Option<Pending<T>>, their oldest unanswered;
              answer_for(bot, offer, choice) -> bool, through the same check as a person's
Offer<T>:     Offer::to(player), ::to_each_player() (each for themselves), ::to_all() (a vote);
              .choices(array_or_iterator), or for a player's own offer .choices_with(hand_for), asked when the
              offer reaches the front of that player's queue;  then nothing, which leaves it open until it is
              answered, or .expires(tick.after(seconds(15))), or .until_phase_ends()
Asked:        what a `choices_with` function is given: asked.player, asked.offer.  It returns the choices, and
              takes any read-only parameter:  fn hand_for(asked: Asked, heroes: Query<&Hero>, random: Random) -> List<Card, 3>
Pending<T>:   an owned copy of an open offer: .offer: OfferId,  .choices() -> &[T]
Tally<T>:     count(choice) -> u32;  leading() -> Option<T>, the choice opened first on a tie;  answered() -> u32;
              waiting(): each Player still to answer;  everyone_answered() -> bool

Space2d:      touching(entity), within(point, radius), in_rect(rect), nearest(point) -> Found;
              ray(from, to), sweep_circle(from, to, radius), motion_of(entity).sweep() -> Swept;
              motion_of(entity).delta() -> Vec2;  move_shape(from, &shape, step) -> Move2d;  at_tick_start() -> Space2d
Found:        .on(layers) .except(entity) .within(r), then first(), any(), count() or iterate: each Near
Near:         entity,  position,  distance
Swept:        .on(layers) .except(entity), then first_hit() -> Option<Hit>, or hits() in order, for a shot that pierces
Hit:          entity: Option<Entity>, None for the level;  layer;  point;  normal: out of what was struck, toward
              the mover, never zero;  fraction: Num, how far along the path, by time;  mover_position: where
              the mover itself stood at first contact;  remaining_motion: from there to where its path ended;
              depth: Num, how far the mover stood inside what it struck, zero unless the path began inside it
              by more than the contact tolerance;  approach_speed: Num, units a second, how fast the two were
              closing along the normal at first contact: above zero closing, below zero parting (section 7)
Move2d:       [.up(direction)] [.except(entity)] .on(layers) -> Moved2d
Moved2d:      what `Transform2d::follow` takes, with the path the move took.
              position: Vec2;  on_floor(),  on_ceiling(),  wall() -> Option<Side>,  blocked(): what the shape rests
              against when the move ends, moved into or not;  contacts() -> &[Hit], everything it met, in order;
              floor() -> Option<Hit>, what it stands on;  slide(velocity) -> Vec2, with what was stopped taken out
Level2d:      the level's fixed collision alone.  move_shape(from, &shape, step) -> Move2d;  ray(from, to) -> Swept;
              touching(at, &shape) -> FoundTiles;  solid_at(point) -> Option<Layer>;  mark(MARK) -> Vec2, the one
              place;  marks(MARK);  bounds() -> Rect
FoundTiles:   .on(layers), then any(), or each Tile { cell, layer }
LevelMut2d:   load(&tilemap): replaces the level, with its solids, spawns, marks and tiles;
              add_tiles(&tilemap, origin): one more map beside what is loaded, such as the next screen;
              either takes .drawn_with(sheet), for a record that names its own tile sheet;
              add_rect(rect, LAYER),  add_segment(from, to, LAYER);  each add returns a LevelPiece;
              remove(piece) -> bool, true for the call that takes it out;  clear().  In an enter function, or any rule
Tilemap:      content, read by a validator and a test too.  Tilemap::from_rows(["# #", "###"]);  size(),  rows(),
              glyph_at(cell) -> char,  cells() -> (cell, glyph),  cells_with(glyph): the centre of each such cell

Content<T>:   get(id) -> &T;  find(name) -> Option<Id<T>>;  iter() -> (Id<T>, &T), in name order
Saving:       request::<T>(slot): "save now", such as at a shrine
Keep:         in a capture.  all(PREFAB) or all((BAT, ELITE_BAT)) -> Kept<N>: every entity a rule or a spawn
              function spawned from those names, disabled or not, with the components N lists.  It compiles
              only where N lists what each of those prefabs' spawns must bring
Restored<T>:  get() -> Option<&T>: the save a scene was entered with.  Some in the functions run as the scene is
              entered from that save, its enter functions and the spawn functions run then, and None from then on
```

### R4. What a frame function may take

It may also take, read-only and as [R3](#r3-what-a-rule-may-take) gives them, `Query` and `Single` (a pose as
`&Pose2d` or `&Pose3d`), `Res<T>`, `Phase<P>`, `Content<T>`, `Family`, `Players`, `Level2d` and `Tick`: the
world as shown, with `Tick` the newer tick of the pair.

```rust
Pose2d, Pose3d:  the transform as this picture shows it, between ticks and predicted, in floats.  position,
              turn or rotation, scale;  cut: bool, true in the first picture after a `teleport`;
              on a Pose3d, forward() and heading()
Events<T>:    read(): the events of every tick since this function last ran, each once, in order;  missed() -> u32
Frame:        dt() and seconds(): this screen's own clock, in f32.  And the tick's clock at the picture's own
              moment, between its two ticks, in f32 and standing through a pause:  in_scene(),  in_phase(),
              left(&timer),  fraction(&timer), each for a Timer or a Cooldown;  seconds_until(tick_number)
Screen<T>:    one value for this screen, used as its value: Camera2d, Camera3d, Sky, Backdrop, InputContext,
              a local setting, the game's own menu state
Shown<L>:     how this screen dresses a look.  shown.of(entity)  .play(clip)  .play_once(clip), whose
              .finished() says it has ended;  .flip_x(bool)  .tint(colour)  .flash(seconds)  .scale(s)
              .turn_y(angle)  .offset(vec3f(0.0, bob, 0.0))  .hide()
Ui:           the interface for this frame (R5)
Draw2d, Draw3d:  things drawn among the world's.  sprite(asset, at) .tint(c) .scale(s);  shape(shape, at);
              line(a, b) .width(w) .color(c);  marker(at);  label(at, text);  burst(effect, at);
              each takes .lasting(seconds) and a place of either kind of number
MyInput<A>:   this player's map as sampled this frame, read as its fields;  `&mut` to set a #[sent] field.
              An Actions or a LocalActions map
Pointer:      position() -> Vec2f,  captured() -> bool,  ray(&camera);
              camera.world_point(&pointer),  camera.ground_point(&pointer)
Bindings:     rebind(Controls::JUMP, input),  reset(),  prompt(Controls::JUMP)
Outbox:       send(command) -> Delivery, which is #[must_use]: Queued, it will be admitted unless the link drops;
              Closed, the match has ended
Request:      scene(Level { def }), which takes .restoring::<T>(slot);  quit()
MyOffer<T>:   open() -> Option<Pending<T>>, an owned copy, so `choose` may be called while its choices are walked;
              choose(value);  status() -> Waiting | Sending | Taken
Pause:        on(),  off(),  toggle(),  is_on() -> bool,  by() -> Option<Player>
Saves:        save::<T>(slot),  can_save::<T>() -> bool,  peek::<T>(slot) -> Loaded<&T>,
              status(slot) -> SaveStatus,  delete(slot),  slots()
Loaded<T>:    Loading | Missing | Ready(T) | Newer { version } | Failed(reason)
SaveStatus:   Idle | Writing | Written { at } | Failed(reason)
Me:           player() -> Player,  character() -> Option<Entity>: which player this screen is
Session:      state() -> Alone | Hosting { ticket, players } | Joining { progress } | Playing | Closed { reason };
              host(),  join(ticket),  leave(),  can_host(),  dropped_ticket()
```

### R5. Looks, cameras, the interface and assets

```rust
// Looks, each a component:
Sprite:       Sprite::new(art::enemies::BAT), ::named("enemies/bat")  .layer(1) .tint(colour) .glow(0.5) .looping(clip)
Shape2d:      Shape2d::circle(r), ::rect(w, h), ::triangle(size), ::line(a, b)   .color(colour)
Model:        Model::new(art::meshes::ROCK), ::cube(), ::sphere()   .material(m) .origin_at_base()
Material:     Material::rough(colour), ::metal(colour), ::textured(art::materials::GRANITE)   .glowing(colour, strength)
Light:        Light::point(colour, strength), ::spot(colour, strength, angle)
Sky, Ground, Backdrop:  Sky::clear();  Ground::flat();  Backdrop::new(art::SKYLINE).parallax(0.3)
SunAngle:     where the Sun stands, for this screen, in f32.  SunAngle::degrees(elevation, azimuth).  Through
              Screen<Sky> a frame function sets it: `sky.sun = SunAngle::degrees(25.0, 150.0)`
Color:        Color::rgb(r, g, b), and named ones such as Color::GOLD, GREY, RED, NIGHT

Camera2d:     Camera2d::fit(24.0, 18.0): at least this rectangle is in view, whatever the window;
              ::exactly(40.0, 22.5): this rectangle and no more, with bars filling the rest;
              ::fit_height(14.0): this height, and as much width as the window gives;
              .centered_on(point) | .following_my_character() | .following(entity)
              .damping(seconds(0.15))  .inside_level() | .inside(rect)
Camera3d:     Camera3d::looking_at(eye, target).fov(35.0);
              ::following_my_character().offset(0.0, 1.5, 4.0).damping(seconds(0.2));
              ::orbiting_my_character().yaw(Walk::LOOK).distance(6.5).height(1.5)
// Through Screen<Camera2d> or Screen<Camera3d>:
camera.rig()                // the rig's numbers, to change: distance, height, offset, damping, fov, follow(entity)
camera.ease_to(other, seconds(1.2))   // asked again for the same view it carries on, and does not start again
camera.cut();  camera.shake(amount);  // a shake plays out on top of the rig
camera.view() -> Rectf;  camera.screen_point(world) -> Option<Vec2f>      // None behind the camera

// src/art.rs, the asset catalog: written by `cargo play`, never by hand. The kind is in the type.
pub mod enemies { pub const BAT: SpriteAsset = SpriteAsset::new("enemies/bat"); }
pub mod hero    { pub const RUN: ClipAsset = ClipAsset::new("hero/run"); }
pub mod meshes  { pub const ROCK: MeshAsset = MeshAsset::new("meshes/rock"); }
// The kinds: SpriteAsset, ClipAsset, TilesAsset (a tile sheet), TileAsset, ImageAsset, EffectAsset, MeshAsset,
// MaterialAsset. How an image becomes the first four: the entry "Sprites, clips and tiles from source images".
```

| `Ui` | |
|---|---|
| Places | `top_left`, `top_center`, `top_right`, `left`, `center`, `right`, `bottom_left`, `bottom_center`, `bottom_right`; `ui.modal(title, ..)`, which takes `.pausing()`; `ui.at_world(point, ..)`, a place that follows a point in the world, and `.pinned_to_edge()` keeps it on screen when the point is not |
| Layout | `ui.row(..)`, `ui.column(..)`, `ui.grid(columns, ..)`, `ui.space(..)` |
| Showing | `text` (wrapped to its place), `title`, `bar(fraction)`, which takes `.mine(bool)` to mark this player's own, `ring(fraction)`, `image`, `time(seconds)`, `status(..)`, `prompt(Controls::JUMP)` (the key or button a player has bound), `toast(text, seconds(2))`, `fade(colour, amount)` over the whole screen |
| Doing | `button(label).clicked()`, `card(title, text).chosen()`, `toggle`, `stepper(label, value, range)`, `copyable(text)`; `.enabled(false)`, `.locked()`, `.key(k)` and `.pressed_by(action)` on any of them |

### R6. Tests, and errors

```rust
game().test() | .test_in(scene) -> thrum::Result<TestGame>          .test_match() -> thrum::Result<TestMatch>
game().test_setup() .in_scene(s) .with_content(name, record) .with_saved(value) .with_save(slot, save)
      .with_profile(value) .seed(n) .start() | .start_match() | .start_match_with_no_window()
      // a match takes the same fixtures, and `with_save` is what its host's slot holds
test_id::<T>("yard"):  the Id of a record a test adds with `with_content`, for a test and nowhere else;
      starting fails where no fixture and no file has that name.  `with_save` and `with_profile` take any
      type of the save's chain, so a test restores an old version
thrum::Result<T>, thrum::Error:  one error enum, each variant with a stable code;  Error::invalid(field, why)
```

| `TestGame` | |
|---|---|
| Time | `step()`, `step_ticks(n)`, `step_for(seconds)`, `step_until(limit, \|game\| ..)`, which fails with the last tick it reached and never waits on a clock; `rewind(ticks)`; `digest()`; `pause()` and `unpause()` |
| Players | `join()`, `leave(p)`, `away(p)`, `back(p)`, `add_bot()`; every input call is for the first player unless `.player(p)` says otherwise |
| Input | `hold(Controls::JUMP)`, `release(..)`, `press(..)` (down for one tick), `steer(Controls::RUN, 1.0)`, which takes a number for an `Axis`, a vector for a `Stick` and an angle for a `Heading`, `send(command)`, `choose(value)` for an open offer; `stall_input(ticks)`, which loses the player's frames as a bad link does |
| Finding | `character(player)`, `single::<F>()`, `find::<F>()`, `nearest::<F>(point)`, `children(e)`, `part(e, name)`; the `Disabled` filter finds the disabled |
| Reading | `get::<T>(e)`, `resource::<T>()`, `phase::<P>()`, `scene::<S>()`, `in_scene::<S>()`, `count::<F>()`, `alive(e)`, `disabled(e)`, `world2d(e)`, `world3d(e)`, `level2d()`, `content::<T>(id)`, the record a rule would read, `events::<E>()` since the last call, `offer::<T>(player)`, `missing_assets()` |
| Changing | `spawn(..)`, `despawn(e)`, `set(e, component)`, `set_resource(value)`, `go_to(scene)`, `go_to_phase(p)`, `run(rule)`, which takes a rule function or a closure with a rule's parameters, so a test hands it a value, and returns what it returns |
| Saves | `save::<T>()`, which runs the capture and returns the save; `restore(scene, save)`; `saved::<T>()`; `slot::<T>(slot)`, what a slot holds; `profile::<T>(player)` |
| The screen | `screen()`, or `screen_of(player)`: a `TestScreen`. Each call on it runs the scene's frame functions once, as one picture, and then acts or answers: `click("Resume")`, which fails on a control that is disabled or not there; `choose(key)`; `press(Menu::PAUSE)`; `move_pointer(dx, dy)`; `has_text(..)`; `find(key)`; `camera2d()`; `camera3d()`; `bursts()`. `advance(seconds)` runs pictures for that long on the screen's own clock, with no tick stepped, so an eased camera arrives |
| Recordings | `play_recording(file)`, which leaves the game where the run ended |

| Call | Returns | Notes |
|---|---|---|
| `join()` | `thrum::Result<Player>` | The seat is known at once. The player enters the game at the next `step()`. The error is that the game is full, with nothing changed |
| `character(player)` | `thrum::Result<Entity>` | Three errors: the join is still waiting for its tick; the player is here with no character; no such player |
| `offer::<T>(player)` | `Option<Pending<T>>` | Their oldest open offer of that type, as `Offers::pending` gives it |
| `single::<F>()` | `thrum::Result<Entity>` | An error for none and for two, with the count |
| `find::<F>()`, `children(e)` | `Vec<Entity>` | In spawn order, and in the order attached |
| `get`, `resource`, `scene`, `phase` | `thrum::Result` of a copy | An error when the entity is gone or has no `T`, nothing provides the resource, or another scene is running |
| `spawn`, `despawn`, `set`, `set_resource`, `go_to`, `go_to_phase`, `run` | `thrum::Result` | They act between ticks. `go_to` enters the scene at once, as `start` does |
| `step()` and the other time calls | `thrum::Result<()>` | The error of a refused tick, after which the game can be stepped again. A step while paused is a paused tick |
| `TestMatch::join(link)` | `thrum::Result<Peer>` | The error is that the match is full. A link the host lets through returns its peer, seated or not |
| `peer(p).my_character()` | `thrum::Result<Entity>` | `character`'s errors and two more: the peer is still loading the world; the peer was given no seat and has left |
| `peer(p).my_player()` | `thrum::Result<Player>` | Who that machine's screen is, with or without a character. The same two errors |

`TestMatch` has `host()` and `peer(p)`, each with a `TestGame`'s calls and `shown::<Pose2d>(e)` or
`shown::<Pose3d>(e)`, the pose that machine's screen shows; `join(Link::delayed(millis(120)))`,
`disconnect(p)`, `reconnect(p)`, `step_for(..)`, `digests_agree()`; and for the lobby `host().set_setup(value)`,
which sends the host's panel choice as its command, `host().start()`, which presses *Start*, so the scene
entered is the one `starts` names or `starts_with` returns, and `host().continue_saved()`, which presses
*Continue*. `start_match_with_no_window()` starts a match whose host has no seat and no screen, where
`peer(p)` of the lowest seat makes those two presses. `set_resource`, `spawn`, `set` and `run` called on
the table itself make the same change on every machine between ticks, where the same call on `host()` or a
peer changes that one world. `Link::delayed(millis(120)).with_profile(value)` gives the machine that joins
by that link a profile to bring.

## The extension map

What the core does not hold, and how each is added (O5). **The owner has said which entries the first release
builds** (O8), and each of those says so under its heading, with the work package that builds it. Nothing
else here is built, and no guide teaches it, until the owner says so. Each entry says when it comes, where it
plugs in, the names it adds, what must hold, and a few lines of the game code it allows.

**Eight extension points.** Every entry adds names at one or more of these, and changes no name or invariant
of [the core](#the-core). A game that does not use an entry compiles and pays nothing for it.

| | Extension point | What may be added there | The rule it must keep |
|---|---|---|---|
| 1 | A rule parameter | A new thing rules ask of the world or change in it | Deterministic, in the profile's numbers, digested. It joins one row of [the control table](#5-players-actions-and-control) |
| 2 | A frame parameter, or a word of one such as `Ui` | A new thing one screen does | It cannot change the world except by sending input |
| 3 | A look, with its `Shown<L>` | A new component the engine presents | Which one it is, is world state. How a screen dresses it is not |
| 4 | An action kind or a binding source | A new shape of input, or a new device | It fits the input frame and reads as a plain field |
| 5 | A line of a game, a scene or a control line | A new thing a description declares | It is part of the rule-set identity when rules can see its effect, and a function it registers that runs in a tick is one of [the checked routes](#1-a-game-one-crate-one-package-one-value) |
| 6 | A kind of admitted input | A new thing that enters the world at a tick | A recording holds it, and every machine admits the same |
| 7 | A kind of asset or content | A new file the cooker reads into the catalog or a record | Typed, checked before play, named by a stable identity |
| 8 | A command of the tool or a call of the test harness | A new way to run or prove a game | The game's code does not change |

### 3D space and a kinematic character

***In the first release** (O8, question 1), as queries and a kinematic move with no rigid bodies. Built in
WP16 with no window and in WP17 on screen. Extension points 1 and 5. Needs the engine module
[`SIM-3D`](../ARCHITECTURE.md#track-7-the-game-sdk): a deterministic 3D index, fixed collision from meshes and
a heightfield, and the move.*

Everything [section 7](#7-space-and-the-level) says holds with a 3 on it: colliders detect and never push, a
space meets the level, one `move_shape` takes the character's own collider, a transform follows a move by
the path it took, a climbed step's legs among them, a hit's normal points out of what was struck, a hit says
where the mover stood, how deep and how fast the two were closing, touching is said within the contact
tolerance, and what a rule adds to the level it can take away. **Every
name of R3's space and level groups exists with a 3 for its 2**, with `Vec3` for `Vec2` and the same
methods: `Collider3d`, `Space3d`, `Level3d`, `LevelMut3d`, `Move3d`, `Moved3d`. What differs is said here.

- **A 3D level is the same recipe**: which terrain is loaded, by its identity in content, with the layer its
  load gave it, and the pieces a rule added, in order. Its collision is cooked beside each mesh, from its
  triangles, and from the terrain's heights. It is content, so it is in the rule-set identity and a test
  builds it with no renderer, and
  [a running game keeps the pair it started with](#10-content-assets-saves-and-settings), what is drawn
  and what is stood on together. `Level3d` is on the control table's second row, so a 3D character runs
  ahead against the ground it stands on.
- **One terrain is one thing, drawn and stood on.** A `Terrain` is content: a field of a record, cooked from
  a height image and a small file beside it that gives its size and its material, as a sheet is cooked.
  `LevelMut3d::load(&terrain, LAYER)` makes it the level's ground: its collision, and the ground this scene
  draws, from the one source, as a 2D load draws the tiles it makes solid. So a region chosen at run time
  cannot stand on one terrain and show another. A test makes one with `Terrain::from_heights(rows, cell)`.
- **Every solid thing in a level is on the layer the rule that put it there named.** A terrain has no layer
  of its own, in its file or in a test: the load names it, as `add` names a model's and `add_box` a box's,
  and the recipe keeps it. A level holds one terrain, so its ground is on one layer. Every query asks
  layers, and meets the level's fixed collision on those layers and no others. So with the hills loaded on
  `TERRAIN` and the rocks added on `SOLID`, a ray on `TERRAIN` meets a hill and passes a rock, a ray on
  `SOLID` meets the rock and passes through the hill, and a line of sight that a hill and a boulder should
  both block asks `(TERRAIN, SOLID)`. A game that wants one layer for both gives both the same one.
- **What a level adds is said once.** `add(model, pose, LAYER)` is drawn as a placement and is solid on its
  layer, from the model's own triangles, and `add_box` is solid and not drawn. `place(model, pose)` is drawn
  and not solid, as a tile map's `.draws` glyph is, for the bushes a region's file lists: it is in the
  recipe, so every screen and a late joiner show it, and it costs no collision. Each returns a `LevelPiece`.
  An enter function builds a level chosen at run time from the scene's value and its content, as
  [a 2D one does](#7-space-and-the-level).
- **A contact is a floor, a wall or a ceiling**, by its normal against the move's `max_slope`, and
  `hit.surface` says which. `Moved3d` has `on_floor()`, `on_ceiling()` and `floor()` as in 2D, and `wall()`
  gives the wall's `Hit` where 2D gives a `Side`. `snap_down` holds a walking shape to a slope it walks down,
  and never pulls down one that is moving up, so a jump is the same chain.
- **`ground_height(x, z, layers)`** is the top of the fixed collision under that point on the layers asked:
  on `TERRAIN` the terrain's own height, and on `(TERRAIN, SOLID)` the top of a solid model that stands on
  it. `None` where those layers hold nothing there.
- **Marks are set by a rule**, from content: `level.set_mark(SHRINE, point)`, and `Level3d::mark` and `marks`
  read them as in 2D.

```rust
Collider3d::sphere(r) | ::capsule(r, height) | ::cuboid(x, y, z)      .offset(x, y, z)  .on(LAYER)
// Space3d:    touching, within, nearest, motion_of, at_tick_start, ray, sweep_sphere, move_shape
// Level3d:    move_shape, ray, sweep_sphere, touching, solid_at, ground_height(x, z, layers), mark, marks, bounds
// LevelMut3d: load(&terrain, LAYER), add(model, pose, LAYER), add_box(center, size, LAYER), place(model, pose),
//             set_mark(MARK, point), remove(piece), clear()
// Move3d:     [.up(direction)] [.except(entity)] .max_slope(angle) .step_up(height) .snap_down(height)
//             .on(layers) -> Moved3d
// Moved3d:    position, on_floor(), on_ceiling(), wall() -> Option<Hit>, blocked(), contacts(), floor(), slide(v)
// A Hit in 3D has what a 2D one has, in Vec3, and surface: Floor | Wall | Ceiling
// In a test:  game.level3d(), and Terrain::from_heights(rows, cell)

fn walk(at: &mut Transform3d, body: &mut Body, shape: &Collider3d, ground: Level3d, tick: Tick) {
    let moved = ground.move_shape(at.position, shape, body.velocity * tick.dt())
        .max_slope(Angle::degrees(40.0)).step_up(0.3).snap_down(0.35).on((TERRAIN, SOLID));
    at.follow(&moved);
    body.velocity = moved.slide(body.velocity);
}
// The valley was loaded with `level.load(&vale.terrain, TERRAIN)` and its boulders added on SOLID.
let in_sight = space.ray(eye, chest).on((TERRAIN, SOLID)).first_hit().is_none();   // a hill hides as a boulder does
```

### Gamepads

***In the first release** (O8, question 2). Built in WP15. Extension points 4 and 2. Pad input and rumble come
from the Mac's and the browser's shells ([ruling 3](../ARCHITECTURE.md#seam-rulings-for-game-sdk)), and
focus by pad goes through `GAME-UI`'s bindings as built.*

A pad fills the same `Axis`, `Stick` and `Button` a key does, so no rule changes. In a modal the pad moves
focus: the D-pad and the left stick are directions, south activates and east cancels, through the binding
table `GAME-UI` already takes. `ui.prompt` shows the glyph of the device the player last used. A trigger
fills a `Button` once it is past half its travel, and an `Axis` by its travel. Each player's machine reads
its own first pad. A pad is one more thing `Bindings::rebind` takes, and `Rumble` is a frame parameter: a
pulse is felt on this screen's pad and is nothing where there is none. The phone's shell is touch alone in
the first release.

**Sensitivity and inversion are each player's own**, kept with their bindings, and they change what a look
action reads from a mouse, a stick or a drag before it travels. `sensitivity` scales how far it turns.
`invert_x` turns a `Heading`, or the yaw of an [`Aim`](#camera-pitch-and-rigs), the other way.
**`invert_y` is the pitch's, so it takes an `Aim` and nothing else**: a `Heading` is one angle with no pitch,
and `invert_y` given one does not compile. WP15 builds and proves `sensitivity` and `invert_x`, on a
`Heading`, and WP17 builds `invert_y` with the `Aim` it turns.

```rust
// Pad::LEFT_STICK RIGHT_STICK (and _X _Y of each) DPAD DPAD_X DPAD_Y SOUTH EAST WEST NORTH
//      LEFT_SHOULDER RIGHT_SHOULDER LEFT_TRIGGER RIGHT_TRIGGER START SELECT
#[bind(Keys::A_D, Pad::LEFT_STICK_X, Pad::DPAD_X, Touch::stick_x())]
pub run: Axis,

fn feel_the_landing(landed: Events<Landed>, mut rumble: Rumble) {
    for hit in landed.read() { rumble.pulse(hit.speed.f32() / 20.0, 0.15); }
}
if let Some(input) = bindings.listen() { bindings.rebind(Controls::JUMP, input); }   // a remap screen
bindings.sensitivity(Walk::LOOK, 1.4);   bindings.invert_x(Walk::LOOK, true);   // a Heading, or an Aim's yaw
bindings.invert_y(Walk::LOOK, true);     // compiles where `look` is an Aim: its pitch, push up to look down
my_input.device()   // Keys, Pad or Touch: what this player last used
// In a test: `game.hold(..)`, `press(..)` and `steer(..)` set the action whatever fills it, and a TestScreen's
// `pad(Pad::SOUTH)` and `pad_stick(Pad::LEFT_STICK, x, y)` work a modal by pad.
```

### Audio

***In the first release, in this small form** (O8, question 3). Built in WP18. Extension points 2, 3 and 7.
Needs the engine module [`AUDIO`](../ARCHITECTURE.md#track-7-the-game-sdk). No agent can hear it, so an agent
proves what was played and that it reaches the mix, and how it sounds is the owner's to judge, in WP19.*

Sound is presentation. A rule cannot take `Audio`, and nothing heard changes the world. There are three forms,
and each has an owner, so nothing leaks:

- **A sound that happens** is played by a frame function on an event, as a burst is drawn. A predicted event
  is heard once.
- **`.limit(n)` bounds how many voices of one sound play at once, and never how many start.** A play asked
  for while `n` voices of that sound are sounding is dropped, and one asked for once a voice has ended
  plays. So forty hits spread through a second with a sound a second long are four voices and thirty-six
  dropped, and the same forty with a click 10 ms long are all heard, one voice at a time.
- **A sound a thing makes** is a look, `Sound`, on the entity. It starts and stops with the entity as a
  `Light` does, and `Shown<Sound>` is how one screen turns it up, down or off, said each frame as a tint is, so a hum follows
  speed and a scrape plays only while its hero slides.
- **Music is this screen's state**, `Screen<Music>`: a frame function says what should be playing, a change
  cross-fades, and saying the same track again does nothing. `scene.music(track)` sets it on entry. Volumes
  are a local setting, `Mix`, with `master`, `music` and `effects`, which the SDK registers for every game
  with sound: it is on the panel, and an options menu sets it through `Screen<Mix>`.
- **Sounds are assets.** A `.wav` or an `.ogg` under `assets/` is cooked into the catalog as a `SoundAsset`,
  `assets/sounds/kick.wav` as `art::sounds::KICK`. One whose small file beside it says `Music(..)`, with
  where it loops from, is a `MusicAsset`, which is streamed. A missing or misnamed one fails
  `cargo play --check`, as any asset does.
- **Where a sound is heard from** is this screen's camera: a sound played `.at(place)` and a thing's `Sound`
  fall off with distance from it, and pan by where they are in its view. A sound with no place is heard as
  it is.
- **A pause holds what the world makes and leaves what the screen makes.** A thing's `Sound` waits with the
  world. A menu's click and the music play on.
- **A test asks what was played**, with no device: `screen.sounds()` gives each sound a `TestScreen`'s
  pictures started, with its place and volume, as `bursts()` does for effects, and `screen.music()` the track.

```rust
.prefab(WARDEN, (Warden, model, Sound::looping(art::sounds::HUM).radius(14.0)))   // heard while it exists

fn sounds(hits: Events<Struck>, mut audio: Audio) {
    // Also .volume(x) and .pitch(x). Each call starts one voice, unless four of this sound are sounding now.
    for hit in hits.read() { audio.play(art::sounds::HIT).at(hit.at).pitch_between(0.9, 1.1).limit(4); }
}
fn score(mut music: Screen<Music>, hunt: Res<Hunt>) {
    music.track = if hunt.chasing { art::music::HUNT } else { art::music::DUSK };
}
```

### Predicted projectiles and contact

*Not in the first release (O8, question 4): first in the release after. Extension points 5 and 1. Uses
`SESSION` as built.*

This is depth 2 of [what a machine runs ahead](#11-multiplayer-an-opt-in-over-input-only-networking). Depth 1
predicts one entity, the character. Depth 2 lets a control line declare more:

- **`.predicts(PREFAB.moved_by(f).at_most(n))`**: what this player's control function launches through its
  `Launch` parameter joins that player's predicted set, up to `n` at a time. A launched thing is named by
  its launcher, the tick and its ordinal, as a predicted event is, so the bolt this screen predicted and the
  bolt the host confirms are one thing, shown once. Who launched it is the game's to keep, in a component
  of its own, as any shot's pilot is: the `Player` component stays the character's alone.
- **`f` has the control function's contract, one entity further out.** It is a function of one launched
  thing: that thing's own components by `&T` and `&mut T`, `&A` for its launcher's action map, and any
  parameter a rule may take, under
  [the same table](#5-players-actions-and-control), row for row. What it reads from the first three rows is
  run ahead. Anything from the last two keeps that player's launched things confirmed and asks for
  `.unpredicted()`, as on any control line. An event it sends is shown when it is predicted, once, so a
  bolt's spark on a wall is not a round trip late.
- **Who spawns it and how it ends.** The control function spawns it, through `Launch`, and it appears when
  that function finishes, as any spawn does. A launched thing ends in one of two ways. Its own function
  ends it, which is predicted: with `Launched::end()` for a fuse that ran out, and with
  `Launched::end_at(hit)` for a wall of the level. Or a rule despawns it at the confirmed tick, for what it
  did to the world, and the predicted one goes when that tick confirms.
- **A flight ended at a contact is judged to the moment of the contact, and no further.** `end()` takes the
  thing out as its function finishes, so no later rule sweeps it. `end_at(hit)` takes the hit of a sweep of
  the thing's own shape along this tick's step, from where it stands to where the step would have taken it:
  for a bolt with a sphere for a collider, `sweep_sphere` with the collider's radius. A ray's hit is the
  wrong one to give it. A ray's `mover_position` is the point on the pier's face, so a bolt of radius 0.25
  set there stands a quarter of a unit inside a pier whose face is at x = 5, where its swept sphere stops
  with its centre at x = 4.75. `end_at` sets the thing at the hit's `mover_position` and ends it when the
  tick does. Its last path is the one leg from where it stood to there, taken in the hit's `fraction` of
  the tick, **and the path ends at that moment**: the thing does not rest there as a stopped body does. So
  for the rules written after the control line `motion_of(bolt).sweep()` reports what the bolt met up to
  the moment it struck and nothing later: it still finds a hull the bolt passed before the pier in that
  step, finds none that stands behind the pier, and finds none that crosses the bolt's last place later in
  the tick.
- **Where it runs: in its seat's turn on the control line, whether or not its launcher is steered.** Each
  seat has a turn on the line, in seat order: the control function for that seat's character, where the
  player has one, and then `f` once for each thing launched from that seat that is still in flight, in the
  order launched. The second half never waits on the first. A thing in flight is stepped in every tick that
  is not paused: when its launcher's character has been despawned, when its launcher is away or has left,
  and in a phase the line is not `.during`. So a bolt half-way to a wall when its shooter falls goes on to
  the wall, and its fuse still runs out.
- **The `&A` it is given is its launcher's input, and no other player's.** A launched thing belongs to the
  `Player` that launched it, which is [one stay in a seat](#5-players-actions-and-control), for its whole
  flight. While that player is here, `f` reads the action map the control function reads that tick, with
  or without a character. While they are away, it reads what their character is given: idle input, or what
  `when_away` says. **Once that stay has ended**, because the player left or a person took a bot's seat,
  **`f` is given the map's idle value, `A::idle()`, in every tick**: no button held, pressed or released,
  every `Axis` and `Stick` at zero, and every `Heading` and `#[sent]` value at zero. The seat's turn
  still steps the thing, after the newcomer's control function, and the newcomer's input reaches their own
  character and what they launch, never what an earlier stay launched. The idle value is the rule set's
  constant, so the host, every peer and a replay step the thing alike. A guided shot that should hold its
  last course when its pilot goes keeps the course in a component of its own.
- **The host and the predicting machine step the same things by the same rule.** A machine goes on running
  its own player's launched things ahead while that player has no character, and a thing leaves its
  predicted set only as it ends. What a player who has left launched is stepped by every machine as
  confirmed state, because no machine is left to predict it.
- **What is predicted is what the control function launches, and nothing else.** A firing rule kept apart
  from movement, as [example E](#e-the-swarm-games-core-loop)'s `fire_at_nearest` is, stays confirmed, and
  adding `.predicts` to its game predicts no shot. To predict such a shot its few lines move into the control
  function, which takes `Launch<P>` where the rule took `Entities`. That is the whole change, and it is a
  change to the game. A rule that aims at what cannot be run ahead, as the swarm's does at the nearest of a
  crowd, is right to stay confirmed.
- **`.against(LAYERS)`** names every layer of colliders the control function's space may ask. Queries on
  those move from the table's "not run ahead" row to "run ahead with the value at the last confirmed tick".
  The level's fixed collision needs no naming, because it is run ahead already, so cover is met either way.
  A platformer's ferry is the same line: `.against(PLATFORMS)` lets a hero ride what a rule moves and keep
  its prediction.
  A query on a layer of colliders the line did not name refuses the tick on the host, so the host and the
  predicting machine never ask different questions.
- **The function leaves its own collider out with `.except(me)`**, as
  [a move does everywhere](#7-space-and-the-level), on the host and on the predicting machine alike. The
  other craft stand where the host last confirmed them, and a difference arrives as a correction, as a
  changed setting does. So a craft moves alone, and rams another, through the same line.
- **What a launched thing does to the world stays an ordinary rule at the confirmed tick**: damage, a score,
  a despawn. So no rule is written twice, and a prediction the host does not confirm is eased away and plays
  no second sound.
- It fits `SESSION`'s predicted controller as built: its state grows from one character to a declared,
  bounded set, compared whole, bit for bit. A test counts what a peer's screen shows with
  `peer(p).shown_find::<With<Bolt>>()`.

```rust
scene.players().spawn(CRAFT).control(drive)
    .predicts(BOLT.moved_by(fly_bolt).at_most(8))     // my bolts leave my muzzle at once
    .against(HULLS);                                  // and my craft meets the others where I see them

fn drive(me: Entity, pilot: &Player, at: &mut Transform3d, craft: &mut Craft, shape: &Collider3d, input: &Fly,
         mut bolts: Launch<Bolt>, space: Space3d, tick: Tick) {
    // SOLID is the level's fixed cover. HULLS is the one layer of colliders, named by `.against`.
    let moved = space.move_shape(at.position, shape, craft.velocity * tick.dt()).except(me).on((SOLID, HULLS));
    at.follow(&moved);
    if input.fire.held && craft.gun.take(tick, seconds(0.2)) {
        // The bolt starts at the craft's centre, so it keeps who fired it, to leave that hull out of its sweep.
        bolts.spawn(BOLT.with(Bolt { from: me, pilot: *pilot, velocity: at.forward() * 90.0 }).at_point(at.position));
    }
}
const BOLT_RADIUS: Num = num(0.25);                   // the BOLT prefab's collider is Collider3d::sphere(BOLT_RADIUS)

/// One bolt's whole flight, under the control function's table: its own components, the level and the tick.
fn fly_bolt(at: &mut Transform3d, bolt: &Bolt, mut me: Launched, level: Level3d, tick: Tick) {
    if me.age() > seconds(2) {
        return me.end();                              // its fuse ran out: gone now, with nothing left to judge
    }
    let next = at.position + bolt.velocity * tick.dt();
    // The bolt's own sphere, swept along this tick's step, and not a ray: the hit's `mover_position` is the
    // bolt's centre with its surface on the pier, a radius short of the face, and its `fraction` is when.
    match level.sweep_sphere(at.position, next, BOLT_RADIUS).on(SOLID).first_hit() {
        // The pier. The bolt stops at the contact on my screen at once, and its path up to that moment stays
        // for this tick's rules: a hull it passed before the pier is still hit, and one behind the pier, or
        // one that crosses where the bolt stopped later in the tick, is not.
        Some(pier) => me.end_at(pier),
        None => at.position = next,
    }
}
```

### Hits judged as the shooter saw them

*Not in the first release (O8, question 5): after predicted projectiles and contact. Extension points 6, 5,
4 and 1. Uses `SESSION` as built.*

This is depth 3. Other players are shown at the confirmed tick, a moment behind, so a shooter who aims at what
they see misses what the world holds. The fix is the field's lag compensation, as input-only networking allows
it:

- **Each player's input carries the view their screen was showing** when it was sampled: the newer tick of
  the pair it drew and how far between the two, on the wire's grid. A tick alone would not say which
  picture, because a screen draws other players part-way between two ticks. It travels with the set's other
  latest values, [in their one action](#5-players-actions-and-control), and is admitted input, so a
  recording holds it.
- **The world keeps each seat's view as how far it stands behind the present tick**, as world state. A frame
  that carries a view replaces it. A held or missing frame leaves it, so the view moves on a tick with the
  world. A player who has left has none, and what they launched is judged against the present.
- **A press carries the moment it was made.** A button marked `#[seen]` on the action map sends, with the
  set's other latest values, four things as they stood when the button went down for its newest press:
  - the view the screen was showing, which is its picture of everyone else;
  - the tick the press was sampled for;
  - **its own character as the screen was drawing it**: which entity this machine was moving as the
    player's character, and where between two ticks the screen had it, as the newer tick of the pair and
    how far between the two, on the wire's grid. This is a second pair, apart from the view. A screen
    draws its own character between the last two steps it ran ahead, as `SESSION`'s local presentation
    gives them, and draws everyone else a moment behind;
  - the set's `Heading` and `#[sent]` values, which are the player's aim.

  They stand in every later frame until the next press, as the button's count does. So a
  press whose own frame was late or lost, and which the world reads from the count on the next frame that
  arrives, as [every press is](#5-players-actions-and-control), comes with the picture, the character, the
  place and the aim the player pressed with, and not the ones their screen shows by then. A player who
  fires straight ahead and turns right before the next frame is sampled has fired straight ahead. The world
  holds all four for the tick that reads the press. Presses read in one tick share the newest one's.
- **A space remembers a layer for a stated time**, as world state, so every machine and every replay judges
  the same hit, and a late joiner is sent the memory with the world. **It remembers each cut too**: the
  tick at which a `teleport` moved a collider is marked in the memory, as it is for the picture. **And it
  remembers whose character a collider was**: with each place, the `Player` whose character the entity was
  at that tick, where it was one.
- **`space.as_seen_by(player)`** is the same space with that layer as that player's screen drew it. A
  collider stands between its places at the two remembered ticks of the view, as far along as the view
  says, in the profile's numbers, so every machine places it alike. **One whose newer tick is a cut stands
  at its new place whatever the fraction**, as [the picture showed it](#9-frame-functions-looks-and-the-interface),
  and never part-way: a hull that teleports from x = 0 to x = 10 is hit at 10 and at no place between. A
  view older than the memory is held at its edge, so a bad link cannot reach far into the past. A rule asks
  each tick, so for a bolt in flight the view moves on by itself, through a held frame and after its pilot
  has gone.
- **`space.at_press(player, Fly::RAIL)`** gives a rule what belongs to the press of that button this tick
  reads, and `None` in a tick that reads none. It is for a shot decided in the tick it is fired, and it
  answers the three questions such a shot asks:
  - **What was there to hit:** `press.seen()` is the space with its remembered layers as that press's
    picture drew them, as `as_seen_by` gives them for the present view.
  - **Who fired:** `press.character()` is the entity the press was made with, the character that player's
    machine was moving. The world takes the press's word for which entity only where its own memory has
    that entity as that player's character at a tick it remembers. Otherwise it is `None`, so a press
    cannot name another player's craft, or one from before the memory. It goes on naming that entity when
    the player has been given another character since, by `respawn` or `possess`, and when the entity has
    been despawned. So a rule can tell a press made with the craft a pilot flies now from one made with a
    craft they have lost, and which of those fires is the game's to say.
  - **Where the shot left from:** `press.place()` is where that character stood on the pressing screen,
    rebuilt by the world from its own memory, and `None` where `character()` is, or where the character is
    on no remembered layer. It is between the places the world had for the character at the two ticks of
    the press's own pair, as far along as the press says, in the profile's numbers, so every machine
    places it alike. A craft that moved from x = 0 to x = 1 and fired when its screen showed it half-way
    fires from x = 0.5, and hits a narrow target straight ahead of that. The limits are `as_seen_by`'s:
    where the newer tick is a cut the character stands at its new place, and a tick older than the memory
    is held at its edge. Where the world despawned the character before the newer tick, it stands at the
    last place the memory has for it. Where the newer tick is the tick being stepped, its place is the
    character's as the rule finds it, so a rule that judges a press is written after the control line, and
    a pair that claims a later tick is held at that one.
    **The places are the world's and never the pressing machine's.** Where that machine ran its character
    ahead rightly, they are the two places its screen drew between. Where the world corrected it, because
    a frame was held or something it read had changed, the shot leaves from where the world had the craft,
    and the difference from the screen is the correction that screen was eased through.
  - **Which way it went:** `press.aim()` is the player's action map with each `Heading` and `#[sent]` value
    as it stood at the press. The direction comes from it, and never from a transform the control function
    has turned since.

  All four are admitted input or world state, so every machine and every replay fires the same ray.

```rust
#[derive(Actions)]
pub struct Fly {
    /* .. */
    #[bind(Mouse::look_x())] pub look: Heading,
    #[bind(Mouse::LEFT)] #[seen] pub rail: Button,       // a press carries its picture, its craft, its place and its aim
}

scene.space3d(Space3d::cells(4.0).remembers(HULLS, seconds(0.25)));

/// A bolt in flight: each tick it is swept against the hulls where its pilot's screen has them now.
fn bolts_hit(bolts: Query<(Entity, &Bolt)>, space: Space3d, mut hulls: Query<&mut Hull>) {
    for (bolt, shot) in &bolts {
        let seen = space.as_seen_by(shot.pilot);
        let Some(hit) = seen.motion_of(bolt).sweep().except(shot.from).on((HULLS, SOLID)).first_hit() else { continue };
        if let Some(mut hull) = hulls.get_mut(hit.entity) { hull.hurt(36); }
    }
}

/// A shot decided at the press. It leaves from where the pilot's screen showed their craft, between two ticks,
/// and goes the way they aimed when they pressed, and is judged in the picture they pressed in, however late
/// its frame came and whichever way the craft has moved or turned since. Written after the control line.
fn rails_hit(crafts: Query<(Entity, &Player), With<Craft>>, space: Space3d, mut hulls: Query<&mut Hull>) {
    for (me, pilot) in &crafts {
        let Some(press) = space.at_press(*pilot, Fly::RAIL) else { continue };    // no press read this tick
        if press.character() != Some(me) { continue }      // pressed with a craft this pilot has since lost: no shot
        let Some(from) = press.place() else { continue };
        let reach = from + press.aim().look.direction_xz() * 60.0;
        let Some(hit) = press.seen().ray(from, reach).except(me).on((HULLS, SOLID)).first_hit() else { continue };
        if let Some(mut hull) = hulls.get_mut(hit.entity) { hull.hurt(60); }
    }
}
```

### Skeletal animation

*Out of the first release (O4, O8, question 8), and the first engine module planned after it. Extension
points 3 and 7.*

A pose is presentation. `Model` gains a rig, and `Shown<Model>` plays clips as `Shown<Sprite>` does. No rule
reads a bone: a rule that depends on a moment of an animation owns that moment as a `Timer`, as today. A part
may ride a bone, which changes where it is drawn and not where rules find it.

```rust
.prefab(PILGRIM, (Pilgrim, Model::new(art::meshes::PILGRIM).rig(art::rigs::PILGRIM)))

fn animate(pilgrims: Query<(Entity, &Body), With<Pilgrim>>, mut shown: Shown<Model>, mut audio: Audio) {
    for (entity, body) in &pilgrims {                                                 // every player's pilgrim
        let speed = body.velocity.f32().length();
        let clip = if speed > 0.1 { art::pilgrim::WALK } else { art::pilgrim::IDLE };
        let playing = shown.of(entity).play(clip).blend(0.15).speed(speed / 1.4);     // feet that do not slide
        for step in playing.marks(art::pilgrim::FOOTSTEP) { audio.play(art::sounds::STEP).at(step.at); }
    }
}
```

### Text fields, lists and more controls

*Not in the first release (O8, question 9). Extension point 2. Needs `GAME-UI`.*

Each is a word of `Ui`. What is being typed is this screen's state, and it reaches the world as a command.

```rust
fn name_entry(mut ui: Ui, mut draft: Screen<Draft>, mut outbox: Outbox) {
    ui.center(|ui| {
        ui.text_field("Name", &mut draft.name);
        ui.slider("Music", &mut draft.music, 0.0..=1.0);
        ui.picker("Difficulty", &mut draft.difficulty, Difficulty::ALL);
        ui.scroll(|ui| for line in &draft.log { ui.text(line).style(Style::small().color(Color::GREY)); });
        if ui.button("Done").clicked() { let _ = outbox.send(SetName(Text::from(&draft.name))); }
    });
}
```

### Levels from an editor's file

*After the first release (O8, question 10), with data packs. Extension point 7.*

A `Tilemap`, and in 3D a list of placements, is cooked from an editor's file where a record names one. The
scene maps the editor's names to layers, prefabs and marks as it maps glyphs today. An object's fields
reach [its spawn function](#7-space-and-the-level) through `Placed`, each checked against the file when it
is cooked. `load`, the recipe, a map's identity and a validator's read of its cells are unchanged.

```rust
// assets/content/levels/rooftops.ron:  LevelDef(name: "Rooftops", par: 42.5, tiles: File("rooftops.ldtk#Rooftops"))
scene.tiles(Tileset::new(art::ROOFS).solid("Brick", TERRAIN).spawns("Coin", COIN).spawns("Moth", moth_at).marks("Start", START));
fn moth_at(placed: Placed) -> impl Spawn { MOTH.with(Sway { home: placed.at, reach: placed.field::<Num>("reach") }).at_point(placed.at) }
// In 3D, the nodes of a glTF scene: a node named for a prefab is spawned, and the rest are placed.
level.load(&vale.placements);     // Placements, with .named("lamp", LAMP) and .solid("wall", SOLID) on the scene
```

### Saved progress for every player

***In the first release** (O8, question 11). Built in WP13 for a game played alone and in WP5 for a guest.
Extension points 5 and 6.*

[A profile](#10-content-assets-saves-and-settings) is each player's own, and the core says whose it is, when
it is written and what a replay does with it. Alone it is read before the first tick. This entry is how a
guest's reaches a match:

- **It comes in as that player's ordered commands.** Their machine reads its slot before they join and sends
  it in numbered pieces as commands, which `SESSION` admits in sequence with no tick to miss. A guest's timed
  input on a late frame is lost, so the pieces do not travel as
  [a host's restore](#10-content-assets-saves-and-settings) does. The world gathers them, and `PlayerJoined`
  is sent once the value has landed, so a rule that meets a player finds their progress.
- **A seat that is still loading leaves its frames to the pieces.** The host admits a frame's own actions
  first and that player's commands into the slots left, eight to a tick in all. In play a sampled frame
  fills one slot at most ([section 5](#5-players-actions-and-control)), so a command always has room.
  Until its value has landed, the player's machine samples no action map and sends idle frames that carry
  no action, because nothing reads the input of a seat with no character. So every slot of every tick takes
  a piece, whatever the game's action set holds, and on a live link the value always finishes landing.
- **A transfer that stalls ends by the seat's own rules.** Until the value lands the player holds their seat
  with no `PlayerJoined` and no character, as a player still loading the world does. If their link drops they
  are away for the grace, as anyone is. On their return their machine reads from the world how many pieces
  landed and sends the rest. When the grace runs out the seat is freed and the pieces are dropped.
- **It goes out with nothing sent.** Every machine holds the same world, so each keeps its own slot equal
  to what the world confirms for its own player,
  [after a baseline as after a tick](#10-content-assets-saves-and-settings). A guest who drops and returns
  within the grace is the same `Player`, so an unlock the host committed meanwhile is written when their
  baseline is installed. A bot's is the default.
- **A guest's file is the guest's word.** `#[save(validate = f)]` runs on every machine as the value lands,
  so `f` is in the `rules` module, and a value that fails it or its check lands as the default and the player
  joins. A recording holds the pieces.

```rust
Game::new("Lampwake").profile::<Progress>(Slot::new("progress"))

fn bank_light(mut progress: ResMut<PerPlayer<Progress>>, tally: Res<Tally>, players: Players) {
    for player in players.iter() { progress[player].light += tally.light_of(player); }
}
```

### Sprites, clips and tiles from source images

***In the first release, built in WP3**, because sprites and sprite animation are (O4). Extension point 7.*

A sheet is an image and a file beside it that names its frames, its clips and its tiles. `cargo play` cooks it
and writes a constant for each name into the catalog. A misspelled frame fails `cargo play --check` with the
file and the nearest name, and a saved sheet is redrawn in the running game. No run-time name changes.
`unit` is how many pixels make one world unit, and `origin` is where on a frame the entity's position falls:
`Center`, the default, `BottomCenter` or `TopLeft`.
A sheet says how a tile is drawn and never what is solid: a tile with a collision shape smaller than its
cell, such as a spike, is [a smaller entry](#smaller-entries) on the scene's `Tileset` line.

```ron
// assets/courier/courier.sheet.ron, beside courier.png
Sheet(grid: (32, 48), unit: 32, origin: BottomCenter, frames: ["idle", "stride", "pass", "reach", "jump"],
      clips: { "run": Clip(frames: ["stride", "pass", "reach", "pass"], fps: 11) })
// assets/roofs/roofs.sheet.ron: a tile that names its edges is drawn by its neighbours
Sheet(grid: (16, 16), tiles: { "brick": Tile(fill: 0, top: 1), "wire": Tile(fill: 2) })
```

```rust
shown.of(courier).play(art::courier::RUN);                    // a ClipAsset the catalog now has
Tileset::new(art::ROOFS)
    .solid('#', STREET).draws('#', art::roofs::BRICK)         // two surfaces on one layer, each with its own tile
    .one_way('=', STREET).draws('=', art::roofs::WIRE)
```

### Camera pitch and rigs

***In the first release, with 3D space** (O8, question 1): a third-person game needs both. Built in WP17.
Extension points 4 and 2.*

`Aim` is an action kind beside `Heading`: a yaw and a pitch the player's own machine keeps, with limits on the
pitch, travelling as a `Heading` does. A rig takes it.
[A rig's numbers](#9-frame-functions-looks-and-the-interface) are already a frame function's to change, and
a boom avoids the level's fixed collision on the layers it is given. A test sets one with
`game.steer(Walk::LOOK, (yaw, pitch))`, and a bot with `Aim::of(yaw, pitch)`.
[`bindings.invert_y`](#gamepads) is built here, with the pitch it turns: with it on, the same push of a
stick or a mouse pitches the `Aim` the other way, within the same limits, and its yaw is as it was.

```rust
#[bind(Mouse::look(), Pad::RIGHT_STICK)]
#[aim(pitch = -35.0..=65.0)]
pub look: Aim,                               // .yaw .pitch: Angle; .direction(): Vec3; .direction_xz()

scene.camera(Camera3d::orbiting_my_character().look(Walk::LOOK).distance(6.5).avoiding((TERRAIN, SOLID)));
fn sprint_pulls_back(mut camera: Screen<Camera3d>, me: Me, gaits: Query<&Gait>) {
    let Some(gait) = gaits.get(me.character()) else { return };            // this screen's own character
    camera.rig().distance = if gait.sprinting() { 8.0 } else { 6.5 };      // the boom still avoids walls
}
```

### Navigation and path queries

*After the first release, first among what follows 3D space. Extension points 5 and 1.*

A scene says who must find a way, and the SDK bakes the answer from the level's recipe and keeps it by the
recipe's identity, as it keeps built collision. So it is the same on every machine, follows a gate a rule
closed or [took away](#7-space-and-the-level), and is asked through the level, read-only, by a rule, a
condition and a test. Each end of a path is put on the nearest ground an agent can stand on, within the
agent's own height, so a hovering craft and a hero in mid-jump ask from where they are. `path_length(a, b)`
gives a rule how far the way is, and a scene may name more than one agent.

```rust
scene.navigation(Nav::agent(0.5, 1.8).max_slope(Angle::degrees(30.0)).on((TERRAIN, SOLID)));

fn wardens_move(mut wardens: Query<(&mut Transform3d, &Mind), With<Warden>>, ground: Level3d, tick: Tick) {
    for (mut at, mind) in &mut wardens {
        let Some(corner) = ground.next_corner(at.position, mind.goal()) else { continue };   // None: no way
        at.position += (corner - at.position).normalized_or_zero() * 3.0 * tick.dt();
    }
}
assert!(game.level3d().path(trailhead, summit).is_some());     // a test: the summit can still be reached
```

### 3D level building and texturing

*After the first release. Extension points 5 and 7. What is solid, and a terrain drawn from the source it is
stood on, are in the first release with [3D space](#3d-space-and-a-kinematic-character); this entry is the
dressing.*

More words on the lines a 3D scene already has. What is solid is in the level's recipe and belongs to the
rules. How the ground is dressed belongs to the screen, and scatter has no collision and no identity.

```rust
scene.ground(Ground::dressing()                       // over whatever terrain the level loaded
    .layer(art::materials::GRASS)
    .layer(art::materials::SCREE).steeper_than(Angle::degrees(35.0))
    .scatter(art::meshes::TUFT, 0.4));
scene.place(Model::new(art::meshes::CAIRN)).on_ground(40.0, -12.0).solid(SOLID);     // its triangles collide
scene.place(Model::cuboid(6.0, 3.0, 0.5).material(stone)).at(10.0, 1.5, 4.0).solid(SOLID);
```

### Smaller entries

Each plugs in at the extension point named, with the code it allows and what must hold.

| Entry | When | Point | The game code it allows | What must hold |
|---|---|---|---|---|
| **A host with no window** | **In the first release** (O8, question 7), built in WP5 | 8 | `cargo play --host --port 7777`, which prints the ticket to join by | No line of the game changes. It is `thrum` with its default features off. What a host's screen may do (a scene request, a shared setting, the lobby's panel, *Start* and *Continue*) falls to the lowest seat. [Its save is read by its own process before the first tick and held in the world](#11-multiplayer-an-opt-in-over-input-only-networking), so *Continue* needs no file on any player's machine |
| **Text in the world** | After the first release | 2 | `draw.text(hit.at, hit.damage).color(Color::GOLD).rising(1.5).lasting(seconds(0.6))` | Drawn as sprites in one batch, so sixty damage numbers are not sixty interface nodes. Played out by the SDK, as a burst is |
| **Two views of one component in one rule** | After the first release | 1 | `fn revive(mut wardens: Views<(Query<&Vitals>, Query<&mut Vitals>)>)`, then `wardens.first()` and `wardens.second()` | It lends one at a time, so the access check has nothing to refuse. `Query::pair_mut` gives two entities of one query already. Until then anything more is two passes over one query with a local `List` between them |
| **A tile with a shape smaller than its cell** | After the first release | 5 | `Tileset::new(art::TILES).solid('^', SPIKES).shaped(TileShape::rect(1.0, 0.4).at_bottom()).solid('/', TERRAIN).shaped(TileShape::slope(Side::Right))` | The shape is said on the glyph's own line of the scene's `Tileset`, after `.solid` or `.one_way`, beside its layer: it is what the glyph means, so it is part of the rule-set identity and never the sheet's, and `.drawn_with` changes no collision. It is built into the level's fixed collision from the recipe with every other tile. So `Level2d::move_shape`, `ray`, `touching` and `solid_at`, and a space's rays, sweeps and moves, meet the shape with no new call, and a match runs it ahead. A glyph with no `.shaped` fills its cell, as today |
| **A mode beside a scene's phase** | In the core already, with no new name | — | `scene.rules(get_up.when(fallen))`, with the mode an enum in a resource, or in a component where each character has its own, beside its `Timer` | A scene keeps one phase machine ([section 4](#4-scenes-phases-time-and-pause)), so the tick's one transition and `Tick::in_phase()` keep one meaning. A second machine in a scene is not offered: it would need a rule for which request stands that the core does not have |
| **Lobby choices for every player** | After the first release | 5 | `Lobby::standard().player_panel(pick_calling).starts_with(night_from)`, with `fn pick_calling(ui: &mut Ui, mine: &mut Calling, ..)`, taking any read-only frame parameter after the two as the host's panel does, and `fn night_from(&Setup, &PerPlayer<Calling>) -> Night` | A player's choice is world state, changed by their own command, and stays a game resource, `PerPlayer<Calling>`, as the host's setup does, so a later scene and a late joiner's spawn function read it. A test sets one with `peer(p).set_choice(value)` |
| **A lobby with the game's own rules and frame functions** | After the first release | 5 | `Game::scene(camp)`, with `fn camp(scene: &mut Scene<Lobby>)`, which adds `scene.rules(buy_boon)` and `scene.each_frame(shop)` | The lobby is an ordinary scene, so a function may describe it further. Its rules run after the lobby's own, in the order written. The standard screen keeps the centre, and the game's frame functions take the other places. Until then a game that wants a shop in its lobby builds its own with `Session` |
| **A character's own children in its control function** | After the first release, with depth 2 | 5, 1 | `scene.players().spawn(hero).control(roam).predicts_children()`, and in `roam` a `Query` asked `under(me)` | The character's children and parts join the set its player's machine runs ahead, bounded and compared whole as [depth 2](#predicted-projectiles-and-contact)'s is, and a query `under` the character moves up to the table's first row. Until then a control function that must know what its character carries reads a component a rule keeps on the character |
| **Events from inside `par_each`** | After the first release | 1 | `foes.par_each_emit(&mut tells, \|(foe, at), out\| if foe.winds_up() { out.send(WoundUp { at: at.position }) })` | Each worker keeps its own events and they are joined in the query's order, so the result is the serial loop's. Until then the loop writes a component and a later rule sends the event |
| **A timeline inside a phase** | After the first release | 5 | `scene.phase(Stage::Night).at(seconds(90), ring_of_shades).at(seconds(300), call_the_bell)` | Each runs once, in the block's place in the written order, and a phase resumed past one does not run it again. A timeline from a content file stays a rule with a cursor |
| **Space queries from a frame function** | After the first release | 2 | `fn prompt(me: Me, space: Space3d, mut ui: Ui)` | It answers from the newer published tick and costs each screen an index. Until then a rule writes what a screen needs, such as a `Focus` component |
| **Rigid bodies** | Out by the owner's scope (O4). Needs a deterministic solver | 1 | `scene.rules((apply_thrust, physics::step, crates_break))`, `Body2d::dynamic().mass(4.0)` and `Events<Contact2d>` | The step is a rule the game places in its written order. Colliders stay detect-only on an entity with no body. In a match bodies are confirmed state, as a crowd is |
| **Data packs** | The year-one commitment (D4) | 5, 7 | `Game::new("Swarm").packs("packs")`, and in a rule `entities.spawn(kind.base.with(Health(kind.health)).at_point(p))` | A pack adds records without a recompile, and declares no prefab name: every name is still made in a `prefabs!` block. A new kind of enemy is a record that names a declared prefab as its base with the values that differ, which the game's rule applies as it spawns. The IDs and prefabs a pack names are checked as it is loaded, and the packs are part of the rule-set identity. New mechanics stay Rust |
| **Water, clouds and weather** | When `WATERS` and `CLOUDS` are built | 5, 1 | `scene.water(Water::sea())`, and in a rule `level.water_height(x, z, tick)` | What one screen shows of it is `Screen<T>`. Where play depends on it, a rule asks the level |
| **Host migration** | After the first release. Needs `SESSION` | 5 | `Multiplayer::players(1..=4).host_migration(true)` | Every machine already holds the whole world and steps it from input, so another takes over as host at a tick. No rule changes |

## Worked examples

These are real games against the proposed API, and the evidence that [the core](#the-core) reads as a game.
[What a builder may change](#what-a-builder-may-change) says how far a builder may depart from them. Every call
they make has its name in [the reference](#the-reference). Example G uses two entries of
[the extension map](#the-extension-map) that the first release builds, 3D space and gamepads, and says where.

### A. A first 2D game

Rockfall: steer a ship under falling rocks. The first page of the guide ends with a program of about 30
lines: `main`, a `play` scene of two lines (its camera and its players' line), and a `rules` block that holds
the `Pilot` map, `ship` and `steer_ship`, exactly as they stand below. The ship already answers the keys and a
drag on a phone, a second player gets a ship of their own beside it, and `cargo play --record` already
records a run that replays in Chrome. The second page adds the rocks, the score and the end. This is the
whole game, still one file:

```rust
use rules::*;
use thrum::prelude::*;

fn main() -> thrum::Result {
    game().run()
}

/// One value that `main` runs and a test steps.
pub fn game() -> Game {
    Game::new("Rockfall")
        .prefab(ROCK, (Shape2d::circle(0.6).color(Color::GREY), Collider2d::circle(0.6).on(ROCKS)))
        .scene(play)
        .scene(game_over)
}

fn play(scene: &mut Scene<Play>) {
    scene.background(Color::NIGHT);
    scene.camera(Camera2d::fit(24.0, 18.0));
    scene.resource(Score::default()); // a scene's resource starts afresh each time the scene is entered
    scene.players().spawn(ship).control(steer_ship);
    scene.rules((drop_rocks.every(seconds(0.4)), fall, crash));
    scene.each_frame(show_score);
}

/// What the world is and how it changes. Everything that changes the world lives in this block, and the
/// check holds it to numbers that agree on every device, so the game records, replays and goes online as it
/// stands. When the game grows, the block moves to `src/rules.rs` as it is.
mod rules {
    use thrum::prelude::*;

    pub const ROCKS: Layer = Layer::new("rocks");

    prefabs! {
        pub const ROCK: Prefab<Rock> = "rock"; // whoever spawns a rock says how fast it falls
    }

    #[derive(Scene, Default)]
    pub struct Play;

    /// A scene's fields are what it is entered with, so the score travels to the result.
    #[derive(Scene)]
    pub struct GameOver {
        pub dodged: u32,
    }

    #[derive(Actions)]
    pub struct Pilot {
        #[bind(Keys::A_D, Keys::LEFT_RIGHT, Touch::drag_x())]
        pub steer: Axis,
    }

    #[derive(Component)]
    pub struct Rock {
        pub fall_speed: Num,
    }

    #[derive(Resource, Default)]
    pub struct Score {
        pub dodged: u32,
    }

    pub fn ship(joined: Joined) -> impl Spawn {
        let x = joined.player.index() as i32 * 2;
        (Shape2d::triangle(1.0), Transform2d::at(x, -7), Collider2d::circle(0.5))
    }

    pub fn steer_ship(at: &mut Transform2d, pilot: &Pilot, tick: Tick) {
        at.position.x = (at.position.x + pilot.steer * 9.0 * tick.dt()).clamp(-11.5, 11.5);
    }

    pub fn drop_rocks(mut entities: Entities, mut random: Random) {
        let rock = Rock { fall_speed: random.between(4.0, 9.0) };
        entities.spawn(ROCK.with(rock).at(random.between(-11.5, 11.5), 10.0));
    }

    pub fn fall(
        mut rocks: Query<(Entity, &Rock, &mut Transform2d)>,
        mut entities: Entities,
        mut score: ResMut<Score>,
        tick: Tick,
    ) {
        for (rock, falling, mut at) in &mut rocks {
            at.position.y -= falling.fall_speed * tick.dt();
            if at.position.y < -10.0 && entities.despawn(rock) {
                score.dodged += 1;
            }
        }
    }

    /// Written after `steer_ship` and `fall`, so it tests the ships and the rocks where they are now. Every
    /// player's ship is asked, and the run ends for everyone when a rock reaches any of them.
    pub fn crash(ships: Query<Entity, With<Player>>, space: Space2d, score: Res<Score>, mut scenes: Scenes) {
        for ship in &ships {
            if space.touching(ship).on(ROCKS).any() {
                scenes.go_to(GameOver { dodged: score.dodged }); // the first request stands; a second is refused
            }
        }
    }
}

fn show_score(mut ui: Ui, score: Res<Score>) {
    ui.top_left(|ui| ui.text(format!("Dodged {}", score.dodged)));
}

fn game_over(scene: &mut Scene<GameOver>) {
    scene.background(Color::NIGHT);
    scene.accepts::<Play>(); // this scene's screen may ask for a new run
    scene.each_frame(show_result);
}

fn show_result(mut ui: Ui, over: Res<GameOver>, mut request: Request) {
    ui.center(|ui| {
        ui.title("Game over");
        ui.text(format!("You dodged {} rocks.", over.dodged));
        if ui.button("Play again").clicked() {
            request.scene(Play); // Enter, a click and a tap all press the button
        }
    });
}

#[test]
fn a_rock_that_reaches_the_ship_ends_the_run() -> thrum::Result {
    let mut game = game().test()?; // every startup check runs here, and a failed one is this error
    let ship = game.character(Player::FIRST)?;
    let at = game.get::<Transform2d>(ship)?.position;
    game.spawn(ROCK.with(Rock { fall_speed: num(6.0) }).at(at.x, at.y + 3.0))?;

    game.step_until(seconds(2), |game| Ok(game.in_scene::<GameOver>()))?;

    assert_eq!(game.scene::<GameOver>()?.dodged, 0);
    Ok(())
}
```

What the reader did not write: a window, a device, a clock, a schedule, component registration, interpolation
between ticks, sprite batching, a web page, or a line for Chrome. `cargo play` opens it on the Mac and
`cargo play --web` serves it to Chrome. It needs no art: shapes draw with no asset. `game_over` has no camera:
a scene without one shows its background and its interface and draws nothing else.

The one piece of structure is the `rules` block, and it is the SDK's first sentence written down: rules
change the world, and everything outside the block only shows it. A rule left outside it is named at startup
with the step to take, so a first game cannot run unchecked by accident.

### B. A first 3D scene

A ball you roll round a cooked, textured rock, on lit ground under the physical sky, with Sun sliders. The
first page of the 3D guide ends with it, so the first thing a reader does in 3D is steer. The rock turns, and
the example teaches the one question a maker asks of every value: can it affect play?

```rust
use rules::*;
use thrum::prelude::*;

fn main() -> thrum::Result {
    Game::new("Thrum starter").local_settings::<Sun>().scene(clearing).run()
}

/// Where the Sun stands is this screen's own, so these are local settings and may be plain `f32`.
#[derive(Settings)]
struct Sun {
    #[slider(-8.0..=60.0, default = 25.0, step = 0.5, unit = "°", label = "Sun elevation")]
    elevation: f32,
    #[slider(0.0..=360.0, default = 150.0, step = 1.0, unit = "°", label = "Sun azimuth")]
    azimuth: f32,
}

fn clearing(scene: &mut Scene) {
    scene.sky(Sky::clear());
    scene.ground(Ground::flat());
    scene.camera(Camera3d::following_my_character().offset(0.0, 2.0, 6.0).damping(seconds(0.2)));
    scene.on_enter(place_rock);
    scene.players().spawn(ball).control(roll);
    scene.each_frame((move_sun, turn_rock));
}

/// What the world is and how it changes, checked to agree on every device.
mod rules {
    use crate::art; // the asset catalog, which the check builds beside this block
    use thrum::prelude::*;

    #[derive(Actions)]
    pub struct Drive {
        #[bind(Keys::WASD, Keys::ARROWS, Touch::stick())]
        pub go: Stick,
    }

    #[derive(Component)]
    pub struct Rock;

    pub fn place_rock(mut entities: Entities) {
        let granite = Material::textured(art::materials::GRANITE);
        entities.spawn((Rock, Model::new(art::meshes::ROCK).material(granite), Transform3d::at(0.0, 0.0, 0.0)));
    }

    /// One ball for each player. A built-in sphere needs no asset.
    pub fn ball(joined: Joined) -> impl Spawn {
        let x = joined.player.index() as i32 * 2;
        (Model::sphere().material(Material::metal(Color::GOLD)), Transform3d::at(x, 0.5, 4))
    }

    /// Runs once a tick for each player's ball, with that player's input. A stick pushed up is forward, which is −z.
    pub fn roll(at: &mut Transform3d, drive: &Drive, tick: Tick) {
        at.position += vec3(drive.go.x, 0.0, -drive.go.y) * 4.0 * tick.dt();
    }
}

fn move_sun(mut sky: Screen<Sky>, sun: Screen<Sun>) {
    sky.sun = SunAngle::degrees(sun.elevation, sun.azimuth);
}

/// The turn is for show: nothing in the game depends on it. So it belongs to this screen, is worked out for
/// each picture, and uses ordinary floats.
fn turn_rock(rock: Single<Entity, With<Rock>>, mut shown: Shown<Model>, frame: Frame) {
    shown.of(*rock).turn_y(Angle::turns(0.1 * frame.seconds()));
}
```

When the turn matters to play, because it aims a light a guard looks along, it is world state, a rule writes
it, and it uses the numbers that agree on every device:

```rust
#[derive(Component)]
struct Turns {
    per_second: Angle,
}

fn turn(mut things: Query<(&Turns, &mut Transform3d)>, tick: Tick) {
    for (turns, mut at) in &mut things {
        at.turn_y(turns.per_second * tick.dt());
    }
}
```

The sky, the Sun's light and shadows, exposure, tone mapping, reconstruction and the governor are the engine's
defaults. The two sliders appear in the Mac panel and on the Chrome page, a parameter file sets them, and the
profiler is a toggle beside them. Either way the rock turns smoothly between ticks and its shadow turns with
it. The ball is moved by a control function that writes its place and nothing else, so on the guide's first
page it rolls through the rock, and the guide says so. The next page gives the scene
[3D space](#3d-space-and-a-kinematic-character), and the rock stops the ball (WP17). `art` is
[the typed asset catalog](#10-content-assets-saves-and-settings), which `cargo play` writes from `assets/`.

The starter's rules are in its `rules` block from the first line, as the `3d` template's are in
`src/rules.rs`: `Drive`, `Rock`, `place_rock`, `ball` and `roll`. So with `.multiplayer(..)` and the standard lobby on the
game, as [example F](#f-the-same-game-online-with-bots) adds them, a friend's ball rolls beside yours and each
of you sees your own answer at once (O6): `roll` reads only what a match runs ahead. The guide's online page
gives the ball a hop, so what is run ahead leaves the ground plane and tilts, and WP5 and WP14 prove a 3D
match on it:

```rust
#[derive(Component, Default)]
struct Ball {
    rise: Num, // its speed upward
}

const GRAVITY: Num = num(18.0);

/// `hop: Button` is one more field of `Drive`, and `ball` spawns a `Ball::default()` with the model.
fn roll(at: &mut Transform3d, ball: &mut Ball, drive: &Drive, tick: Tick) {
    if drive.hop.pressed && at.position.y <= 0.5 {
        ball.rise = num(6.0);
    }
    ball.rise -= GRAVITY * tick.dt();
    let velocity = vec3(drive.go.x * 4.0, ball.rise, -drive.go.y * 4.0);
    at.position += velocity * tick.dt();
    if at.position.y <= 0.5 {
        at.position.y = num(0.5); // on the ground again
        ball.rise = num(0.0);
    } else {
        at.face(velocity); // in the air it noses up as it rises and down as it falls
    }
}
```

### C. A slice of the V1 showcase

The exit scene: the cooked valley, 10,000 entities of which 1,500 drift, six stops and the hour of the day.

```rust
fn valley(scene: &mut Scene) {
    scene.sky(Sky::clear());
    scene.place(Model::new(art::meshes::VALLEY).material(Material::textured(art::materials::GROUND)));
    scene.camera(STOPS[0].camera());
    scene.resource(Tour::default());
    scene.on_enter(populate); // 10,000 spawns from `layout::PROPS`, 1,500 of them `.with(Drifts { .. })`
    scene.rules((drift, tour)); // `tour` moves on a stop when any player presses `next_stop`
    scene.each_frame((sun_follows_hour, fly_to_stop)); // the hour is a local setting, as example B's Sun is
}

fn drift(mut movers: Query<(&Drifts, &mut Transform3d)>, tick: Tick) {
    movers.par_each(|(drifts, mut at)| {
        let turn = drifts.starts_at + drifts.per_second * tick.seconds();
        at.position = drifts.around + turn.direction_xz() * drifts.radius;
    });
}

fn fly_to_stop(mut camera: Screen<Camera3d>, tour: Res<Tour>) {
    camera.ease_to(STOPS[tour.stop as usize].camera(), seconds(1.2));
}
```

`STOPS` and `layout` are the game's own data, and `layout::PROPS` is a table of prefabs. The valley is placed,
not spawned: it has no gameplay identity, so the scene describes it once and the engine draws it by its
cheapest path. A prop is the same `Model` and material on an entity. The 8,500 that no rule writes cost nothing
after they are placed, and `par_each` spreads the movers over the engine's worker threads. GPU culling, cooked
mesh rows, the recipe for this Mac and the Chrome build are not in the game's code at all.

### D. A precision platformer

Skylark: run, jump with coyote time and a buffered jump, die on spikes, restart at the flag you last touched.
Levels are content: adding one is a file. `src/rules.rs` is the game, and it is [checked](#8-numbers) to give
the same run on every device.

```rust
// src/rules.rs
use thrum::prelude::*;

#[derive(Actions)]
pub struct Controls {
    #[bind(Keys::A_D, Keys::LEFT_RIGHT, Touch::stick_x())]
    pub run: Axis,
    #[bind(Key::Space, Touch::button("Jump"))]
    pub jump: Button,
}

/// "Play the next level" is a value: `scenes.go_to(Level { def: next })`.
#[derive(Scene)]
pub struct Level {
    pub def: Id<LevelDef>,
}

#[derive(Scene)]
pub struct Cleared {
    pub level: Id<LevelDef>,
    pub time: Seconds,
}

/// One file under `assets/content/levels/` for each level.
#[derive(Content)]
pub struct LevelDef {
    pub name: String,
    pub tiles: Tilemap,
    pub par: Seconds,
    pub next: Option<Id<LevelDef>>, // checked when the game starts: it names a level that exists
}

#[derive(Component, Default)]
pub struct Hero {
    pub velocity: Vec2,
    pub on_floor: bool,
    pub coyote: Timer,        // still allowed to jump, just after leaving the ground
    pub buffered_jump: Timer, // a jump pressed just before landing
    pub checkpoint: Vec2,
    pub dying: Timer,         // each hero's own, so one hero's fall never stops another's run
}

/// The whole match plays with one feel: the host's panel sets it and the rules read it.
#[derive(Settings)]
pub struct Feel {
    #[slider(4.0..=14.0, default = 9.0, label = "Run speed")]
    pub run_speed: Num,
    #[slider(8.0..=24.0, default = 16.0, label = "Jump speed")]
    pub jump_speed: Num,
    #[slider(20.0..=80.0, default = 48.0, label = "Gravity")]
    pub gravity: Num,
}

#[derive(Event)]
pub struct Jumped {
    pub at: Vec2,
}

pub const TERRAIN: Layer = Layer::new("terrain");
pub const SPIKES: Layer = Layer::new("spikes");
pub const COINS: Layer = Layer::new("coins");
pub const START: Mark = Mark::new("start");
pub const FLAG: Mark = Mark::new("flag");

prefabs! {
    pub const HERO: Prefab = "hero";
    pub const COIN: Prefab = "coin";
}

// A level the code names. Startup checks it against the files, as it checks a prefab's name.
ids! {
    pub const ROOFTOPS: Id<LevelDef> = "rooftops";
}

pub fn prefabs(game: Game) -> Game {
    let body = Collider2d::rect(0.8, 1.4); // the hero's shape, written once: its move is given this collider
    game.prefab(HERO, (Hero::default(), Sprite::new(art::hero::SHEET).layer(2), body))
        .prefab(COIN, (Sprite::new(art::pickups::COIN), Collider2d::circle(0.3).on(COINS)))
}

pub fn level(scene: &mut Scene<Level>) {
    scene.tiles(
        Tileset::new(art::TILES)
            .solid('#', TERRAIN)
            .solid('^', SPIKES)
            .spawns('o', COIN)
            .marks('@', START)
            .marks('F', FLAG),
    );
    scene.on_enter(build_level);
    scene.players().spawn(hero_at_start).control(run_and_jump);
    scene.rules((collect_coins, touch_flags, reach_the_end));
}

fn build_level(level: Res<Level>, levels: Content<LevelDef>, mut tiles: LevelMut2d) {
    tiles.load(&levels.get(level.def).tiles); // the solid tiles, the coins and the marks, from the file
}

fn hero_at_start(tiles: Level2d) -> impl Spawn {
    let start = tiles.mark(START);
    HERO.with(Hero { checkpoint: start, ..Hero::default() }).at_point(start)
}

/// The hero's whole movement, its death included. Everything it reads is something a match can run ahead
/// with: the hero, its player's controls, the tick, the shared feel and the level's fixed collision. Dying is
/// the hero's own `Timer`, not a phase of the scene, so with a second player this function does not change.
fn run_and_jump(
    hero: &mut Hero,
    at: &mut Transform2d,
    body: &Collider2d,
    controls: &Controls,
    feel: Res<Feel>,
    tiles: Level2d,
    mut jumped: Emit<Jumped>,
    tick: Tick,
) {
    if hero.dying.running(tick) {
        return; // 0.6 seconds in which this hero is not steered
    }
    if hero.dying.finished(tick) {
        hero.dying.clear();
        hero.velocity = Vec2::ZERO;
        at.teleport(hero.checkpoint); // a cut: no picture shows the hero part-way between the two places
    }

    hero.velocity.x = controls.run * feel.run_speed;
    hero.velocity.y -= feel.gravity * tick.dt();

    if controls.jump.pressed {
        hero.buffered_jump.start(tick, seconds(0.1));
    }
    if hero.buffered_jump.running(tick) && hero.coyote.running(tick) {
        hero.velocity.y = feel.jump_speed;
        hero.buffered_jump.clear();
        hero.coyote.clear();
        jumped.send(Jumped { at: at.position });
    }
    if controls.jump.released && hero.velocity.y > 0.0 {
        hero.velocity.y *= 0.5; // let go early for a short hop
    }

    let moved = tiles.move_shape(at.position, body, hero.velocity * tick.dt()).on(TERRAIN);
    at.follow(&moved); // where the move ended, by the path it took: a slide is two legs, not one straight line
    hero.velocity = moved.slide(hero.velocity); // what a wall, a floor or a ceiling stopped is gone
    hero.on_floor = moved.on_floor();
    if hero.on_floor {
        hero.coyote.start(tick, seconds(0.1)); // six ticks
    }
    if tiles.touching(at.position, body).on(SPIKES).any() {
        hero.dying.start(tick, seconds(0.6));
    }
}

/// Written after the heroes' line, so a hero who touched spikes in the tick they reached the end has died.
fn reach_the_end(
    heroes: Query<(&Hero, &Transform2d)>,
    level: Res<Level>,
    tiles: Level2d,
    mut scenes: Scenes,
    tick: Tick,
) {
    let end = tiles.bounds().max.x - 1.0;
    if heroes.iter().any(|(hero, at)| !hero.dying.running(tick) && at.position.x > end) {
        scenes.go_to(Cleared { level: level.def, time: tick.in_scene() });
    }
}
```

`Run` is a scene resource with the coin count, registered in `level` in the full game. `Cleared`'s enter
function writes the time into `Records` when it beats the best, so a best time is recorded only by a tick whose
one transition was the clear. Nothing here asks which hero or how many: every rule, and every frame function
below, walks the heroes there are, so hosting this game with a friend changes no line (K3). A level is a file:

```ron
// assets/content/levels/rooftops.ron
LevelDef(
    name: "Rooftops",
    par: 42.5,
    next: Some("foundry"),
    tiles: [
        "                          F      ",
        "        o o       #####  ###     ",
        " @    #####   ^^             o   ",
        "########################   ######",
    ],
)
```

What this screen shows is another file. Nothing in it can change a run:

```rust
// src/screens.rs
use crate::rules::*;
use thrum::prelude::*;

/// Keys only this screen cares about. They never travel to the rules or to another player.
#[derive(LocalActions)]
pub struct Menu {
    #[bind(Key::Escape, Touch::button("II"))]
    pub pause: Button,
}

pub fn level(scene: &mut Scene<Level>) {
    scene.camera(Camera2d::fit_height(14.0).following_my_character().damping(seconds(0.15)).inside_level());
    scene.backdrop(Backdrop::new(art::SKYLINE).parallax(0.3));
    scene.accepts::<Level>(); // the pause menu may restart the level
    scene.each_frame((animate_hero, dust_on_jump, show_clock, pause_menu));
}

/// A frame function reads the tick, so the clock on screen is the run's own time, and it stands in a pause.
fn show_clock(mut ui: Ui, tick: Tick) {
    ui.top_right(|ui| ui.time(tick.in_scene()));
}

/// Which clip plays is this screen's choice, so it is made here and not kept in the world. Every hero is
/// dressed from its own state, so with two players one hero falls while the other runs on.
fn animate_hero(heroes: Query<(Entity, &Hero)>, mut shown: Shown<Sprite>, tick: Tick) {
    for (entity, hero) in &heroes {
        let clip = if hero.dying.running(tick) {
            art::hero::FALL
        } else if !hero.on_floor {
            art::hero::JUMP
        } else if hero.velocity.x != 0.0 {
            art::hero::RUN
        } else {
            art::hero::IDLE
        };
        shown.of(entity).play(clip).flip_x(hero.velocity.x < 0.0);
    }
}

fn dust_on_jump(jumped: Events<Jumped>, mut draw: Draw2d) {
    for jump in jumped.read() {
        draw.burst(art::effects::DUST, jump.at);
    }
}

fn pause_menu(mut ui: Ui, mut pause: Pause, menu: MyInput<Menu>, level: Res<Level>, mut request: Request) {
    if menu.pause.pressed {
        pause.toggle(); // while paused no rule runs and the game's clock stands, so every timer waits, `dying` too
    }
    if !pause.is_on() {
        return;
    }
    ui.modal("Paused", |ui| {
        if ui.button("Resume").clicked() {
            pause.off();
        }
        if ui.button("Restart level").clicked() {
            request.scene(Level { def: level.def }); // admitted under the pause, and the change of scene ends it
        }
    });
}
```

`src/lib.rs` holds `game()`: `Game::new("Skylark")` with `.settings::<rules::Feel>()`,
`.content::<rules::LevelDef>("levels")`, `.saved::<rules::Records>(Slot::new("records"))` for best times,
`.add(rules::prefabs)` and three scenes, each a pair such as `.scene((rules::level, screens::level))`. `main`
runs it. `collect_coins` and `touch_flags` are a few lines each, as example E's `collect_gems` is.

The feel of a platformer is proved without a window, and a whole level stays beatable:

```rust
#[test]
fn a_jump_just_after_the_ledge_still_jumps() -> thrum::Result {
    let (mut game, hero) = hero_just_off_the_ledge()?;
    game.step_ticks(2)?; // in the air, inside the six ticks of coyote time
    game.press(Controls::JUMP);
    game.step()?;

    assert!(game.get::<Hero>(hero)?.velocity.y > 0.0);
    Ok(())
}

/// Without this one, a hero who could jump from anywhere in the air would pass the test above.
#[test]
fn a_jump_after_the_coyote_time_does_not_jump() -> thrum::Result {
    let (mut game, hero) = hero_just_off_the_ledge()?;
    game.step_ticks(8)?; // in the air, past the six ticks
    game.press(Controls::JUMP);
    game.step()?;

    let hero = game.get::<Hero>(hero)?;
    assert!(!hero.on_floor && hero.velocity.y < 0.0); // still falling
    Ok(())
}

/// The hero has run off the first roof and this is its first tick in the air.
fn hero_just_off_the_ledge() -> thrum::Result<(TestGame, Entity)> {
    let mut game = game().test_in(Level { def: ROOFTOPS })?;
    let hero = game.character(Player::FIRST)?;
    game.set(hero, Transform2d::at(5.5, 2.7))?; // the last tile of the first roof
    game.steer(Controls::RUN, 1.0);
    game.step_until(seconds(1), |game| Ok(!game.get::<Hero>(hero)?.on_floor))?;
    Ok((game, hero))
}

/// Recorded once with `cargo play --record tests/runs/rooftops.run`. It fails when a change to the jump or
/// to the level means the recorded run no longer reaches the end.
#[test]
fn rooftops_is_still_beatable() -> thrum::Result {
    let mut game = game().test()?;
    game.play_recording("tests/runs/rooftops.run")?;
    assert!(game.in_scene::<Cleared>());
    Ok(())
}
```

### E. The swarm game's core loop

A hero for each player holds out against a growing crowd, with attacks that fire by themselves and a choice of
upgrade between waves.

```rust
// src/rules.rs: the run. It gives the same result on every player's machine.
use thrum::prelude::*;

#[derive(Actions)]
pub struct Steer {
    #[bind(Keys::WASD, Keys::ARROWS, Touch::stick())]
    pub walk: Stick,
}

pub const HEROES: Layer = Layer::new("heroes");
pub const ENEMIES: Layer = Layer::new("enemies");
pub const GEMS: Layer = Layer::new("gems");

// Names are declared together, so a name with no definition is found when the game starts.
prefabs! {
    pub const HERO: Prefab = "hero";
    pub const BAT: Prefab = "bat";
    pub const ELITE_BAT: Prefab = "elite bat";
    pub const GEM: Prefab = "gem";
    pub const SHOT: Prefab<(Shot, Velocity)> = "shot"; // the hero that fires it gives these
}

#[derive(Scene, Default)]
pub struct Arena;

#[derive(Scene)]
pub struct Ended {
    pub won: bool,
    pub wave: u32,
}

#[derive(Phase)]
pub enum Stage {
    Gathering,
    Wave,
    Intermission,
}

#[derive(Component)]
pub struct Hero {
    pub fire: Cooldown,
    pub fire_every: Seconds,
    pub damage: i32,
}
#[derive(Component)]
pub struct Enemy;
#[derive(Component)]
pub struct Shot {
    pub damage: i32,
    pub fades: Timer,
}
#[derive(Component)]
pub struct Gem;
#[derive(Component)]
pub struct Velocity(pub Vec2);
#[derive(Component)]
pub struct Health(pub i32);

impl Health {
    pub fn hurt(&mut self, by: i32) {
        self.0 = (self.0 - by).max(0);
    }
    pub fn is_spent(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Resource, Default)]
pub struct Run {
    pub wave: u32,
    pub gems: u32,
}

#[derive(Event)]
pub struct EnemyDied {
    pub at: Vec2,
}

#[derive(Data)]
pub enum Upgrade {
    FasterFire,
    MoreDamage,
    Heal,
}

pub fn prefabs(game: Game) -> Game {
    let hero = Hero { fire: Cooldown::READY, fire_every: seconds(0.4), damage: 1 };
    let knight = Sprite::new(art::heroes::KNIGHT).layer(2);
    let bat = Sprite::new(art::enemies::BAT).layer(1);
    game.prefab(HERO, (hero, Health(100), knight, Collider2d::circle(0.5).on(HEROES)))
        .prefab(BAT, (Enemy, Health(3), Velocity(Vec2::ZERO), bat, Collider2d::circle(0.35).on(ENEMIES)))
        .prefab(ELITE_BAT, BAT.variant().with(Health(8)).tweak(|sprite: &mut Sprite| sprite.tint = Color::GOLD))
        .prefab(GEM, (Gem, Sprite::new(art::pickups::GEM), Collider2d::circle(0.3).on(GEMS)))
        .prefab(SHOT, (Sprite::new(art::shots::BOLT).layer(3).glow(0.5), Collider2d::circle(0.15)))
}

/// One tick of the arena, top to bottom.
pub fn arena(scene: &mut Scene<Arena>) {
    scene.space2d(Space2d::cells(4.0)); // part of the rules, so the same for every player
    scene.resource(Run::default());
    scene.players().spawn(hero_in_line).control(move_hero);
    scene.phases(Stage::Gathering);
    scene.phase(Stage::Gathering).lasts(seconds(3)).then(Stage::Wave);
    scene.phase(Stage::Wave)
        .lasts(seconds(60))
        .then(Stage::Intermission)
        .on_enter(next_wave)
        .rules((
            spawn_enemies.every(seconds(1)),
            steer_enemies,
            fire_at_nearest,
            move_bodies,
            enemies_hurt_heroes,
            shots_hit,
            enemies_die,
            collect_gems,
            heroes_fall,
        ))
        .on_exit(clear_enemies);
    scene.phase(Stage::Intermission)
        .lasts(seconds(15))
        .then(Stage::Wave)
        .on_enter(offer_upgrades)
        .rules(take_upgrades);
}

fn hero_in_line(joined: Joined) -> impl Spawn {
    HERO.at(joined.player.index() as i32 * 3 - 4, 0)
}

/// A hero's whole movement: the hero, its player's input and the tick. In a match the local player's hero
/// runs ahead with it and answers at once.
fn move_hero(at: &mut Transform2d, steer: &Steer, tick: Tick) {
    at.position = (at.position + steer.walk * 6.0 * tick.dt()).clamp_to(ARENA);
}

/// Runs as the wave phase is entered. A request made while a transition lands is the next tick's first
/// request, so after the last wave the arena ends one tick later, and no rule of that tick can ask first.
fn next_wave(mut run: ResMut<Run>, mut scenes: Scenes) {
    if run.wave == LAST_WAVE {
        scenes.go_to(Ended { won: true, wave: run.wave });
    } else {
        run.wave += 1;
    }
}

/// Once a second while the wave lasts: a bat for each hero, times the wave's number.
fn spawn_enemies(run: Res<Run>, heroes: Query<&Transform2d, With<Hero>>, mut entities: Entities, mut random: Random) {
    for _ in 0..run.wave * heroes.count() {
        let Some(near) = random.pick(&heroes) else { return };
        let bat = if random.chance(0.1) { ELITE_BAT } else { BAT };
        entities.spawn(bat.at_point(near.position + random.direction() * 30.0));
    }
}

fn steer_enemies(mut enemies: Query<(Entity, &Transform2d, &mut Velocity), With<Enemy>>, space: Space2d) {
    enemies.par_each(|(me, at, mut velocity)| {
        // The heroes are a layer of their own, so this looks at four colliders and not at the crowd.
        let Some(hero) = space.nearest(at.position).on(HEROES).first() else { return };
        let chase = (hero.position - at.position).normalized_or_zero();
        let mut apart = Vec2::ZERO;
        for other in space.within(at.position, 1.0).on(ENEMIES).except(me) {
            let away = at.position - other.position;
            apart += away / away.length_squared().max(0.02);
        }
        velocity.0 = (chase * 2.5 + apart * 0.6).clamp_length(3.0);
    });
}

fn fire_at_nearest(mut heroes: Query<(&mut Hero, &Transform2d)>, space: Space2d, mut entities: Entities, tick: Tick) {
    for (mut hero, at) in &mut heroes {
        let Some(target) = space.nearest(at.position).on(ENEMIES).within(12.0).first() else { continue };
        let every = hero.fire_every;
        if !hero.fire.take(tick, every) {
            continue; // each hero keeps its own time, so a late joiner does not fire in step with the rest
        }
        let aim = (target.position - at.position).normalized_or_zero();
        let shot = Shot { damage: hero.damage, fades: Timer::after(tick, seconds(1.5)) };
        entities.spawn(SHOT.with((shot, Velocity(aim * 18.0))).at_point(at.position));
    }
}

// `move_bodies` adds each body's velocity to its place under one `par_each`, and `enemies_hurt_heroes`
// takes one health from a hero for each enemy that `space.touching(hero).on(ENEMIES)` finds.

fn shots_hit(
    shots: Query<(Entity, &Shot)>,
    mut enemies: Query<&mut Health, With<Enemy>>,
    space: Space2d,
    mut entities: Entities,
    tick: Tick,
) {
    for (shot, hit) in &shots {
        // The path the shot took this tick against the path each bat took, so a fast shot cannot pass
        // through a bat between two ticks. A shot fired this tick starts where it was spawned.
        if let Some(struck) = space.motion_of(shot).sweep().on(ENEMIES).first_hit() {
            if let Some(mut health) = enemies.get_mut(struck.entity) {
                health.hurt(hit.damage);
            }
            entities.despawn(shot);
        } else if hit.fades.finished(tick) {
            entities.despawn(shot);
        }
    }
}

fn enemies_die(
    enemies: Query<(Entity, &Health, &Transform2d), With<Enemy>>,
    mut entities: Entities,
    mut died: Emit<EnemyDied>,
) {
    for (enemy, health, at) in &enemies {
        if health.is_spent() && entities.despawn(enemy) {
            entities.spawn(GEM.at_point(at.position));
            died.send(EnemyDied { at: at.position });
        }
    }
}

/// The gems `enemies_die` just dropped are already here: its spawns landed when it finished.
fn collect_gems(
    heroes: Query<&Transform2d, With<Hero>>,
    space: Space2d,
    mut entities: Entities,
    mut run: ResMut<Run>,
) {
    for hero in &heroes {
        for gem in space.within(hero.position, 1.5).on(GEMS) {
            if entities.despawn(gem.entity) {
                run.gems += 1; // `despawn` is false when another hero took this gem in the same tick
            }
        }
    }
}

fn heroes_fall(heroes: Query<&Health, With<Hero>>, run: Res<Run>, mut scenes: Scenes) {
    if !heroes.is_empty() && heroes.iter().all(Health::is_spent) {
        scenes.go_to(Ended { won: false, wave: run.wave });
    }
}

fn clear_enemies(enemies: Query<Entity, With<Enemy>>, mut entities: Entities) {
    for enemy in &enemies {
        entities.despawn(enemy);
    }
}

/// Each player is asked once in each intermission. The offer is world state, so the cards come back after a
/// lost connection, and an answer that arrives after the wave has started is dropped with the offer.
fn offer_upgrades(mut offers: Offers<Upgrade>) {
    let choices = [Upgrade::FasterFire, Upgrade::MoreDamage, Upgrade::Heal];
    offers.open(Offer::to_each_player().choices(choices).until_phase_ends());
}

fn take_upgrades(mut offers: Offers<Upgrade>, mut heroes: Query<(&mut Hero, &mut Health)>) {
    for answer in offers.accept() {
        let Some((mut hero, mut health)) = heroes.of_player(answer.player) else { continue };
        match answer.choice {
            Upgrade::FasterFire => hero.fire_every = hero.fire_every * 0.85,
            Upgrade::MoreDamage => hero.damage += 1,
            Upgrade::Heal => *health = Health(100),
        }
    }
}
```

```rust
// src/screens.rs: what each player sees. Nothing in this file can change the run.
use crate::rules::*;
use thrum::prelude::*;

pub fn arena(scene: &mut Scene<Arena>) {
    scene.background(Color::rgb(0.05, 0.06, 0.08));
    scene.camera(Camera2d::fit_height(40.0).following_my_character());
    scene.each_frame((pop_where_enemies_die, status, upgrade_cards));
}

fn pop_where_enemies_die(died: Events<EnemyDied>, mut draw: Draw2d) {
    for death in died.read() {
        draw.burst(art::effects::POP, death.at);
    }
}

fn status(mut ui: Ui, run: Res<Run>, heroes: Query<(&Player, &Health), With<Hero>>, me: Me) {
    ui.top_left(|ui| {
        for (player, health) in &heroes {
            ui.bar(health.0 as f32 / 100.0).mine(*player == me.player());
        }
    });
    ui.top_center(|ui| ui.text(format!("Wave {}", run.wave)));
    ui.top_right(|ui| ui.text(format!("{} gems", run.gems)));
}

fn upgrade_cards(mut ui: Ui, mut offer: MyOffer<Upgrade>) {
    // From the world: it stays until this player's answer lands. `open` is an owned copy, so `choose` is free.
    let Some(open) = offer.open() else { return };
    ui.modal("Choose an upgrade", |ui| {
        for upgrade in open.choices() {
            if ui.card(upgrade.name(), upgrade.blurb()).key(upgrade).chosen() {
                offer.choose(upgrade);
            }
        }
        ui.status(offer.status()); // "Sending…" until the host has taken the choice
    });
}
```

These cards come between waves, so a modal suits them. A game that deals cards in the middle of a fight
describes them in `ui.bottom_center` instead: outside a modal the walk stick stays the hero's, and each card
takes `.pressed_by(Cards::FIRST)`, a local action. Several level-ups in a second queue, oldest first.

The fight is proved with the real combat rules and no window. The test tells the two bats apart, so a weapon
that picked the first bat spawned would fail it:

```rust
#[derive(Scene, Default)]
struct Fight;

/// A scene is a function, so a test builds the one it needs from the game's own rules.
fn fight(scene: &mut Scene<Fight>) {
    scene.space2d(Space2d::cells(4.0));
    scene.players().spawn(HERO).control(move_hero);
    scene.rules((fire_at_nearest, move_bodies, shots_hit, enemies_die));
}

#[test]
fn a_hero_shoots_the_nearer_bat_first() -> thrum::Result {
    let mut game = Game::new("fight").add(prefabs).scene(fight).test()?;
    let far = game.spawn(BAT.at(-8.0, 0.0))?; // spawned first, and further away
    let near = game.spawn(BAT.at(3.0, 0.0))?;

    game.step_until(seconds(3), |game| Ok(!game.alive(near)))?;

    assert!(game.alive(far));
    assert_eq!(game.get::<Health>(far)?.0, 3);
    assert_eq!(game.count::<With<Gem>>(), 1);
    Ok(())
}
```

### F. The same game, online, with bots

No rule changes. The game says how many may play and takes the standard lobby:

```rust
pub fn game() -> Game {
    Game::new("Swarm")
        .multiplayer(Multiplayer::players(1..=4))
        .lobby(Lobby::standard().starts(Arena))
        .add(rules::prefabs)
        .scene((rules::arena, screens::arena))
        .scene((rules::ended, screens::ended))
}
```

The standard lobby offers playing alone, hosting where this device can host, and joining from a link, a paste or
a launch argument. It shows the host a link to send, shows everyone who is here and how many may come, gives
*Start* to the host alone, and after a refused or closed match says why and offers to try again or go back.
`Lobby::standard().ready_up()` adds a ready mark each player sets for themselves, and `.host_panel(pick_arena)`
gives the host a panel of the game's own, whose choices the scene it starts is entered with. The lobby is an
ordinary scene on the match's clock, so a ready mark and a host's choice travel as any input does
([how](#11-multiplayer-an-opt-in-over-input-only-networking)). A game that wants its own screens uses
[`Session`](#11-multiplayer-an-opt-in-over-input-only-networking).

What made this possible is in the rules already: each player's hero is spawned by `scene.players()`, its
movement is a control function a match can run ahead, the upgrade is an offer, and the numbers agree on every
device. A game played alone is the same game with one player.

Alone, the arena's first tick has one player and one hero. With three friends in the lobby, four heroes stand
in a row on every machine. A friend who joins while a wave is running is sent the world as it stands, and
their hero is spawned at the tick they enter.

A bot is a player whose input a rule writes. Nothing else in the game knows it is not a person: it has a hero
from the same line of the scene, the same control function moves it, and it is offered the same upgrades, which
a rule answers for it.

```rust
pub fn arena(scene: &mut Scene<Arena>) {
    // ..as above, with two lines added before the players' line:
    scene.on_enter(fill_empty_seats);
    scene.rules(bots_flee); // written before the heroes move, so this tick's hero reads this tick's input
    scene.players().spawn(hero_in_line).control(move_hero);
    // ..and one rule added to the intermission, before `take_upgrades`:
    scene.phase(Stage::Intermission).rules((bots_choose, take_upgrades));
}

/// `PlayersMut` is the roster a rule may change. A frame function is given `Players`, which can only look.
fn fill_empty_seats(mut players: PlayersMut) {
    while players.count() < 4 && players.add_bot().is_some() {}
}

/// A bot's answer goes through the same check as a person's: once, for a choice it was offered.
fn bots_choose(mut offers: Offers<Upgrade>, players: Players, mut random: Random) {
    for bot in players.iter().filter(|player| players.status(*player) == PlayerStatus::Bot) {
        let Some(open) = offers.pending(bot) else { continue };
        if let Some(choice) = random.pick(open.choices()) {
            offers.answer_for(bot, open.offer, *choice);
        }
    }
}

fn bots_flee(mut bots: BotInputs<Steer>, heroes: Query<(&Player, &Transform2d), With<Hero>>, space: Space2d) {
    for (player, at) in &heroes {
        if !bots.is_bot(*player) {
            continue;
        }
        let nearest = space.nearest(at.position).on(ENEMIES).first();
        let away = nearest.map_or(Vec2::ZERO, |bat| (at.position - bat.position).normalized_or_zero());
        bots.set(*player, Steer { walk: Stick::toward(away) });
    }
}
```

**A person is never turned away for a bot.** A friend who joins this arena while bots fill it takes the
lowest seat a bot holds: in that tick the bot leaves as any player does, with its hero and its part in an open
offer, and the friend joins with a hero of their own. Seats, bots and joins are world state and admitted input,
so the host, the friend's machine and a replay agree ([seats](#5-players-actions-and-control)).

A match is proved in one process, in simulated time, with a slow link:

```rust
#[test]
fn a_late_joiner_gets_a_hero_that_answers_at_once() -> thrum::Result {
    let mut table = game().test_match()?; // a host and its peers, each with a world of its own
    table.host().start()?; // presses *Start*, so the lobby enters the scene it `starts`
    table.step_for(seconds(5))?;

    let late = table.join(Link::delayed(millis(120)))?;
    table.step_for(seconds(1))?;
    let hero = table.peer(late).my_character()?;
    let before = table.peer(late).shown::<Pose2d>(hero)?.position.x;

    table.peer(late).steer(Steer::WALK, Stick::RIGHT);
    table.step_for(millis(50))?; // less than one trip to the host

    assert!(table.peer(late).shown::<Pose2d>(hero)?.position.x > before); // predicted on their own screen
    table.step_for(seconds(2))?;
    assert!(table.digests_agree()); // and every world is still the same world
    Ok(())
}
```

### G. A third-person 3D slice, on the extension map

Hollow Vale: walk a valley, carry a relic on your back, stay out of a warden's sight, alone or with a friend.
**This example uses entries of [the extension map](#the-extension-map) that the first release builds** (O8):
`Space3d`, `Level3d` and `move_shape` in [3D](#3d-space-and-a-kinematic-character), and
[the gamepad bindings](#gamepads). It is the 3D game the design leads to.

```rust
#[derive(Actions)]
pub struct Walk {
    #[bind(Keys::WASD, Pad::LEFT_STICK, Touch::stick())]
    pub go: Stick,
    /// Which way the player is looking. It is kept on their own machine at the frame rate, so the camera
    /// turns with no delay, and its latest value travels with each tick's input.
    #[bind(Mouse::look_x(), Pad::RIGHT_STICK_X)]
    pub look: Heading,
    #[bind(Key::E, Pad::SOUTH, Touch::button("Take"))]
    pub take: Button,
}

pub const BACK: Part = Part::new("back"); // a named place on the pilgrim, declared in its prefab

pub fn vale(scene: &mut Scene<Vale>) {
    scene.phases(Chapter::Arrival);
    scene.on_enter((load_the_valley, begin_or_resume)); // the ground and its solid models, from content; then the journey
    scene.players().spawn(PILGRIM).control(walk);
    scene.rules((take_or_set_down, wardens_look, wardens_move, wardens_catch));
}

/// The valley's ground and what stands on it, from content. Each solid thing is on the layer named here.
fn load_the_valley(vales: Content<ValeDef>, mut level: LevelMut3d) {
    let vale = vales.get(HOLLOW_VALE);
    level.load(&vale.terrain, TERRAIN); // the hills: drawn and stood on, on TERRAIN
    for boulder in &vale.boulders {
        level.add(Model::new(boulder.mesh), boulder.pose, SOLID); // drawn, and solid on SOLID
    }
}

/// "Forward" is away from the camera, and the camera's angle arrives as input. The pilgrim's own collider is
/// the shape that moves. Everything here is something a match runs ahead.
fn walk(
    at: &mut Transform3d,
    body: &mut Body,
    shape: &Collider3d,
    input: &Walk,
    tuning: Res<Tuning>,
    ground: Level3d,
    tick: Tick,
) {
    let forward = input.look.direction_xz();
    let wish = (forward * input.go.y + forward.right() * input.go.x) * tuning.walk_speed;
    body.velocity = vec3(wish.x, body.velocity.y - tuning.gravity * tick.dt(), wish.z);

    let moved = ground
        .move_shape(at.position, shape, body.velocity * tick.dt())
        .max_slope(Angle::degrees(40.0))
        .step_up(0.3)
        .on((TERRAIN, SOLID));
    at.follow(&moved);
    body.velocity = moved.slide(body.velocity);
    if wish != Vec3::ZERO {
        at.face(wish);
    }
}

fn take_or_set_down(
    pilgrims: Query<(Entity, &Player, &Transform3d), With<Pilgrim>>,
    inputs: Inputs<Walk>,
    space: Space3d,
    mut entities: Entities,
) {
    for (pilgrim, player, at) in &pilgrims {
        if !inputs.of(*player).take.pressed {
            continue;
        }
        let back = entities.part(pilgrim, BACK);
        if let Some(carried) = entities.children(back).next() {
            entities.detach(carried).keep_world(); // it stays where it is, on the ground
        } else if let Some(relic) = space.within(at.position, 1.8).on(RELICS).first() {
            entities.attach(relic.entity).to(back).keep_local(Transform3d::ORIGIN);
        }
    }
}

fn wardens_look(
    mut wardens: Query<(&Transform3d, &mut Mind), With<Warden>>,
    pilgrims: Query<&Transform3d, With<Pilgrim>>,
    space: Space3d,
    tuning: Res<Tuning>,
    tick: Tick,
) {
    for (at, mut mind) in &mut wardens {
        let eye = at.position + Vec3::UP * 1.6;
        for pilgrim in &pilgrims {
            let chest = pilgrim.position + Vec3::UP * 1.0;
            // One ray: a space meets the level too, so a hill hides a pilgrim as a boulder does.
            let hidden = space.ray(eye, chest).on((TERRAIN, SOLID)).first_hit().is_some();
            if eye.distance(chest) < tuning.sight && !hidden {
                *mind = Mind::Chasing { last_seen: pilgrim.position, gives_up: Timer::after(tick, seconds(4)) };
            }
        }
    }
}

pub fn vale_screen(scene: &mut Scene<Vale>) {
    scene.sky(Sky::clear());
    // The ground is the terrain `load_the_valley` loaded: drawn from the source the rules stand on.
    scene.camera(
        Camera3d::orbiting_my_character()
            .yaw(Walk::LOOK)
            .distance(6.5)
            .height(1.5)
            .damping(seconds(0.12))
            .avoiding((TERRAIN, SOLID)),
    );
    scene.each_frame((sun_follows_the_hour, journal, save_at_the_shrine));
}
```

A warden's state machine is an enum with timers in it (`Mind`), which rewinds and saves with the world. The
journey is saved at the shrine and resumed from the title screen, into the chapter it was saved in:

```rust
#[derive(Save)]
#[save(version = 2, from = ValeSaveV1)]
pub struct ValeSave {
    pub hour: Num,
    pub chapter: Chapter,
    pub chapter_clock: PhaseClock, // how long the chapter has run and how long it has left: `chapter.clock()`
    pub relics: Kept<(Relic, Transform3d)>, // each relic, by its prefab's stable name, where it lies
    pub wardens: Kept<(Warden, Mind, Transform3d)>, // each warden with its mind and its timers, asleep or awake
}

/// Reads the world at a committed tick and returns what to keep. It runs when a save is asked for.
pub fn capture(day: Res<Day>, chapter: Phase<Chapter>, keep: Keep) -> ValeSave {
    ValeSave {
        hour: day.hour,
        chapter: chapter.get(),
        chapter_clock: chapter.clock(),
        relics: keep.all(RELIC), // a relic on a pilgrim's back is kept where it stood in the world
        wardens: keep.all((WARDEN, NIGHT_WARDEN)),
    }
}

/// A save chosen on the title screen arrives here, as input to this tick.
fn begin_or_resume(
    restored: Restored<ValeSave>,
    mut chapter: PhaseMut<Chapter>,
    mut entities: Entities,
    mut day: ResMut<Day>,
) -> thrum::Result {
    match restored.get() {
        Some(save) => {
            // The chapter carries on with the clock it had. No chapter is entered, so its sting does not play
            // again, and its length is not asked for again, so it ends when it would have.
            chapter.resume(save.chapter, save.chapter_clock);
            day.hour = save.hour;
            entities.respawn(save); // every kept relic and warden, each from its prefab with what was kept
            Ok(())
        }
        None => new_journey(&mut entities, &mut day), // an error here refuses the tick: never half-entered
    }
}

// In the game: `.save::<ValeSave>(capture)`. The title screen shows *Continue* where
// `saves.peek::<ValeSave>(Slot::AUTO)` is `Loaded::Ready`, and asks with
// `request.scene(Vale).restoring::<ValeSave>(Slot::AUTO)`.
```

With a friend it is the same valley. The game says `.multiplayer(Multiplayer::players(1..=2))` and takes the
standard lobby, and no rule above changes (O6): `walk` reads only what a match runs ahead, so a guest's pilgrim
answers at once against the ground it stands on. [Example F's test](#f-the-same-game-online-with-bots) proves
it here with `shown::<Pose3d>` in place of `shown::<Pose2d>`.

## Research and selection

Bevy 0.20, Godot 4.6, Unity, Unreal's gameplay framework, Photon Fusion 2, macroquad, lightyear and bevy_ggrs
were each read for how a game's code is shaped, not for its features, and the owner's review of 26 November
compared the first design with seven of them line by line. Its findings are the decisions above. The choices
that give the core its shape, and what would change each:

1. **Plain-data components and rule functions, not objects with methods** (from Bevy, with its names: `Query`,
   `Res`, `With`, `Single`). Thrum's simulation is a fixed tick over registered state that is rewound,
   replayed, digested and shared by causes, and only state the engine can see whole can do that. Godot's state
   inside objects cannot. *It would change* if multiplayer and replay were dropped from the SDK's aims.
2. **One written order, with no schedule graph.** Bevy's graph finds parallelism between systems and is the
   largest source of surprises in Bevy code. Thrum's parallelism is inside a rule, where a crowd's cost is.
   *It would change* if a profile showed whole rules that could run side by side dominating a tick.
3. **What a rule did is what the next rule sees** (R2), in place of edits that land when a tick ends, which
   made a stomp, a pick-up and a wave's last clean-up each a tick late.
4. **Two kinds of function, told apart by type** (from Fusion's `FixedUpdateNetwork` and `Render`). A rule
   cannot take `Ui` and a frame function cannot take `ResMut<T>`, and each mistake is a compile error.
5. **Looks are components, and how one screen dresses them is not** (R6).
6. **One vocabulary for 2D and 3D, with the dimension in the name where the two differ.**
7. **Portable numbers in rules, floats on the screen** (D1, R4). `f32` everywhere for games played alone would
   make "turn on multiplayer" a rewrite and give up replay across devices.
8. **A scene is a type, a prefab and a layer are typed constants, and an action is a field**, so each is found
   by an editor's completion, checked by the compiler and named the same in a save, a recording and an error.
9. **The interface is described each frame and kept by the engine**, where current interface kits have settled
   (React, SwiftUI, Compose).
10. **One package, with a `rules` module checked by being built alone**, not a rule crate a first-time maker
    must learn a workspace for.
11. **A controlled character that a match runs ahead, and a confirmed world round it** (from Fusion's
    predicted controller and Factorio's latency hiding), not whole-world rollback, which `SESSION` measured at
    about 50 ms a correction for a crowd of 10,000. Unreal's separation of a player, their controller and
    their pawn gives a player who outlives their character and a bot that is a player.
12. **Only input travels** (from lightyear's and bevy_ggrs's deterministic replication), made the only way to
    write a rule and not bolted on afterwards.

## The system as it runs

### State and owners

| State | Owner | The SDK's part |
|---|---|---|
| Entities, components, resources, the tick, rewind, publication | `thrum-core` (G1, G2S) | Registers what the game's description names; adapts rule functions to registered systems; asks for the barrier after each |
| The rules profile, the digest, the 2D index, fixed collision and the character move | `SIM-2D` | `Num` and friends are its numbers; `Space2d` is its index, one for each layer; `Level2d` is its fixed collision |
| The 3D index, fixed collision from meshes and a heightfield, and the 3D character move | `SIM-3D`, a new module (O8) | `Space3d` is its index, one for each layer; `Level3d` is its fixed collision, built from the level's recipe |
| The mix, its voices and each target's sound output | `AUDIO`, a new module (O8) | `Audio`, the `Sound` look and `Screen<Music>` say what should be heard. Nothing heard is world state |
| The frame, the recipe, reconstruction, EDR, the governor | `APPLE-TILE`, `APPLE-PRESENTATION` | None. A game gets them by default |
| Sprite rows, sheets, the 2D scene | `RENDER-2D` | Maps `Sprite`, `Shape2d`, the tile layer and `Transform2d` to rows |
| Sky, Sun, air | `ATMOSPHERE` | `Sky` sets its inputs |
| Cooked products and live edits | `CONTENT-LOOP`, `COOKED-DRAW` | Resolves the catalog's names; gives an entity's `Model` its cooked mesh row and a placement to the cooked placement as built; cooks content files; in a game with 3D space, gives a running game no new mesh or terrain whose cooked collision differs from the one it started with |
| The 3D scene's entity rows and the passes that place them | `thrum-render` | Maps an entity's `Model`, its material and `Transform3d` to a row |
| Typed controls, parameter files, the profiler | `CREATOR` | `Settings` declares controls |
| The interface tree, events, fonts | `GAME-UI` | `Ui` keeps the tree |
| The device, recovery, the game contract | `HOST` | Implements the game contract once, for every game |
| The match, the roster, prediction | `SESSION`, over `TRANSPORT` | Implements the session's world once, and gives a control function to the predicted controller as its step |
| Which scene and phase run, the tick each was entered at and the transition asked for; the seats, with who is a bot, each button's last count and any slot given no seat; whether the game is paused, by whom, and the game's tick; open offers, and commands and answers waiting for a pause to end; the turn kept for each player's heading; the level's recipe, with each tile map's identity, origin and the sheet its load named, and the pieces that stand; each phase's clock; the lobby's setup; each player's profile; the pieces of a save being brought in, and the save a host with no window read before the first tick, until its lobby is left; which prefab each entity was spawned from; what a leaver's seat held, until the tick they left in ends; an offer's choices once they are drawn; how many times a function has run inside a tick at the game's tick, which no run outside a tick changes | The world, as resources and components the SDK registers | Keeps them as world state, so they are digested, rewound and sent to a late joiner |
| Events for the frame, effects under way, `Screen<T>` with the input context, `Shown<L>`, bindings with a pad's, the music and the sounds under way, the built collision of each recent level recipe, the path a followed move gave an entity until its tick ends | The SDK, as this screen's or this machine's state. It is not world state and is in no digest | The ring of recent ticks' events, each cursor, the count of this player's predicted emissions shown for each recent tick, the bursts still playing; on each machine, whether its screen shows the pause, which decides what its sampler counts, which `Player` the machine is and what its profile's slot last held |
| Saves and recordings | The SDK, in each target's storage | Runs the capture on the world as last committed, writes away from the tick, brings a save in as input, and keeps a profile's slot equal to what the world confirms for its own player, after a tick and after a baseline. It records each pass of the rules check against the rule sources' digest, in the game's build folder |
| The game's scenes, rules, looks and content | The game | — |

Rules run on the world's own thread where the shell has one, as today. Frame functions never wait for them:
they read published pages, on the thread that samples the scene.

## Interfaces

**Provided.**

| Interface | To | What it is |
|---|---|---|
| **The `thrum` crate** | Every game, the kit games, the templates | [The core](#the-core), sections 1 to 13, with the names in [the reference](#the-reference) |
| **The game commands** | Every game | [Section 14](#14-running-a-game-documentation-and-templates), and `./dev new` |
| **The SDK guides and the one file** | Game makers and their agents | [Section 14](#14-running-a-game-documentation-and-templates) |

**Asked of other modules.** Each is new work inside state another module owns, built by this module's builder
under [the seam rulings](../ARCHITECTURE.md#seam-rulings-for-game-sdk), on what the owner has landed, and the
owner's afterwards. No contract's text changes. C01's stable identities, C02's committed state, C11's event
order and C13's authority are what the core stands on.

1. **The rules profile** (`SIM-2D`): literal constructors that are `const`; angles and turning; `thrum` as a
   dependency checked rules may have; refusals of files, sockets, processes and standard input; a check of a
   game's `rules` module built on its own, outside the engine's registry; and, as additions to `thrum-sim2d`,
   exact tests for rectangles and capsules beside the index's circles, fixed collision built from tiles and
   shapes, the box and circle move that reports its contacts and the legs it took with the time of each,
   and [the 2D contact tolerance](#7-space-and-the-level) as a public constant its sweeps and moves hold.
2. **The registered simulation** (core): queries over several components with filters; a barrier after each
   registered system at which the tick's staged structural edits are applied, inside the tick's one
   transaction, for a schedule that asks for it; and what a parent link and a disabled entity need.
3. **The shells and the tool** (`CREATOR`, `HOST`, `GAME-UI`): what the window, the page and the phone shell
   still assume of the V1 game or the sky kit; local actions, the pointer and its capture handed to the SDK;
   storage for saves on each target; a driver that need not step a paused tick that admits nothing; the game
   commands in `./dev`; and, for O8, a pad's sticks, buttons and triggers and its rumble on the Mac and in
   the browser, and a game hosted with no window.
4. **The interface and the match** (`GAME-UI`, `SESSION`): used as built. A press count and a sampled bit on
   the input frame's axes; the predicted controller's whole state carrying what the step reads, compared as
   `SESSION` compares it; seats, with bots, that the SDK keeps in world state over `SESSION`'s slots; a lobby
   that runs on `SESSION`'s ticks from the moment of hosting; a pause kept as world state while the host goes
   on committing; a save brought in as pieces budgeted against the host's whole 920-byte frame, with a
   failed restore retried inside the world's one step; a match of fewer than four seats kept to its number
   by the SDK, which is what hands `SESSION` each link and what seats each joined slot; a note of a step's
   emissions held in the `Rules` value the SDK gives the predicted controller; a player's name sent as
   their first command; a `Heading` carried as a latest-value action; and a 3D world's image, digest and
   control function all fit what `SESSION` has, and none asks it to change.
5. **The repository guard** (`REPO-RULES`, `GAME-SPLIT`): `thrum` registered, and
   [the rule](#1-a-game-one-crate-one-package-one-value) that every package of a game names no engine crate but `thrum`.
6. **The 3D scene's entity rows** (`thrum-render`, and `CONTENT-LOOP`'s cooked mesh rows): the turn and the
   scale at both ticks, interpolated by every pass that places an entity row; a row's mesh chosen by name
   when its entity is spawned; and a row's material taking a texture.
7. **3D space** (`SIM-3D`, a new module the owner's answer to question 1 asks for). Used through its public
   interface, as `SIM-2D`'s is: an index of spheres, capsules and cuboids on layers that answers overlaps,
   rays and swept shapes; fixed collision built from triangle meshes, a heightfield and boxes, each on the
   layer the SDK gives it from the level's recipe, which answers the same for the layers asked and the
   height of the ground on them; and a kinematic move against both that slides, climbs a step, holds to a
   slope, and reports the legs it took with their share of the step, and its contacts with each one's
   normal, where the mover stood, how deep it began and the motion it had left. Its contact tolerance, a
   public constant of its pair tests, is what [touching](#7-space-and-the-level) means in a 3D game. All of
   it is in the rules profile's numbers and equal on the three targets, with no rigid body. [The module list](../ARCHITECTURE.md#track-7-the-game-sdk) holds the interface.
8. **Sound** (`AUDIO`, a new module the owner's answer to question 3 asks for). Used through its public
   interface: sounds and music cooked from `.wav` and `.ogg` sources; a mix on the Mac, in Chrome and on the
   iPhone simulator that plays a sound as a voice with a volume, a pitch and a place, loops one, streams
   music and cross-fades it, and bounds the voices of one sound; three volumes; and the mix rendered to a
   buffer with no device, so a test reads what would be heard. It runs off the frame and off the tick.

## Cost

The SDK is a layer over engine paths that are already costed. It must add nothing that scales with the number
of entities beyond what the engine does.

| Work | Account |
|---|---|
| A rule | One call each tick through the engine's registered callback, into a function monomorphised for the rule's parameters. The loop over entities is inside it: no dynamic call for each entity and no allocation in a tick |
| The barrier after a rule | Nothing when the rule staged no structural edit, which is most rules in most ticks. Otherwise the staged batch, once |
| A query | A walk over the engine's columns. `par_each` uses the engine's chunks |
| A space query | One index for each layer, with its storage kept. A layer is rebuilt at most once after each rule that changed it, and not at all in a tick that did not: a crowd that moves once a tick is indexed once a tick, as the reference crowd is |
| The level | Built once for what is solid in each recipe, when a rule that changed it finishes, and kept by that identity while a rewind or a step run ahead may ask for it. A character move tests the tiles its box sweeps |
| A followed move's path | Kept only for an entity whose path this tick is something other than one straight leg that took the whole tick from where the entity stood as the tick began: a move that bent, one that was stopped and came to rest before the tick was over, or one that began somewhere else. Dropped when the tick ends. A move that met nothing, a step along the floor stood on, and a carry with a step across open ground each come to that one leg and keep nothing, so a crowd that follows its moves across open ground pays for none |
| A parent's children | `under(parent)` walks that parent's own children and searches nothing else. An entity with no parent and no child pays nothing for the tree |
| Where an entity came from | One small value on each entity spawned from a prefab, which a spawn from a bare tuple does not carry |
| A control function | One call for each player's character |
| The access check | Against signatures and prefabs at startup, when an entity with a new set of components first appears, and when a player is given another character |
| Extraction | Changed rows only, mapped by code written once in the SDK. A tile layer is extracted when it is loaded |
| A frame function | Runs once a frame, over what it names |
| The interface | A description of tens to a few hundred nodes compared with the last one. An unchanged interface draws once |
| A save | The capture, once, on the world as last committed, with no tick stepped for it; the file is written off the tick. A restore is gathered a frame's room of pieces a tick, about 850 bytes in a match, and decoded once |
| A save's kept entities | Read once at the capture, over the entities of the prefabs it names, and spawned once at the restore, after one pass that takes out what stands of those prefabs |
| Sound | Mixed by `AUDIO`, off the frame and off the tick. A frame function's call queues a voice and returns |
| A pause | A paused tick that admits input runs the rules marked for it. One that admits nothing moves the committed tick and nothing else, so alone it is not stepped, and a recording keeps the count |
| A capability left out | Nothing: a 2D game compiles no 3D pass, a game with no interface has no stage, a game without the `multiplayer` feature links no network crate |

**Tripwires**, recorded and investigated while building goes on; none is a pass condition beyond a missing
measurement or a collapse:

- the reference crowd's behaviours at 10,000 bodies as SDK rules cost about 25% more a tick than the
  hand-written reference, the brief's figure for a comparable regression;
- any allocation in a warmed tick or frame of the 2D starter that the same scene on the engine's API does not
  make;
- a frame of the showcase on the SDK that costs about 25% more than the same frame on today's API.

## The work packages

Nineteen, in the order they are built. [The plan](PLAN.md) names each and [the done-when file](DONE.json) holds
the items of the first three. Numbers are names, not the order: WP3, WP5, WP6 and WP7 keep the numbers other
modules already wait on. The owner's yeses (O8) added WP15 to WP19, widened WP13 and WP5, and ask for two
engine modules, `SIM-3D` and `AUDIO`, which [the module list](../ARCHITECTURE.md#track-7-the-game-sdk) names
and whose own planners plan them. A work package builds an entry of
[the extension map](#the-extension-map) only where this table gives it one. The work packages that wait for
a module not yet built come after WP7, so nothing `SWARM` waits on waits for them.

| Work package | Builds | Waits for |
|---|---|---|
| WP1 | Sections 2, 3 and 4 with no window, for [the headless subset](#what-wp1-compiles): the world, rules in written order with each rule's edits landing when it ends, parameter structs, scenes as types, phases with their clock and `resume`, timers, the test harness; the surface and documentation checks | Nothing |
| WP11 | Prefabs and the tree of section 2: prefab names in `prefabs!` blocks that the program lists for the SDK, variants and what a spawn must bring; `Transform2d` and `Transform3d` as plain data made from integers; children, parts, attaching and disabling, with turns that are quarter turns, which need none of WP2's angle maths (WP2.3 proves `keep_world` under any other angle); `keep_world` with its bound and its refusals; the access check made on what exists | Nothing |
| WP2 | Sections 5 and 8 and `Space2d`: the numbers page and the rules check; action maps and how a button's presses are counted, players as stays in seats, a controlled character, bots; the crowd on the SDK, equal on three targets, with its cost | `SIM-2D:WP3`, Done |
| WP12 | The rest of section 7 and content: the level's recipe and fixed collision, tile maps, a space that meets the level, the character move and the path a followed move leaves, a contact's depth; typed content, its validator and IDs; and a shared setting as a game resource with its defaults, which rules read and a test sets, with no panel. Example D's rules, proved with no window | Nothing |
| WP3 | 2D on screen: section 9 in 2D, section 14's commands and the asset catalog for the Mac and Chrome, with [sheets cooked from source images](#sprites-clips-and-tiles-from-source-images), the first `run()`, which plays only rules the check has passed as built, cameras and rigs, `Shown`, local actions, input contexts, the pointer, the pause alone and `--shot`. Rockfall and Skylark without their interface | Nothing still open |
| WP4 | 3D on screen: section 9 in 3D, with entities' models drawn as entity rows that turn, scale and take a texture, placements, and the rest of settings: local settings and the panel, and a shared setting's panel edit admitted at a tick. The starter, stones and the lighthouse are written on the SDK as new kit games | For its change to the instance passes, the V1 work packages that hold them |
| WP6 | Section 6 and the interface: modals, commands, offers that queue with a bot's answer, requests, what a menu does under its own pause, and `TestScreen`. The interface of Rockfall (its score and result page) and of Skylark (its pause menu) | `GAME-UI:WP4` |
| WP13 | Saves and recordings: a saved resource, a run saved and resumed alone into its phase, the entities a save keeps, [a profile](#saved-progress-for-every-player) for a game played alone, `--record`, `--replay` and `play_recording`, with each input held at its committed tick | Its own WP6 |
| WP5 | Section 11: the crowd hosted and joined, a control function run ahead with what it reads, the standard lobby on the match's clock with `continues` and `continues_with`, seats and bots in a match, a pause in a match, a save restored in a match, a guest's profile brought in and kept, [a host with no window](#smaller-entries) and the save it brings in, `TestMatch`, and a 3D match rehearsed with no window | `SESSION:WP7` |
| WP14 | A 3D match on screen (O6): the 3D starter hosted on the Mac and joined from Chrome and a second Mac process through the standard lobby, each player's ball answering at once on their own screen | Its own WP4 and WP5 |
| WP7 | The iPhone simulator | `GAME-UI:WP6` |
| WP15 | [Gamepads](#gamepads): a pad as a binding for every action kind, a trigger, a modal worked by pad, the prompt's glyph for the device last used, remapping, a `Heading`'s sensitivity and inversion, and rumble, on the Mac and in Chrome | Its own WP6. Pad input in the shells is its own to build, under ruling 3 |
| WP16 | [3D space](#3d-space-and-a-kinematic-character) with no window: `Collider3d`, `Space3d`, `Level3d` and `LevelMut3d` over `SIM-3D`, a terrain as content, the character move, and a 3D character run ahead against the level in a rehearsed match | `SIM-3D`, once it is planned: the work package that delivers the move |
| WP17 | 3D space on screen: a terrain drawn from the source that is stood on, what a level adds and places drawn, [`Aim` and a rig that pitches](#camera-pitch-and-rigs) with the pitch's inversion and a boom that avoids the level, the 3D starter given ground and a rock that stops the ball, and [a solid shape's edit held for the next start](#10-content-assets-saves-and-settings), on the Mac and in Chrome | Its own WP4 and WP16 |
| WP18 | [Audio](#audio): sounds and music cooked into the catalog, `Audio`, the `Sound` look with `Shown<Sound>`, `Screen<Music>` and `Mix`, on the Mac and in Chrome, with function on the simulator | `AUDIO`, once it is planned: the work package that delivers the mix on the three targets |
| WP8 | The guides, the reference file and the templates of section 14, [the guard](#1-a-game-one-crate-one-package-one-value), and three fresh agents | Its own WP1 to WP7 and WP15 to WP18 |
| WP19 | The owner's sitting: the owner plays a 2D game and a 3D game with a pad in hand and the sound on, and says what is wrong | Its own WP15, WP17 and WP18 |
| WP9 | The showcase on the SDK, in the games repository | `GAME-SPLIT:WP4`, which follows the V1 exit |
| WP10 | The old creator route removed | Its own WP9 |

Each work package's surface arrives with its reference documentation and its rows of
[the mistakes table](#12-mistakes-that-explain-themselves), because WP1's guard holds every later one to them.
A work package is compared with the worked examples only for the surface it builds.

What the planner who gives a later work package its items carries into them. Each proof is listed under the
first work package that has everything it uses, so no item waits on a later one.

- **WP12:** a level loaded from a content file in an enter function; a shape given as the character's own
  collider that lands on a floor, stops at a wall, passes up through a one-way tile and stands on it;
  `moved.slide`; a `teleport` that sets the place and marks the tick; a ray and a `motion_of(..).sweep()` asked
  of the space on two layers, one the level's and one of colliders, each reporting whichever stands nearer
  along the path, with no entity for the level; a hit's normal pointing toward the mover on a floor, a wall
  and a ceiling, and a sweep that starts in contact reporting fraction zero with a normal that is not zero; a
  move asked of the space on its mover's own layer, which with `.except` passes its own collider and stops
  at another's; a shape at rest beside a wall it is not moving into, whose move reports `wall()`; a glyph
  whose spawn function reads `Placed` and a content record and spawns a `Prefab<N>` with its `N` at each
  such cell, in row order, the same after a rewind and a replay, and one defined outside the `rules` module
  refused at startup; two maps loaded side by side with `add_tiles`, each wall where its origin puts it; one
  map loaded at one origin `.drawn_with` each of two sheets by a rule that keeps no note of which, where the
  two worlds' digests differ, a rewind to a tick at which the other sheet was loaded gives that tick's
  digest again, and both give the same `move_shape` answers from one built collision, which a count the
  test reads shows; a
  tile map from a file and a rectangle a rule added later, both giving the same `move_shape` and `ray` answers
  after a rewind and a replay; one content record that holds two tile maps with a wall in different places,
  and a list of them, where loading each gives its own wall before and after a rewind to a tick at which the
  other was loaded; a validator that counts a map's cells of one glyph; a content record refused before play
  by its type and by the game's validator; a validator that takes a second content type and refuses a level
  that names an enemy with no record; an `Id` declared in an `ids!` block and used only in a branch of
  a rule, refused at startup when no file has its name, with the nearest name; rules that read a shared setting registered with its defaults and
  set by the test; a rectangle a rule added and kept as a `LevelPiece` in a resource, removed by a later
  rule with `remove` returning true once and false after, where a move that stopped at it passes, the
  recipe's count goes down by one, and a rewind to a tick before the removal stops the move again; an
  entity teleported in a tick whose `motion_of(..).sweep()` meets nothing that stands between the two
  places; **the three ways a path meets a thing, against the level**, as WP2.12 proves them against a
  collider: a swept circle that meets a wall on the way, whose `mover_position` touches the wall and does
  not overlap it, with a `depth` of zero and a `remaining_motion` that with it ends where the path ended;
  one that begins touching the wall, which reports fraction zero, the place it began, a depth of zero and
  the whole path; one of radius 1 that begins with its centre half a unit inside the wall's face, which
  reports fraction zero, the place it began, a depth of 1.5 along the wall's normal and the whole path, and
  which set at `mover_position + normal * depth` touches the wall and does not overlap it; each of the
  last two reported the same when the path leads away; and a move that begins inside a wall, which reports
  that contact first with its depth, goes no deeper, travels the part of its step that leads along and
  out, and is not pushed out; **a followed move:** a hero whose step of (3, 3) from (0, 0) is
  stopped by a wall when its centre reaches (1, 1) and slides up it to (1, 3), with a still circle on
  another layer at (1, 2) and the two radii summing to 0.20, where after `at.follow(&moved)` the hero's
  `motion_of(..).sweep()` reports the circle with a `fraction` between one third and one, a `mover_position`
  on the wall's leg, `mover_position + remaining_motion` at (1, 3) and an `approach_speed` that is the
  second leg's own, three units in a tick's length, and the same move with its position
  written by hand reports nothing; a circle that crosses the wall's leg and is gone before the hero's first
  third of the tick is over, which is not reported, and one that crosses it while the hero is on it, which
  is; a position written after the `follow`, after which the path is the straight line again; a move that
  met nothing, followed, which sweeps as the same position written by hand does; **a move stopped
  short:** a hero whose step of (4, 0) from (0, 0) is stopped by a wall when its centre reaches (1, 0),
  with a still circle on another layer at (0.7, 0) and the two radii summing to 0.20, where after
  `at.follow(&moved)` the sweep reports the circle at a `fraction` of one eighth with
  `mover_position + remaining_motion` at (1, 0), and the same end written by hand reports it at one half; a
  circle that crosses the hero's line at (0.75, 0) three quarters of the way through the tick, which the
  hero that followed does not report and the one written by hand does; and a circle that comes against
  (1, 0) in the tick's last quarter, which the hero that followed reports with a `mover_position` of (1, 0),
  a `remaining_motion` of nothing and an `approach_speed` that is the circle's own; **a carried rider:** a
  rider that follows a carry of (2, 0) from (0, 0) that met nothing, and then a move of nothing from where
  it stands, with a still circle on another layer at (1, 0) and the two radii summing to 0.20, whose sweep
  reports the circle at a `fraction` of 0.4; the same rider with a second move of (0, 1), whose path is
  the straight line to (2, 1), so that a still circle at (1, 0.5) is reported at a `fraction` between 0.4
  and 0.5 and one at (2, 0.5), which the carry and then the step taken one after the other would pass
  through, is not; the same carry written by hand as a position and the move of (0, 1) followed after it,
  which reports the same two answers; and a second move followed from another place, after which the path
  begins at that place and the circle at (1, 0.5) is not reported; and each of these the
  same after a rewind and a replay; a group of rules `.during` two of a scene's three phases, which runs in those two and not in the
  third; a `List` in a scene's fields, read in the scene entered with it; a `Side` kept in a component; a
  test that names a fixture's record with `test_id`, reads it back with `content`, and hands a value to
  `game.run` in a closure; and example D's two coyote tests.
- **WP3:** a picture the engine encodes and drops, with an event delivered to its run and a `burst` started
  in it shown in the next; a `teleport` with no picture between the two places; a pause, toggled by a local
  action, in which the game's tick stands and no timer moves, and which the same key ends; a rule marked
  `.even_when_paused()` that counts its runs, which runs in a paused tick that admits a join and in no
  paused tick that admits nothing, while no unmarked rule runs in either; a run paused at the game's tick
  40 and stepped through ten paused ticks that admit nothing, beside the same run with those ten left out:
  in both `Tick::number()` reads 40 until the pause ends and 41 in the first tick after it, every component and resource reads
  back equal value by value, and the two digests differ, because the first run stands ten committed ticks
  further on; a game with no `rules` module, which `test()` steps and `run()` and `cargo play` refuse, each
  naming its rules and `mod rules { .. }`; a tap of a shared button made during that pause, which no tick
  after it reads, a tap made after it, which is read once, and a direction held through it, which is still
  held after with no `released`; a key bound in two input contexts driving only the active one's action, and
  a key that went down in one not pressing the other's; a frame function that reads a `Timer` through `Tick`
  and shows the same time left a rule reads, and one that draws `frame.fraction(&cooldown)`, which moves
  between two ticks while `cooldown.fraction(tick)` reads the newer tick's and which stands in a pause; a
  frame function that switches this screen to an input context with no `Heading` bound to the mouse, which
  frees a captured pointer, and back to the play context, which captures it again; a map loaded
  `.drawn_with` one of two sheets, drawn with that sheet and drawn with it again after a rewind past a load
  that named the other; a following rig whose damping a frame function changes through `Screen<Camera2d>`, which goes
  on following; `Camera2d::exactly` showing the same rectangle in a wide window and a tall one; a sheet with a
  clip and two tiles cooked from an image and its file, a misspelled frame refused by the check with the
  nearest name; a frame function that asks for `PlayersMut`, which does not compile; a game outside the
  checkout, laid out as the templates are, whose registered frame function does float arithmetic and keeps a
  `HashMap`, which passes its check and runs; and the same game built by Cargo alone, with no `cargo play`
  command, which runs through `game().run()` once `cargo play --check` has passed; which runs again with no
  new check when Cargo alone builds it with a feature of its own package turned on, a feature its `game()`
  reads to choose a resource's starting value; and which, with a file
  read then put inside its `rules` module and built by Cargo alone again, is refused by `run()` before the
  first tick with a message that names the check, while `cargo play` refuses it in the check with the line;
  and a rule marked `.even_when_paused()` that draws from `Random` in two paused ticks at one game's tick,
  whose two draws differ and are the same two in a replay and in a run that committed ten empty paused ticks
  between them.
- **WP4:** example B's cooked, textured rock turned by the example's rule `turn`, with `turn_rock` left out
  and the tick rate low enough to see, so its `Transform3d` holds two different turns at two adjacent ticks:
  pictures captured between those ticks show the rock at turns between the two, its lit faces and its cast
  shadow together, and with `turn` taken out of the scene the same capture shows it still. Then the
  decorative turn alone: `turn_rock` turns the rock in every picture through `Shown<Model>` while its
  `Transform3d` does not change. A built-in cube does not stand in for the rock. An orbiting rig whose
  distance a frame function changes through `Screen<Camera3d>`, which goes on orbiting. A rule that calls
  `players.face` for a player, after which a later rule of that tick reads the new heading, a frame sampled
  before it turns the heading only by what the mouse moved since, and the rig cuts to it. And example B's
  ball, steered by the keys on the Mac and in Chrome.
- **WP6:** an offer answered twice, answered late, answered for a bot by a rule, and shown to a player who
  joins while it is open; three offers opened to one player in one tick, shown and answered one at a time in
  the order opened; a command sent in a dropped picture, admitted once; a `request.scene` the scene does not
  accept; a pause menu as a modal that the key which opened it closes; a modal opened in a scene whose input
  context binds a `Heading` to the mouse, which frees the captured pointer for its buttons and gives it
  back when it closes; a modal that holds the pause, whose
  button sends a command that a rule marked `.even_when_paused()` reads, after which the world closes the
  modal and the pause ends; a command no such rule takes, sent during a pause and read once in the first tick
  after it; a `request.scene` sent from a pause menu, which lands and ends the pause; a card outside a modal
  pressed by a local action while the stick that walks the hero goes on walking it; an offer with no expiry
  opened in one phase and answered in the next; a card chosen inside the loop that walks an open offer's
  choices; three offers opened to one player in one tick with `.choices_with`, whose function leaves out
  what the hero already holds, where the second hand is drawn only when the first answer has landed and
  leaves out what it gave, the function is asked once for each, a player who reconnects is shown the hand
  already drawn, and one defined outside the `rules` module is refused at startup; an answer admitted while
  a modal holds the pause and no marked rule takes it, reported once with its type; a struct of frame
  parameters taken by two frame functions; and `advance`, after which an eased camera has arrived with no
  tick stepped.
- **WP13:** example A as it stands, one file with its `rules` block, recorded by `cargo play --record` on
  the Mac and replayed by `--replay` in Chrome and by `play_recording` in its own test, each to the run's
  digest; a game with no `rules` module refused by `play_recording` with the step to take; two recordings of
  one run alone that differ only in a pause, during which two commands are admitted in one tick in the first
  and in successive ticks in the second: a rule marked `.even_when_paused()` counts one run in the first
  and two in the second, and each recording replays to its own run's digest at every committed tick; a
  recording of a run stepped through ten paused ticks that admit nothing, whose replay ends at the same
  committed tick with the same digest; a capture defined outside the `rules` module refused at startup, and a `Save` whose
  conversion from its older type is written outside it refused by the check; a save written by one build's
  type and restored by its successor's; two save types in one game,
  each request reaching its own capture; a save asked for from a pause menu alone, during a jump, written
  with the menu still open and no tick stepped, the committed tick and the digest the same before and
  after, and the hero in the air when it is restored; a save whose `save_if` condition takes `Random`,
  asked through `saves.can_save` twice in one picture of a pause menu at one paused tick, where the two
  answers are equal, the committed tick, the game's tick and the digest stand, and, beside the same run
  with the menu never opened, a rule's draws after the pause and the digest at every committed tick are
  equal; a
  save restored into a phase whose `lasts` function read ten seconds from a shared setting on entry, captured
  four seconds in after the setting had changed to twenty, which ends six seconds after the restore with no
  enter function of any phase run and that function not asked again; a save captured in a phase after its
  `then_run` rule had run and asked for nothing, whose clock reads `left()` as `None`, restored, in which
  the rule does not run again; a `Timer`
  of four seconds saved two seconds in and one of eight seconds saved six seconds in, each restored with
  two seconds left, the first with a `fraction` of one half and the second of three quarters, and a
  `Cooldown` saved part-way through its wait whose `left` and `fraction` read after the restore what they
  read at the capture, and whose ring, drawn by a frame function from `frame.fraction` in a picture taken
  at the committed tick with no time passed since it, is the same before the capture and after the
  restore; a restore whose enter function fails, a save over its limit and a damaged one, each leaving the
  world as it was and the scene not entered, with the step that tried it returning no error and the next step
  running; a recording that holds a restored save and a saved resource's starting value, replayed with
  another value in the slot to the same digest. **Kept entities:** a save with two `Kept` fields, one of two
  prefabs and a variant, captured with one entity disabled and one hanging under an entity of the other
  field by a spawn's `.child`, and restored into a fresh scene: each comes back from the prefab it was
  spawned from with its kept components, a `Timer` among them with its time left, the disabled one disabled,
  the child under its parent at its local place, a definition's own child once and not twice, and a player's
  character not at all; two captures with no tick between them equal; an entity of a named prefab that
  lacks a listed component, which fails the capture with the three names and writes nothing; a file whose
  kept name the game no longer declares, which is `Loaded::Failed` with the field and the name, and the
  same file loaded through a conversion that renames it; two entities of a `Prefab<N>` spawned with
  different values of its `N`, each coming back with its own; `keep.all` of that prefab into a `Kept` that
  leaves its `N` out, which does not compile and names the component; a file that names a prefab whose
  spawn must bring a component its field does not hold, which is `Loaded::Failed` with the field, the name
  and the component; and the size of a save that keeps a thousand entities, recorded beside its limit.
  **A level and a save together:** a map whose glyphs place two coins and a moth, with one coin collected,
  saved with a `Kept` field filled from the coin's prefab and restored into a scene whose enter function
  loads the same map and then calls `respawn`: a move is stopped by the floor where the map put it, exactly
  one coin stands, in the survivor's cell, and the moth stands once; the same with the load in one enter
  function and the `respawn` in the next; the same save taken with both coins collected, restored to no
  coin; and a field a conversion started as `Kept::none()`, whose `respawn` takes nothing out. **A profile alone:** registered with `Game::profile`, read before
  the first tick from a fixture, written to the slot when a tick commits a change to the player's value and
  not by a step a test rewinds, restored from an older version through its chain, landing as the default
  where its `validate` fails, and held in a recording, which replays to the same digest with another value
  in the slot.
- **WP5:** a control function that reads a shared setting, the phase, the level and `tick.in_scene()`, run
  ahead for a player who joined at a tick other than zero, in a scene entered at a tick other than zero:
  every step it runs ahead with the frame the host then admits for that tick, while nothing it reads has
  changed, equals the host's, and a hero the function holds still for the scene's first two seconds is
  released at the host's tick and not the joiner's; a frame the host holds, after which that tick is
  corrected; the host changing that setting while the character stands still, after which the peer is
  corrected and its later steps are equal again on the same terms; the same phase entered again, corrected
  the same way; a function that sends one dust event when run ahead and two at the confirmed tick, where
  the player's screen is given the first once, when predicted, and the second once, at the confirmation,
  and one that sends the same event in every run, given once across a correction; two players
  who send the same event from the same control line in one tick, where each screen is given its own
  player's when it is predicted and the other's at the confirmation, each once and each with its own place;
  the startup refusal
  without `.unpredicted()`; a press sent on a late frame and one made across a neutral frame, each read
  once; with the last count read at five, a tap made while a peer's screen shows a pause and a tap made
  after it shows the game running, each with that peer's frames lost through the pause's end, where the
  first is never read and the second is read once; a person joining a table bots have filled, with the same
  seats, characters and digest on the host, the joiner and a replay, and a shot the bot had fired naming
  nobody afterwards; in a match of two seats with a host and a guest, a third person turned away as full
  with no event and no character, the same third person given no seat when they arrive while the guest is
  away, and the guest returning within the grace to their seat and character; a ready mark and a host's
  panel choice, made in the lobby from a panel that lists content, carried into the scene it starts and
  read as `Res<Setup>` by a rule of the scene after that, and a rule that returns everyone to the lobby; a pause ended from a peer, with no burst of ticks after, and a command a peer sent during it
  read once; a run with a pause in which a marked rule runs once, on a command, recorded alone, where the
  driver commits no paused tick that admits nothing, and replayed as a match, to the run's digest at every
  committed tick; the same script played live in a match, whose host commits a paused tick each tick, after
  which every component and resource reads back equal to the run alone, at a committed tick further on by
  the paused ticks the host committed; an open offer shown again after a reconnect; a rectangle a rule added to the level, met by a
  peer who joined after it; a map a rule loaded `.drawn_with` one of two sheets, drawn with that sheet on
  the screen of a peer who joined after the load; a save within 1 KiB of its 64 KiB limit that the host restores while peers play
  and while the host sends a command in some of the same ticks, arriving over many ticks with no frame
  `SESSION` refuses and every world's digest equal afterwards, and a damaged one, after which the match goes
  on, the scene is not entered and the digests are equal; **and a rehearsed 3D match** that leaves
  the ground as example B's hop does, so that a predictor which kept only the ground plane and the yaw would
  fail it: a control function
  that writes a `Transform3d` from a `Stick`, a `Heading` and a `Button`, leaving the ground and facing
  along its velocity, run ahead for a late joiner whose shown `Pose3d` moves along the ground, rises and
  tilts in less than one trip to the host; a correction made while it is in the air, after which its height
  and its turn are eased to the host's, the turn the shorter way; a peer who joins while another's character
  is in the air and is shown it at the host's height and tilt; and every world's digest equal afterwards,
  with each character's `Transform3d` read back equal, height and rotation included. **A guest's profile:**
  a peer that joins with a profile of 12 KiB, whose `PlayerJoined` is read only once the value has landed
  and whose rules then read it; the same join with the guest's frames lost for a second in the middle,
  which finishes; a guest whose link drops while a rule raises their best, and who returns within the grace
  to find it in their slot though they stepped no tick that changed it; a guest who returns after the grace
  and joins as a new stay with what their slot held; a value that fails `validate`, landing as the default
  on every machine; no slot written for another seat's player or for a bot; and the digests equal
  throughout. **`continues`:** a host's lobby given a save by `test_setup()..start_match()`, where
  *Continue* enters the scene with the save restored on the host and on a guest who joined the lobby, and
  *Start* from the same lobby enters it afresh; with `continues_with`, a save made in region B continued
  from a lobby whose `starts` names region A, which enters region B on every machine and passes a scene's
  own check of its save against its value, while *Start* enters A; and a `continues_with` function defined
  outside the `rules` module refused at startup.
  **A host with no window:** `cargo play --host` on the Mac, joined from Chrome and a second Mac process,
  with no renderer, window or GPU crate in the host's program, the lowest seat's player pressing *Start*,
  and the digests equal. **Its save:** the same host started with a save in the slot its lobby continues
  from and nothing in the Chrome player's storage, where the lowest seat's screen shows *Continue*, the
  press enters the scene with the save restored on the host and on both peers, and the digests are equal;
  started with that slot empty, and with a file of a newer version, where the host prints which and the
  lobby shows *Start* alone; and in a rehearsed match started with `start_match_with_no_window()`: a peer
  who joins the lobby after the host started is shown *Continue*, no player's input frame carries a piece
  of the save, the world holds none of its bytes once the lobby is left by *Continue* and by *Start*, a
  recording of the match replays to its digest with another value in the slot, `saves.can_save` is false
  on the lowest seat's screen and its `restoring` request is refused with the message, and a rule's
  `Saving` writes the host's slot. **In the harness:** `peer(p).my_player()` for a peer between a wreck and a
  relaunch, and `set_resource` on the table changing every machine alike.
- **WP14:** example B's game with its hop, `.multiplayer(..)` and the standard lobby, hosted on the Mac and
  joined from Chrome and a second Mac process: a late joiner's ball on the host's screen, the joiner's own
  ball rolling and leaving the ground on their screen before the host confirms it, another player's hop seen
  at its height on each screen, and the three digests equal; captured in motion on the Mac and in Chrome.
- **WP15:** an action map whose `Stick`, `Axis` and `Button` are each bound to keys and to a pad, driven by
  scripted pad state through the shell's own route on the Mac and in Chrome, with the rules reading the same
  values as from keys; a trigger that presses its `Button` past half its travel and fills an `Axis` by its
  travel; a pause menu opened, walked with the D-pad and the stick, activated with south and closed with
  east, while the stick that walks the hero is not read by the game under the modal; `ui.prompt` showing the
  key after a key press and the pad's glyph after a pad press, captured on both targets; a remap screen
  that listens and rebinds a pad button, kept across a restart; `sensitivity` and `invert_x` on a `Heading`,
  where the same scripted movement of the right stick, and of the mouse, turns the angle the rules read
  1.4 times as far under `sensitivity(.., 1.4)` and the other way under `invert_x`, each kept across a
  restart (the pitch's `invert_y` is WP17's);
  a `Rumble` pulse that reaches the shell's pad route and is nothing with no pad; and no pad crate in the
  closure of a game built with `thrum`'s default features off. A pad in a person's hand is WP19's.
- **WP16:** each proof of WP2.2, WP2.12 and WP12 for a space and a level, with a 3 on it, over `SIM-3D`: an
  overlap, a ray and a swept sphere against colliders and the level on two layers, with no entity for the
  level; a capsule moved by `Level3d::move_shape` over a terrain made with `Terrain::from_heights`, which
  walks up a slope under `max_slope` and is stopped by a steeper one, climbs a step under `step_up` and is
  stopped by a taller one, holds to a slope it walks down with `snap_down`, and is not pulled down while it
  rises; a terrain loaded on one layer with a box added on another, where a ray, a swept sphere and a move
  asked on the terrain's layer alone meet the hill and pass the box, asked on the box's layer alone meet
  the box and pass through the hill, and asked on both meet whichever stands nearer; `ground_height` under
  that box, which is the terrain's height on the terrain's layer and the box's top on both; a climbed step
  followed with `follow`, whose legs a sweep of the capsule meets as it does a slide's; `hit.surface` for a
  floor, a wall and a ceiling; a model added with `add`, solid from its own triangles, and one given to `place`, which
  stops nothing; a piece removed, with a rewind; a mark set and read; a terrain and a level built from a
  content record in an enter function, the same after a rewind and a replay and for a peer who joined
  after; the same digests for a scripted walk on the Mac, in Chrome and on the simulator; and in a rehearsed
  match, example G's `walk` run ahead against the terrain for a late joiner, corrected when the host
  removes a piece under them.
- **WP17:** example G on the Mac and in Chrome through T2 capture: the terrain drawn where the pilgrim's
  feet meet it across a walk over a ridge; an added model and a placed one drawn, the first stopping the
  pilgrim; a rig that takes an `Aim` and pitches within its limits, whose boom shortens against a hillside
  and a wall and lengthens again; `game.steer` of an `Aim` in a test; `invert_y` on that `Aim`, where the
  same scripted push of the right stick, and of the mouse, pitches it up with it off and down with it on,
  within the pitch's limits and with the yaw as it was, and `invert_y` given a `Heading` does not compile;
  and the 3D starter's second page, on which the rock stops the ball, alone and in WP14's match. **A solid
  shape edited while the game runs:** with the ball at rest on the starter's rock under `cargo play` on
  the Mac, the rock's source made taller and saved: no later picture shows the taller rock, the ball rests
  where it did, and the console and the panel name the asset that is waiting; started again, the rock is
  drawn taller and the ball rests on its new top; and that rock's texture changed alone, which is redrawn
  in the running game with the ball's rest unchanged.
- **WP18:** a `.wav` and an `.ogg` cooked into the catalog, a `Music(..)` file making a `MusicAsset`, and a
  misnamed sound refused by the check; with `AUDIO`'s mix rendered to a buffer and no device: a sound played
  on an event present in the buffer once, a sound sent by a predicted event not played again at its
  confirmation; under `.limit(4)`, forty plays spread evenly through one second: of a sound one second long,
  where the buffer never holds more than four voices of it, four are heard and thirty-six dropped, and one
  more play made after the first voice has ended is heard; and of a sound 10 ms long, where all forty are
  heard and no two voices of it sound together; a sound `.at` a place louder
  near the camera than far and on the side it stands, a thing's `Sound` starting and stopping with its
  entity and with `disable` and `enable`, `Shown<Sound>` turning one down for one screen, music that
  cross-fades on a change and does not restart when the same track is said again, a pause that holds a
  thing's `Sound` and leaves the music, and `Mix` set through `Screen<Mix>` kept across a restart; the same
  through the device on the Mac and in Chrome, with function on the simulator; no audio crate in the closure
  of a game with `thrum`'s default features off; and no stall of a frame or a tick while a sound loads,
  recorded as a tripwire. How it sounds is WP19's.
- **WP8:** the guard's refusals; an unfamiliar change made from the guides alone; the guides' pages for a
  pad, for sound and for 3D space.
- **WP19:** one sitting, with `needs` of `human_feedback` and `hardware`: the owner plays the 2D template's
  game and example G's with a pad in hand and the sound on, on the Mac, and says of each what is wrong with
  how the pad feels, how the camera follows and how the game sounds. What they ask for is planned from
  their words.

### What WP1 compiles

WP1 has no window, no input, no prefab and no space query. Its test games are written from this subset, and
may be [sketches with no `rules` module](#1-a-game-one-crate-one-package-one-value), which `test()` steps:

- **The game and its scenes:** `Game::new`, `.scene(..)` with one function and with a tuple, `.resource(..)`,
  `.component::<T>()`, `.max_entities(..)`, `.tick_rate(..)`, and `.test()` and `.test_in(..)`, which return
  a result; scene types that
  derive `Scene`, with and without fields; in a scene, `resource`, `on_enter`, `on_exit`, `rules`, `phases`,
  `phase(..)` with `on_enter`, `rules`, `on_exit`, `lasts`, `then` and `then_run`, and `add`.
- **Rule parameters:** `Query` with `With`, `Without` and `pair_mut`, `Single`, `Res`, `ResMut`, `Entities` (`spawn` of a
  tuple, which returns its `Entity`, `despawn`, `insert`, `remove`), `Events` and `Emit` for the game's own
  events, `Tick`, `Random`, `Scenes`, `Phase` with `clock` and `PhaseMut` with `resume`, and a struct that derives
  `Params`; `every`, `when` and `par_each`.
- **Data:** the `Component`, `Resource`, `Data`, `Event`, `Scene` and `Phase` derives, `Timer`, `Cooldown` and
  `PhaseClock`,
  with plain components in place of transforms and looks. `Num` and `Vec2` are the profile's numbers as they
  stand, made from integers. Literals, angles and `Vec3` arrive with WP2.
- **The harness:** `step`, `step_ticks`, `step_for`, `step_until`, `spawn`, `despawn`, `set`, `get`,
  `resource`, `set_resource`, `count`, `find`, `single`, `alive`, `phase`, `scene`, `in_scene`, `go_to`,
  `go_to_phase`, `run`, `events`, `digest` and `rewind`.

From the examples, that is: example A's `Score`, `drop_rocks` with its cadence, `fall`, and `Play` and
`GameOver` joined by `go_to` with the score as the scene's field; and example E's `Run`, `Stage`, its three
phase blocks with their lengths, `next_wave`, `enemies_die`, `heroes_fall` and `clear_enemies`, with a tuple
in place of each prefab and a plain position in place of `Transform2d`.

## Risks

1. **Core cannot apply structural edits after each system inside one transaction.** Early sign: WP1's first
   item, which is written to be tried before anything else. If it cannot be built, the builder ends `BLOCKED`
   and the planner brings the owner the choice, because R2 is the owner's.
2. **The API in the examples cannot be built as written.** The likeliest places: an action map read as plain
   fields over the input frame, a control function with its context as a predicted controller, the access
   check made on what exists and seen through a parameter struct, frame functions with no window, and the
   interface kept from a per-frame description. Early sign: WP1, WP2, WP5 and WP6. The builder writes the
   difference in the log and ends `BLOCKED`, because a change of shape is the owner's to approve.
3. **A convincing fake of K2:** an example that reads plainly because the plumbing moved into a helper file
   of the game. Early sign: the reviewer's count of game lines includes every file of the game.
4. **The surface is too wide to document well.** Early sign: WP1's surface check, which fails a public item
   with no example, and WP8's agents.
5. **Angles and 3D turns in the profile differ on one target.** Early sign: WP2's conformance run. The fallback
   is a table.
6. **Compile errors from typed parameters read badly.** Early sign: each work package's mistakes item, which
   holds the message text.
7. **The layer costs the crowd its speed**, with a barrier after each rule and layers rebuilt on change.
   Early sign: WP2's cost record.
8. **Turning, textured models change passes the V1 exit stands on.** Early sign: an exact image that moves in
   WP4. A row that does not turn and has no texture is placed and shaded exactly as today.
9. **A wrong substitute for K7:** a fresh agent that succeeds because it read the examples' source in the
   engine checkout. Its brief allows the guides, the one file and the template, and its record lists every
   file it opened.
10. **A target's program cannot carry the list of `prefabs!` and `ids!` blocks.** Registration at link time is
    ordinary on the Mac and the simulator; on the browser target it needs the program's constructors run at
    start. Early sign: WP11.3 on the Mac, and WP2's conformance runs, whose trace row carries how many
    prefab names the SDK knew before it read the game's description. If a target cannot carry the list, the
    builder ends `BLOCKED`, because rooting a block by a line of game code is a change of shape.
11. **A 3D game's action set does not fit `SESSION`'s input frame.** Three 16-bit axes carry every button,
    `Axis` and `Stick` of the set, and a `Heading` rides an action. Early sign: WP2.4, which holds a set of a
    `Stick`, a `Heading` and four `Button`s that must fit. If an ordinary set cannot, the builder writes a
    proposal in `SESSION`'s log (ruling 4) and ends `BLOCKED`.
12. **An entry of the extension map turns out to need a core name changed.** That would mean the shape was
    not locked. Early sign: the design review of this design, which checks each entry against the core, and
    the plan of the first entry the owner asks for. The planner brings the difference to the owner before
    anything is built.
13. **The built program cannot carry its rule sources' digest without a line of the game.** `run()`'s
    refusal of unchecked rules, and the rule-set identity, both rest on that digest being fixed when the
    game is compiled. `SESSION`'s reference derives one in a build script, which a one-file game does not
    have. Early sign: WP3's item for a game built by Cargo alone. If it needs a line of the game's code, a
    second source file in the one-file starter, or a way to start a game that `cargo run` does not reach,
    the builder ends `BLOCKED`, because each is a change of shape.
14. **`SIM-3D` and `AUDIO` are not planned yet.** WP16 and WP18 wait for them, and WP17, WP8 and WP19 behind
    those. Early sign: the status page shows either with no plan while WP7 is being built. Nothing before
    WP16 waits: the 2D SDK, multiplayer, saves, the simulator and gamepads are built without them.
15. **No agent can hear a game or hold a pad.** An agent proves what was asked for and that it arrived: the
    samples in the mix, the action a pad state fills. Whether a pad feels right and a game sounds right is
    the owner's, in WP19, and a "no" there reopens WP15 or WP18. Early sign: the first capture of a mixed
    buffer, and the first scripted pad run on the Mac.
16. **A terrain cannot be drawn from the source it is stood on.** The engine draws cooked meshes, so the
    cooker must make a terrain's mesh and its collision heights from the one height image. Early sign:
    WP17's first capture. The fallback keeps the shape: a terrain given as a cooked mesh, whose own
    triangles are its collision, as a solid model's are.
17. **A save that keeps entities outgrows its limit** in a game with thousands of them. Early sign: WP13's
    record of a save of a thousand entities beside the 64 KiB default. `#[save(max_kib = n)]` raises it.
18. **The compiler cannot hold `keep.all` to what a prefab's spawn must bring**, or says it in words nobody
    can read. It is a bound over two lists of types, as a spawn's own check is. Early sign: WP13's first
    doc test that must fail. If the type cannot say it, the builder writes the difference in the log and
    ends `BLOCKED`: the same check made when the capture first runs is the fallback, and moving a mistake
    from the compiler to a run is the planner's to bring to the owner.
19. **A transform cannot carry a followed move's path to the end of its tick without every entity paying
    for it.** `follow` is a call on plain data, as `teleport` is. Early sign: WP12's followed-move proof,
    and the size of a transform before and after it, which WP12's builder records. If only a path kept in
    every transform can be built, the builder ends `BLOCKED`, because the crowd's cost is the owner's.
20. **A function's name is the compiler's rendering of its path**, which two versions of the compiler may
    spell differently. [The promise](#3-rules-and-the-tick) is one version of the compiler: every profile,
    feature set and target it builds draws alike. Two programs built by two versions are two builds, and
    the build is in [the rule-set identity](#11-multiplayer-an-opt-in-over-input-only-networking), so they
    start no match together; a recording replayed by a program built with another compiler may draw
    differently, as it may after any change to the rules. Early signs: WP1.3's literal draws in a debug
    and a release build, and WP2's digests on the three targets, which one compiler builds. If a profile or
    a target renders a path differently, the builder ends `BLOCKED`.
21. **Rust gives two functions one path**, as it does two declared under one name in two blocks of one
    function's body. [The SDK refuses the pair](#3-rules-and-the-tick) where both take `Random`, by asking
    whether the two have one Rust type. Early sign: WP1.3's pair.

## Proof

- **Correctness**, where a fault is silent and costly: written order and what a rule sees; equal digests when
  a run is repeated, rewound and replayed, and on the three targets; a refused tick leaving the world
  unchanged; the access checks; a match that stays equal, in 2D and in 3D. CPU tests on small worlds.
- **Equal digests are never the only witness.** A digest that left something out would agree with itself. So
  a refused tick is proved by reading the components, the resources and the entities back, value by value;
  the digest is shown to change when any one of them changes; and the crowd's trace carries witnesses of the
  work it claims.
- **Mistakes**: one test for each row of [the table](#12-mistakes-that-explain-themselves), holding the
  message's code, object, cause and next step.
- **The surface**: no type of another crate in `thrum`'s public signatures, Rust's own and the game's own
  apart; an example on each public item; guides that compile.
- **The shape**: a design reviewer checks each entry of the extension map against the core: it names its
  extension point, adds names and changes none.
- **Looks**: the kit games drawn on the Mac and in Chrome, inspected in motion through T2 capture.
- **Sound and pads**: in tests, by what was played and what reached the mix, and by the action a scripted
  pad fills. How they sound and feel is judged by the owner, once, in WP19.
- **The docs**: three fresh agents, commissioned by the lead as `CREATOR`'s was, one for each kind of game,
  each then asked for a change the guides did not show.
- **Cost**: the tripwire records above.

## What a builder may change

The core, the reference and the worked examples are the shape the owner approved. A builder may change a **spelling**: a
method's name, an argument's order, a derive's attribute, where the game's code would read as well or better.
Each is a line in the log. A builder may not change the **shape**: add a concept a game maker must learn,
remove one of the fifteen words, change what a rule sees or when its edits land, change the written order,
change what a control function may read or which of it a match runs ahead, change what happens in a pause,
move a number between the portable kind and the float kind, move a line of plumbing into game code, make a
game name an engine crate, build an entry of the extension map that
[the work packages](#the-work-packages) do not give its work package, or build something that closes one of
the eight extension points. Where the shape cannot be built, the builder ends `BLOCKED` with the difference, and
the planner brings it to the owner.

When a work package is Done its examples in this design are replaced by the ones that compile, so the design
keeps reading as what is true.

## Open questions

- **How many entities a world holds by default.** WP2's builder chooses from the cost record and documents it.
- **How the SDK learns the module a registered function is defined in.** WP1's builder chooses, and says in
  the log what a game maker would see if it guessed wrong.
- **How the built program comes to carry its rule sources' digest with no line of game code**
  ([risk 13](#risks)). WP3's builder chooses, for `run()`'s check, and says in the log what a game maker
  sees. WP5 derives the rule-set identity from the same digest, with the content and the build.
- **Whether 64 KiB is the right default limit for a save.** At the host's frame budget it takes about a
  second and a third to bring in to a match. WP5 records the time it measures.
- **How a followed move's path reaches the end of its tick** ([risk 19](#risks)): what a transform carries
  and where the legs and their times are kept. WP12's builder chooses, and says in the log what an entity
  pays whose path is one straight leg over the whole tick.
- **The fraction that is the 2D contact tolerance.** WP2's builder measures it on WP2.12's cases and sets
  it, a power of two no larger than 2⁻¹⁶, and the reference says it.
- **Which fixed direction is the normal where no way out is the shallowest.** WP2's builder chooses, and the
  reference says it.
- **How a terrain's height image is laid out, and how fine its cells are.** WP16's builder chooses, over what
  `SIM-3D` gives, and the reference says it.
- **Which sound formats a `MusicAsset` streams from.** `AUDIO`'s planner chooses, and WP18 follows.
- **How an action set is packed into the input frame**: how many presses a button's count holds, above the
  three the design asks for, and how fine a stick is. WP2's builder chooses from what fits, and the reference
  says it.
