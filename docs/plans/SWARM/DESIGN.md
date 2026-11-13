# SWARM: the swarm-survival game itself

13 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the brief](../BRIEF.md). C02, C03, C05, C07, C11 and C13 in [CONTRACTS](../../design/CONTRACTS.md) govern.
[SIM-2D](../SIM-2D/DESIGN.md), [RENDER-2D](../RENDER-2D/DESIGN.md), [GAME-UI](../GAME-UI/DESIGN.md),
[TRANSPORT](../TRANSPORT/DESIGN.md) and [SESSION](../SESSION/DESIGN.md) provide what this module stands on.

## The request

The owner's words weren't passed on with this task; this design answers the lead's task, "Plan module SWARM. It
needs: SIM-2D, RENDER-2D, GAME-UI, TRANSPORT, SESSION", which is row 21 of the module list. What the owner asked
for is on [the page they set on 10 November 2026](../../design/NEXT-GOAL-SWARM-GAME.md), which
[the brief](../BRIEF.md) carries as decision 7:

> - **A swarm survivor.** One or more players hold out against ever-growing crowds of enemies, with automatic
>   attacks and upgrades between waves. It's an original game: its own name, characters, art and rules.
> - **2D support in the engine.** The engine supports 2D games as well as 3D, and this game proves it.
> - **Multiplayer.** Several players play together over the network.

That page also sets defaults, which are an agent's and not the owner's: cooperative online play for 2 to 4
players; top-down 2D with sprites on the Mac, in Chrome and on the iOS simulator; thousands of enemies and
projectiles on screen at once. The module list's brief for this module, which
[its entry](../ARCHITECTURE.md#swarm) now replaces with a pointer here, left to this planner the co-op rules
(shared or separate upgrades, what a fallen player does, whether a choice pauses the run), and three asks of
existing modules: an app runner for a second game on the three targets, a repository guard that admits a second
game, and a world that steps when a session says.

This design was written after reading the five designs above and the code on main: `thrum-app`'s window and page
shells, `examples/v1-game-ios`, `thrum-core`'s world lane and `.config/architecture.toml`. None of the five
modules is built, so what this design says of their code is what their designs promise.

## Purpose and scope

A player holds out against a crowd that grows into the thousands, their attacks firing on their own, and picks
upgrades between waves. Up to three friends join the same run from a Mac, a browser or a phone.

The game exists to prove the engine in a product. It is game content: it lives in `examples/`, and no engine
crate names anything of it.

**Consumers.** The owner, who plays it and judges it. Track 3, which this module closes with a record of what
the five engine modules proved in a real game.

**What exists.** Nothing of the game. `examples/v1-game` is a different game. `thrum-app` runs any game in a Mac
window and on a page; its iOS shell lives in `examples/v1-game-ios` and serves that one game. The repository
guard names one external game.

**Commissioned.** The twelve mechanisms below: one arena, one run of about ten minutes, four heroes, six kinds
of enemy and a boss, six weapons and about twenty upgrades, alone on the three targets and together for two to
four players. The counts are this plan's scope, not the owner's; they are an assumption in the plan.

**Not built.** Sound and music. A gamepad. Saved progress, unlocks or currency between runs. More than one
arena. Difficulty settings. Players against each other. A public host, matchmaking, accounts or a lobby browser.
Text in any language but English. Obstacles and path-finding: the arena is open. A map larger than the arena.
Typed names or chat.

**Words used here.** A **run** is one attempt, from its first wave to a win or a loss. A **wave** is a stretch
of a run in which the crowd attacks. An **intermission** is the stretch between two waves, in which upgrades
are chosen. A **body** is anything the simulation moves or tests for overlap: a hero, an enemy, a projectile or
a pick-up. The **cap** is the most bodies the world may hold. A **hero** is a player's character, and belongs
to a [player slot](../SESSION/DESIGN.md#purpose-and-scope). A slot is **present** when its player is connected
and has a hero. The **team level** is the one level every hero shares. An **owed choice** is an upgrade a hero
has earned and not yet chosen. A **driver** is whatever decides which input frames a tick is stepped with. The
**view** is what one player sees: a rectangle of the arena around their own hero.

## What a player must see and feel

| Cue | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **G1 A crowd that grows into the thousands.** Each wave is larger than the last, and late in a run thousands of enemies and projectiles are in view at once | From the first wave to the last; one to four heroes; a 16:9 view | A large count that is mostly off screen, or reached only in a test scene. Rejected by counting bodies inside the view rectangle of a scripted player's own run |
| **G2 Attacks that fire on their own and visibly land.** Every weapon finds its targets without aim, and a hit, a death and a pick-up each read at a glance | Every weapon, alone and all at once; a packed crowd; the boss | Damage applied to whatever is nearest in a list. Rejected by each weapon's hits compared with a brute-force search |
| **G3 A hero that answers at once.** The hero moves in the frame after the input | Alone, and in a match at A0's profile and the harsh one; keys and a touch stick | Drawing the hero at the confirmed tick in a match. Rejected by comparing input-to-motion frames in a match with those alone |
| **G4 Upgrades between waves that change how the run plays.** | Every upgrade; one to four players choosing at once | A menu that changes a number nothing reads. Rejected by a test for each upgrade that a later tick's outcome differs |
| **G5 A run that can be won and lost.** | Many seeds; each hero | A crowd that cannot kill, or one nobody survives. Rejected by a hero who stands still losing every seed and a scripted player winning some |
| **G6 Readable at a glance.** The player always finds their own hero; enemy kinds, projectiles and pick-ups tell apart at game size; the status display reads over the crowd | The run's peak; 1920×1080 and 3456×2234; Chrome; a phone in both orientations | Sprites judged enlarged on a sheet. Rejected by inspection in the running game at the size shown |
| **G7 Together.** Every player sees the same crowd and the same run. A friend joins mid-run, one drops and comes back, a fallen hero returns, and nobody waits on anybody | Two to four players on any mix of the three targets; A0's profile; a join in any wave | Players who share one process, or who each fight their own crowd. Rejected by each peer's own digests |
| **G8 The same game on three targets.** | Keys and pointer on the Mac and in Chrome; touch on the simulator; a phone's safe area | A phone build that draws and cannot be played. Rejected by real touches that move the hero and make a choice |
| **G9 It survives the lifecycle.** | Resize, a replaced device, a lost focus, a hidden tab, background and foreground, a dropped link | A run that restarts. Rejected by the same run carrying on, by digest |
| **G10 Smooth.** No visible stutter or hitch at the run's peak | The Mac at 1920×1080 and 3456×2234; Chrome at 1920×1080 | Smooth in the first wave. Rejected by real-time capture and frame records at the peak. Times are baselines ([cost](#cost)) |
| **G11 Original, and none of it in the engine.** | Every name, sprite, font and rule; every engine crate | A game type or screen named in an engine crate. Rejected by a question on the code |

The bar is Vampire Survivors, Brotato, Halls of Torment and Deep Rock Galactic: Survivor, and from M2 on the
concept the owner picks in M1. It is judged in words, in the running game.

## Research and selection

- **Waves with upgrades between them is Brotato's shape.** Its run is 20 waves of 20 to 90 seconds, and after
  each one a player selects "upgrades earned from leveling up", then shops, and the next wave starts
  ([Brotato wiki, Waves](https://brotato.wiki.spellsandguns.com/Waves), read 13 November). The owner's words,
  "upgrades between waves", ask for this and not for Vampire Survivors' level-up in the middle of play. So the
  run has no pause inside a wave, which is also what a match needs:
  [the session's ticks never pause](../SESSION/DESIGN.md#4-timed-frames-and-ordered-commands).
- **Online co-op batches choices so that one player never stops the others.** Vampire Survivors' online co-op,
  shipped in October 2025, shows a level-up only "after everyone has leveled up", and its pause menu gives the
  pausing player a moment of invincibility "rather than pausing the game for everyone"
  ([Gematsu, version 1.14](https://www.gematsu.com/2025/10/vampire-survivors-version-1-14-update-now-available-alongside-free-balatro-collaboration-dlc)).
  Its couch co-op brings a fallen player back while a friend survives, at part of their health
  ([God is a Geek](https://godisageek.com/2023/08/vampire-survivors-co-op-guide-how-does-it-work/), from the
  search summary; the page itself refused the fetch). [Mechanism 8](#8-playing-together) takes all three ideas.
- **A spawn budget, not a list of spawns.** The common practice gives each kind of enemy a cost and each moment
  a budget, counted in threat and not in heads, with a curve over time
  ([a 2026 tutorial for Unity 6 and Godot 4](https://gamineai.com/blog/how-to-build-a-reusable-enemy-spawn-director-in-unity-6-and-godot-4-wave-curves-budgets-cooldowns),
  from the search summary). It scales with the number of players by one multiplication.
- **Thousands in view is more than the genre's best-known games draw.** Brotato holds at most 100 enemies at
  once ([wiki, More Enemies](https://brotato.wiki.spellsandguns.com/More_Enemies), from the search summary). The
  brief's default asks for thousands, so enemies here are drawn smaller than in those games;
  [the check below](#check-that-it-can-work) says how small.
- **Enemies that only seek and push apart.** The genre's crowds walk straight at a player and are kept from
  stacking by separation. `SIM-2D`'s reference crowd is exactly this and is costed at 10,000 bodies, so it is
  the selected behaviour, and the arena is open so that nothing needs a path.
- **What would change the selection.** If the owner would rather have level-ups in the middle of a wave, the
  intermission's choice becomes a per-player overlay while play goes on, which is a change to mechanisms 5 and 8
  and not to the engine. If M2 finds the cap in Chrome below the brief's thousands, the builder stops and a
  planner chooses between a smaller crowd for every peer and a lighter neighbour pass.

## Mechanisms

### 1. The concept is the owner's pick, from options made the way the game will be made

The game's name, premise, characters and look are taste, and no reviewer can settle taste for the owner. M1
makes at least three **concepts** that differ in kind: in premise, and in how the art is drawn. Each has a name,
a paragraph of premise, one hero, two enemies, a palette, and one mock arena at 1920×1080 with 2,000 enemies
around the hero at the size [the check below](#check-that-it-can-work) derives.

Each concept's sprites are made by the method the finished game would use for that look, so the owner chooses
from what can be delivered. Art is made in this repository: drawn by a script, or drawn as image files by the
builder, original, with nothing taken from another game. A paid generation service is spending and is the
owner's decision. The mock is composed as the engine draws, premultiplied and in linear light, so soft art
looks as it will in the game.

The pick is recorded as Human Feedback, and the builder writes it into `examples/swarm/CONCEPT.md` with
anything the owner asked to change. That file is the reference every later visual item is compared with, by
the qualities it names and never as a target image.

The game's packages are named `swarm` whatever the game is called. The owner's name for it is content, shown on
the title screen.

### 2. The run is a state machine in the world

Everything about a run is authority state, stepped by rules in one rule package under
[`SIM-2D`'s profile](../SIM-2D/DESIGN.md#mechanisms): which state the run is in, the wave number, the wave's
clock, the team level, each hero's upgrades and owed choices, every body. The states are **gathering**, **wave**,
**intermission**, **won** and **lost**.

A state changes only for two causes: a tick, or an action in an admitted input frame. Screens never change
state by themselves. A screen shows the state it reads from the committed world and sends an action, and the
rule decides. An action that makes no sense in the state it meets (a choice that was not offered, a hero picked
twice, a start during a wave) changes nothing, on every peer alike.

**Actions.** *Pick a hero* (which one), *ready*, *choose an upgrade* (which of the offer) and *leave the run*.
Movement is the frame's two axes. There is no aim and no fire button.

**Gathering.** Each slot picks one of four heroes; two slots may pick the same one. The run starts at the tick
every present slot is ready. No slot has a privilege, so a run needs no host player and a headless host works.

**Won and lost.** The run is won at the tick the last wave's boss dies, and lost at the tick no present hero is
standing. Both show the run's figures and return to gathering when every present slot is ready. The next run's
random streams are keyed by the match seed and a run counter, so each run differs and every peer agrees.

**Why in the world.** A match shares causes, not state, so anything a peer must agree on has to be a function
of the admitted sets. A wave timer kept by a screen would differ on every peer. With the run in the world, a
player who joins mid-run receives it with the baseline, a replay reproduces it, and the digest covers it.

### 3. Waves and the crowd

A run has a fixed number of waves, between eight and twelve, each longer and larger than the one before, and a
whole run is at most 36,000 ticks: ten minutes at the engine's 60 ticks a second, the length `SIM-2D` and
`SESSION` prove. The count and lengths are tuning.

**The budget.** Each kind of enemy has a cost. A rule holds a spawn budget that grows along a curve through the
wave and from wave to wave, and is multiplied by the number of present heroes. While the living enemies' cost is
under the budget, the rule spawns more on a ring outside every present hero's view, choosing kinds and places
with [the profile's random function](../SIM-2D/DESIGN.md#3-random-numbers-keyed-by-seed-tick-stream-and-entity).
The last wave adds one boss.

**The cap.** The world never holds more bodies than the cap, whatever the budget asks. The cap is a constant
of the rule set, the same for every peer, and at most 10,000, the size the engine modules are proved at. M2
measures it first: it is the largest size at which, in Chrome, a tick and a frame's CPU work together take
less than the tick period. Chrome runs both on the page's one thread, and
[a peer that cannot step at the pace of play is dropped](../SESSION/DESIGN.md#10-slow-silent-and-hostile-peers),
so that is the size every peer can hold. M2 measures at each thousand bodies from 2,000 to 10,000 and stops if
no size from 3,000 up qualifies, because 2,000 enemies and projectiles in view need room beside them for
heroes, pick-ups and what is off screen. When the cap is reached spawning waits; pick-ups merge
([mechanism 5](#5-pick-ups-the-team-level-and-the-intermission)) and projectiles have short lives, so the crowd
keeps most of it.

**Enemies** seek the nearest present hero and push apart from their neighbours, through
[`SIM-2D`'s index and queries](../SIM-2D/DESIGN.md#5-the-index-and-its-queries), read at the start of the tick.
Kinds differ in speed, health, size, cost and one behaviour each: a walker, a fast one, a heavy one, one that
shoots, one that splits when it dies, one that charges in a line. The boss is an oversize body, which the index
carries on its oversize list. An enemy that touches a hero damages it at a fixed rate. Health and damage are
integers.

**The arena** is a bounded open field several views wide, with a wall no body crosses. Nothing in it blocks a
path.

### 4. Automatic attacks

A hero's weapons fire on their own clocks, counted in ticks. Each weapon is one of four patterns over the
queries, so every one is a few calls to the index and none needs an angle:

| Pattern | Finds its targets by |
|---|---|
| A shot at the nearest enemies | The nearest-`k` query, then a projectile that flies straight and hits the first enemy its capsule overlaps, in query order |
| A blast around a point | A circle overlap |
| A sweep or a beam | A box or capsule overlap from the hero along its facing |
| Bodies that circle the hero | A circle overlap around each, its offset turned every tick |

**No trigonometry.** The profile has no sine. A direction is a unit vector. It is turned by multiplying with a
constant rotation, whose two numbers are written in the rule set as constants, and renormalised with a square
root, all inside the profile's operations. No angle is stored. Presentation, which is outside the rule package,
may use the platform's maths to turn a sprite.

A projectile is a body with a short life. Damage, piercing and life are integers. A dead enemy despawns and
drops a pick-up.

### 5. Pick-ups, the team level and the intermission

**Pick-ups** sit in an index of their own. One within a hero's reach flies to that hero and is collected. When
pick-ups are many, those close together merge into one of their summed worth, so their count stays bounded.

**One team level.** Every pick-up adds to one pool, whoever collects it, and the pool raises the team level.
Each level gained gives every hero one owed choice. Nobody races a friend for a drop, and a hero who joined
late or was down is never behind.

**The intermission** begins the tick a wave's clock ends: enemies and their shots are cleared, and pick-ups
left on the field fly to the pool. For each owed choice a rule makes that hero an **offer** of three upgrades,
drawn with the random function from what that hero can still take. The offer is world state, so every peer
shows the same one. A player's answer is the *choose an upgrade* action, and the rule applies it once.

The intermission ends at the tick every present slot is ready. While more than one slot is present it also ends
when its clock runs out, and every owed choice left is then made by the rule with the random function, so no
upgrade is lost and nobody waits. A player alone has no clock.

**Upgrades** raise a hero's figures (speed, health, reach, damage, rate), add a weapon or strengthen one. Each
must change what a later tick does; one that only changes a displayed number is a defect.

### 6. Heroes, and what the crowd may do to one

A hero's movement is one function of its own state, the frame's axes and the rule set's constants: the axes
are read through [the engine's input conversion](../SIM-2D/DESIGN.md#the-input-conversion), clamped to unit
length, scaled by the hero's speed, and the position is kept inside the arena. That function is the game's
[predicted controller](../SESSION/DESIGN.md#11-the-predicted-controller), and the movement rule calls it, so
prediction and authority are one piece of code.

**Enemies never move a hero.** They damage it. A crowd that pushed the hero would correct the prediction on
every tick of contact, which is most of a run. Health is not part of the controller, so damage never corrects
it. The controller changes for three causes only: the player's input, an upgrade to speed, and the hero
leaving or entering the field.

The four heroes differ in their first weapon and their starting figures.

### 7. Who admits the frames: alone and in a match

The rules see only admitted input frames, one for each slot and tick, and roster events. Three drivers supply
them, and the rules cannot tell which:

| Driver | Where | It steps |
|---|---|---|
| **Local** | A player alone, on any target | On the world lane's fixed clock, with the one slot's frame built from the controls and the actions the screens sent |
| **Host** | The Mac, in a window with its own player or headless | When [the session's pacing](../SESSION/DESIGN.md#6-the-playout-clock) says, from the frames it admitted |
| **Client** | Any target, joined to a host | When the session says, from the host's admitted set and nothing else |

An action is sent as [a command](../SESSION/DESIGN.md#4-timed-frames-and-ordered-commands) in a match, and
arrives in the slot's frame; alone it is put in the frame directly. The rule reads the frame either way.

**A driven world.** `thrum-app`'s world lane runs a task that advances on a fixed clock and raises an overload
when it falls behind. A host's and a client's world must commit the ticks the session names, none, one or
several, and draw at the session's playout interpolation. That is
[the module list's ask of core](../ARCHITECTURE.md#what-track-3-asks-of-existing-modules), built here under
[seam ruling 4](../ARCHITECTURE.md#seam-rulings-for-swarm): a world may be **driven**. Its driver names how
many ticks to commit at each advance and where the presentation stands between the last two; a driven world
raises no overload for ticks nobody asked for; and a world on the fixed clock behaves as it did.

**Pausing.** Alone, the pause menu pauses the world lane, as `P` does today. In a match the menu opens over a
run that carries on, and the lane's pause is not offered.

**The session lives with the world.** It is driven from the world's task, which drains the link at the barrier
as [`TRANSPORT`'s design](../TRANSPORT/DESIGN.md#4-the-world-lane-never-waits-on-the-network) expects. Held
axes reach it through the lane's control mailbox, and actions through a bounded queue that never coalesces.
What a screen needs of the connection comes back as a small value beside the page.

### 8. Playing together

- **Hosting and joining.** A Mac player chooses *host* and gets a
  [join ticket](../TRANSPORT/DESIGN.md#3-the-hosts-identity-and-the-join-ticket), shown with a Copy button and
  as a link. A friend joins by opening the link (the page's fragment in Chrome, an address the app is opened
  with on the phone) or, on the Mac, by a launch argument or a paste. Nobody types it. Chrome and the phone
  only join.
- **Joining mid-run.** A joined slot picks a hero, and the hero enters at the next tick beside the others,
  briefly unharmed. It holds one owed choice for each team level, taken at the next intermission.
- **Away.** When [the roster](../SESSION/DESIGN.md#2-the-world-knows-slots-the-session-knows-peers) says a slot
  is away, its hero leaves the field at that tick and keeps everything it has. *Back* returns it, as a join
  does. *Left* removes it.
- **Fallen.** A hero whose health is spent is down for the rest of the wave and stands again at the next wave
  with half its health. The run is lost when no present hero is standing.
- **Nobody present.** If every slot is away, the wave's clock and the budget hold until one is back. If every
  slot has left, the run ends and the world returns to gathering.
- **The crowd's size** follows the number of present heroes through the budget, from the tick of each roster
  event.
- **What each player sees.** Their own view, centred on their own hero. The other heroes are marked, and one off
  screen has a pointer at the view's edge.

### 9. What is drawn

**Rows.** The scene source maps bodies to [sprite rows](../RENDER-2D/DESIGN.md#3-sprite-rows): frame animation
by tick, a flash from the tick a body was last hit, a tint for a kind's variants, additive for shots and
blasts. Layers from the back: ground tiles, pick-ups, bodies ordered by height on screen, shots, marks. Damage
numbers, the pointers to other heroes and other marks are frame rows. Everything drawn is a function of the
committed page, the frame's alpha and presentation memory; nothing drawn is ever written to the world.

**The view** shows a fixed span of the arena across the window's shorter side, so every player sees the same
amount of world whatever their window, and the count in view does not depend on it. A common enemy's body is
about a fiftieth of that span ([check](#check-that-it-can-work)).

**Hiding the lead in a match.** A client draws its own hero from the prediction, ahead of the crowd by
[the lead plus the confirmed delay](../SESSION/DESIGN.md#11-the-predicted-controller), nine or ten ticks at A0.
Its weapons fire in the confirmed world, from where the hero was. So:

- what is attached to the hero (circling bodies, a blast centred on it, a muzzle flash) is drawn at the hero
  as drawn;
- a free projectile of the player's own is drawn easing from the drawn hero onto its confirmed path over its
  first ticks;
- a correction is eased over a few frames, and a large one snaps.

All three are presentation memory. Hits are decided by the confirmed world alone. How this feels is the
owner's to judge in M8.

**The game's sheets** are at most eight in the scene and eight in the interface, `RENDER-2D`'s limits.

### 10. Screens

Built from [`GAME-UI`'s nodes](../GAME-UI/DESIGN.md#6-the-tree-and-its-layout): the title, with *play alone*,
*host* and *join*; the hero pick; the status display (the hero's health, the wave and its clock, the team
level and its progress, the enemies alive, and in a match each other hero's state); the intermission's offer of
three cards with the other players' progress and the clock; the pause menu; the join screen, which shows
[`SESSION`'s connection states](../SESSION/DESIGN.md#9-negotiation-and-leaving) and their reasons in words; and
the won and lost screens with the run's figures.

The intermission's offer opens as an exclusive scope, disarmed for a moment, so a movement key held through the
end of a wave neither chooses a card nor is lost. The font is cooked at the sizes the screens use; a size that
was not cooked is refused by the engine, so the screens name their sizes in one place.

On the phone the hero is steered by a stick that appears where a thumb lands outside any control.

### 11. Three targets, one game

| Package | Holds |
|---|---|
| `swarm-rules` | The rule package: components, rules, the controller's function, the action wire, the constants |
| `swarm` | The game: the scene source, the screens, the three drivers, the Mac window, the page, the headless host |
| `swarm-ios` | The iOS app |
| `swarm-proof` | The scripted player, the trace runner for the three targets, the match harness |

They sit under `examples/swarm/`, with the game's sheets and font in `examples/swarm/assets/`. The names are
fixed here because checks name them; where a file sits inside a package is the builder's.

Three things the game needs are inside state other modules own, and are built by this module's builder under
[seam rulings](../ARCHITECTURE.md#seam-rulings-for-swarm):

1. **The repository guard admits a second game** (M2).
2. **The Mac window and the page run a game that brings no sky kit** (M2): whatever `thrum-app`'s shells and
   the capture and packaging scripts still assume of the V1 game or the sky kit.
3. **The iOS shell serves a second game** (M6). The parts of `examples/v1-game-ios` that name nothing of the V1
   game move into `thrum-app`, generic over a game as the Mac shell is, and both apps use them. A second copy
   was not chosen: [two copies of platform policy drift](../PLATFORM-RUNTIME-WEB/DESIGN.md), which is why the
   browser shell was lifted the same way.
4. **A driven world** (M7), as [mechanism 7](#7-who-admits-the-frames-alone-and-in-a-match) says.

### 12. A run replays from its seed and its inputs

The rules are deterministic, so a run alone is its seed and the frame admitted at each tick. The game can
record that as a **trace** and step a trace in place of live input, in a window or with none. One mechanism
then serves five uses: the conformance run on the three targets, captures at named ticks, the scripted
player's runs, the cost records, and a defect a builder can replay.

**The scripted player** is a deterministic policy in `swarm-proof`: it reads the committed page, steers away
from the densest nearby crowd and toward pick-ups, and answers offers by a fixed rule. It is no judge of fun.
It makes runs that reach the last wave, so that the late game can be measured and captured.

### Check that it can work

- **Thousands in view.** In a 16:9 view whose short side is `S`, bodies of diameter `d` that cover a share `c`
  of the view outside a clear ring around the hero number about `2.2 c S² / d²`. At 1920×1080: bodies of 27
  px (`S/40`) need 60% cover for 2,000; bodies of 22 px (`S/50`) reach 2,170 at 40% and 3,260 at 60%
  ([probe](../../evidence/SWARM/planning-probe.md)). So a common enemy's body is about a fiftieth of the short
  side: 22 px at 1080p and 45 px on the built-in display's 2234. A sprite may be drawn taller than its body,
  since rows overlap in height order. `RENDER-2D`'s pixel sampling is exact from one output pixel a texel, so
  an enemy of 12 to 16 texels stays crisp down to a short side of about 700 px, and is drawn softer below.
- **The tick.** `SIM-2D`'s probe stepped 10,000 seeking, separating bodies in 5.0 ms on the Mac and 5.6 ms in
  Chrome. The game adds weapons, pick-ups and the budget, each a few queries a hero, against a 16.7 ms tick
  period. Chrome is the tight target, which is why the cap is measured there before anything is built on it.
- **Rows and order.** `RENDER-2D` is designed and costed for 10,000 rows in one draw with a kept order.
- **A join.** `SESSION` estimates a 10,000-body image at 1.3 MiB; the game's bodies carry more than the
  reference crowd's, so M5 measures its own.
- **The intermission in a match.** The offer is world state and the answer a command admitted at one tick,
  which is the case [`SESSION`'s mechanism 4](../SESSION/DESIGN.md#4-timed-frames-and-ordered-commands) was
  designed for.
- **No angle.** A unit vector multiplied by a constant rotation and renormalised uses multiply, add, divide and
  square root, all exactly rounded under the profile, so every peer turns it identically.

## The system as it runs

| State | Owner | Kind |
|---|---|---|
| The run, the wave, the team level, heroes, enemies, projectiles, pick-ups, offers | The world, stepped by `swarm-rules` | Authority |
| The spatial indexes | `thrum-sim2d`, rebuilt each tick | Derived |
| The match, roster, lead, playout clock, prediction ring | `thrum-network`'s session | Session |
| A link and the host's ticket | `thrum-transport` | Runtime |
| Which screen is open, focus, the stick, the view, flashes, eased positions, damage numbers | `swarm` | Presentation memory |
| Sheets, the font, their cooked products | `examples/swarm/assets/`, cooked by `CONTENT-LOOP` | Content |
| The concept | `examples/swarm/CONCEPT.md` | The owner's pick |
| Traces and their retained digests | `swarm-proof` | Exact comparisons, this module's |

**A tick, walked.** (1) The driver hands the rules each slot's admitted frame and any roster events. (2) Run
rules apply actions and roster events and advance the run's state and clocks. (3) The indexes are built from
the committed bodies. (4) Hero movement calls the controller's function. (5) Enemies steer; the budget spawns.
(6) Weapons fire and query; projectiles move and hit; damage and deaths are staged as integers and structural
edits. (7) Pick-ups fly, merge and are collected; the team level rises. (8) The engine validates and commits
the tick once.

**A frame, walked.** The shell routes events to the interface, then the game. The game turns what it was handed
into axes and actions for the driver. On a new committed tick the scene source extracts rows. The game gives
the view at the frame's alpha, its frame rows and the interface's paint list. The sprite pass draws, then the
interface stage.

**An evening, walked.** A player opens the game on the Mac and hosts; the title shows the ticket and they send
the link. They pick a hero and start alone. A friend opens the link in Chrome, picks a hero during wave three
and appears beside them. At the intermission both choose at once. The friend's tab is hidden for a minute:
their hero leaves the field and returns. In the last wave the host falls, the friend kills the boss, and both
see the run won at the same tick.

## Interfaces

**Provided.** Nothing to an engine module. To the project: the game, one command that starts it, and
[the record that closes track 3](#proof-and-builder-handoff).

**Used,** as [the module list](../ARCHITECTURE.md#interfaces-track-3) names them: the deterministic rules
profile, the 2D queries and the conformance command (`SIM-2D`); sprite rows and sheets (`RENDER-2D`); the
interface tree, event routing, the font product, text drops and the clipboard (`GAME-UI`); the join ticket, the
socket adapters and the relay (`TRANSPORT`); the session, the connection state, the predicted controller and
the match proof (`SESSION`); the app runner and typed controls (`CREATOR`); engine capture (`T2-CAPTURE`).

**Asked of other modules.** The four [seam rulings](../ARCHITECTURE.md#seam-rulings-for-swarm) of
[mechanism 11](#11-three-targets-one-game). No milestone is added to another module.

**Contracts.** No contract's text changes. C13's rule-set addition is `SESSION`'s, and this game is its first
user: it declares its rule set, its action wire and which actions are commands.

**What the game asks nobody for.** More than eight sheets, more font sizes than it cooks, an angle in the
profile, or blending in display space. If the chosen concept needs one, the builder writes the proposal in the
log and a planner settles it with the owning module.

## Cost

No budget is set (brief, decision 3). The account, at the cap and four players:

| Work | Count | From |
|---|---|---|
| The tick: two or three index builds, the neighbour pass, weapons, pick-ups | 60 a second on every peer | 5.0 ms on the Mac and 5.6 ms in Chrome for the neighbour pass at 10,000 (`SIM-2D`'s probe); the rest not measured |
| Extracting, ordering and uploading rows | Each tick, up to the cap | `RENDER-2D`'s account: about 64 bytes a row, 121 µs to order 10,000 |
| Drawing | One draw of the scene and one of the interface | `RENDER-2D` and `GAME-UI`; a crowd packed in view shades about its own area several times |
| The interface | About 200 rows for the status display, 600 for an offer | `GAME-UI`'s account |
| A match, each way | One datagram a tick | `SESSION`'s account: 6.5 to 19 KB/s down, 7.6 to 11.3 KB/s up |
| A join | One world image, then a catch-up | Not measured for the game's bodies; M5 records it |

The cost that matters is Chrome's one thread, which steps and draws. The design meets it by measuring the cap
there first, by keeping enemies' behaviour to one neighbour pass, and by merging pick-ups.

**Baselines and tripwires.** Every time and byte count recorded is a baseline, and a cost command succeeds
when its record is complete and made at the claimed commit. Flagged in the record and investigated while
building goes on: a median tick longer than the tick period on any target at the cap; a frame whose CPU and GPU
time together exceed the display's refresh interval on the Mac at the run's peak; a row about a quarter worse
than the last comparable one; a join that takes longer than an intermission.

**Two figures decide a result.** The cap, which M2 sets from its own measurement and which must be at least
the size G1 needs. And [`SESSION`'s floor](../SESSION/DESIGN.md#cost): in a match, at least 95% of ticks admit
each player's own frame.

Whether motion is smooth on the built-in panel itself is not measured: it needs an attended run, and is named
in the closing record as not run.

## Risks

1. **The cap in Chrome is below the brief's thousands.** Early sign: M2.1. The builder stops; the choices are
   under [research](#research-and-selection).
2. **Small enemies do not read.** Early sign: M1's mock arenas, which show them at game size before any game
   exists. If no concept reads at a fiftieth of the short side, the owner is told with the options, and trades
   count for size.
3. **Art made by an agent falls short of the bar.** Early sign: M1's options, which are made the same way.
   A paid service is the owner's decision.
4. **A convincing fake of G1:** bodies outside the view. The count is taken inside the view rectangle.
5. **A convincing fake of G7:** peers in one process. Digests come from each peer's own record, as `SESSION`'s
   do.
6. **A convincing fake of G5:** a scripted player tuned to one seed. Twenty seeds, each hero.
7. **A rule reads something outside the profile,** such as a screen's clock or an unconverted axis. Early sign:
   the conformance run's wire route, and M5's match.
8. **The hero's lead over the crowd feels wrong.** Early sign: M7's captures. The three easings of mechanism 9
   are presentation only and can be retuned without touching a rule.
9. **Lifting the iOS shell breaks the V1 app.** Early sign: M6.3, which runs the simulator checks the other
   modules left.
10. **The engine modules land differently from their designs.** Early sign: each milestone's first reading.
    The builder writes the difference in the log, and ends `BLOCKED` if a mechanism here cannot stand.
11. **The simulator cannot keep the pace of a match at the cap.** It proves function only, and a peer too slow
    is dropped by design. M8 records what happened; a slow simulator is no defect.
12. **Balance is tuned for the scripted player and not for a person.** Only the owner's play in M8 settles
    it.

## Proof and builder handoff

- **Targets.** `mac-metal`, `chrome-webgpu` and `ios-sim`, the last for function only. The host is always
  native on the Mac.
- **Order between modules.** M1 waits for `SHOWCASE:M8`, as all of track 3 does. M2 waits for `SIM-2D:M3`;
  for `SESSION:M1`, after which every rule package supplies both halves of the state encoding, so the game's
  components are written once; and for `RENDER-2D:M5` and `GAME-UI:M5`, whose builders work in the shells and
  the sprite files until then. M5 waits for `SESSION:M4`, which delivers the predicted controller on the
  in-memory link. M7 waits for `SESSION:M6`, its last, which implies `TRANSPORT:M2`.
- **One command.** `scripts/swarm.py`, with the scenario as its argument, runs every proof that needs a window,
  Chrome or a socket, and `scripts/swarm_sim.py` those that need the simulator's screen. It reuses `SIM-2D`'s conformance command and `SESSION`'s match harness and
  relay, and copies neither.
- **The sandbox.** Chrome, the simulator and sockets do not work inside Codex's `workspace-write` sandbox. A
  builder or a reviewer asks to run the command outside it, one request for each run, and nothing in
  `./dev check` opens a socket, a browser or the simulator. A reviewer whose request is refused cannot judge
  those items, and the review goes to Claude.
  [Whether to keep asking for each run is the owner's](../ARCHITECTURE.md#owner-decisions).
- **Events in proofs.** As [`GAME-UI`'s](../GAME-UI/DESIGN.md#proof-and-builder-handoff): real in Chrome and on
  the simulator, and on the Mac read from a script and entered where the window's own events enter.
- **Capture.** Inspected items use T2 engine capture and name their mode. Stills are `fixed` captures of a
  trace at named ticks. Motion is `realtime` at 1920×1080, judged as consecutive frames at 1x. In a match each
  peer has its own capture.
- **Exact comparisons.** The game's traces and their retained digests are this module's, under
  [rule 1 of exact baselines](../ARCHITECTURE.md#exact-baselines). A change to a rule refreshes them on
  purpose, with a line in the log. No image baseline is registered.
- **Exact baselines.** This module edits `crates/`, `scripts/`, the workspace manifest and the lockfile, which
  are covered paths, and moves no retained image. Every milestone but M1 carries the verifier.
- **GPU.** Every check that draws holds the `gpu` lock. Scratch goes in `tmp/swarm-<milestone>/`; selected
  evidence in `docs/evidence/SWARM/`.
- **The closing record,** `docs/evidence/SWARM/TRACK-3-RECORD.md`, written in M8: what each of the five engine
  modules proved in the game and on which targets, and what was not run.
- **Stopping.** If peers cannot be held equal with the game's rules, the builder keeps the first differing
  tick, the component and both digests, and ends `BLOCKED`.

## Limits on every milestone

Each has its reason beside it.

- No engine crate names a type, constant, string or screen of this game, because the engine is reusable and
  the game is its proof.
- `swarm-rules` passes `SIM-2D`'s guard with nothing suppressed, and uses no platform maths, clock or unordered
  container, because one such use makes peers disagree.
- Presentation never writes the world, and a rule never reads presentation, for the same reason.
- If your change fails an exact test, a baseline or a check another module owns, do not edit it. Write the
  proposal in the log and end `BLOCKED`; [exact baselines](../ARCHITECTURE.md#exact-baselines) rule 1 makes it
  the owner's.
- If the code a module left cannot carry a mechanism here, write the difference in the log and end `BLOCKED`.
  Do not build a second sprite path, interface, session or shell beside the engine's, because each would be a
  second owner of state that has one.
- Every name, sprite and rule is original. Art and fonts are your own, public domain, CC0 or Apache-2.0, with
  the licence text beside them and each file's source listed. Take nothing from another game or from
  Broadsides.
- No time or byte count stops the build (brief, decision 3). Record a row that trips a tripwire as measured,
  note what you found in the log, and keep building.
- Record only runs of the commit being claimed, with the load on the machine.
- If a launch outside the sandbox is refused, leave the item open and say so; a Mac result does not stand in
  for Chrome or the simulator.
- No check waits on a person. Only M1.4 and M8.12 do.
- Run the exact-baseline verifier before `land`, and again when `land` refuses a rebased commit for lack of a
  proof.

## Open questions

| Question | Settled by |
|---|---|
| What is the cap in Chrome? | M2.1 |
| Do enemies a fiftieth of the short side read, in each concept? | M1's mock arenas, then M4's inspection |
| How large is the game's world image, and how long is a join at the cap? | M5's cost record |
| Does the hero's lead over the crowd feel right with the three easings? | M7's captures; the owner in M8 |
| Is the run fun, and its difficulty right for a person? | The owner in M8 |
| How does the game look on an EDR panel, and is it smooth on the built-in panel itself? | Not judged here; named as not run in the closing record |
