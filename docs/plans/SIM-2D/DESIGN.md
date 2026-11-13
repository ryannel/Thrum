# SIM-2D: a crowd of thousands that steps the same on every device

13 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the brief](../BRIEF.md). C01, C02, C03 and C13 in [CONTRACTS](../../design/CONTRACTS.md),
[the G2 decision](../../design/decisions/G2-GENERIC-SIMULATION.md) and
[the input wire](../../design/decisions/A5-INPUT-WIRE.md) govern.

## The request

The owner's words weren't passed on with this task; this design answers the lead's task, "Plan module SIM-2D. It
needs: SHOWCASE", which is row 16 of the module list. What the owner asked for is on
[the page they set on 10 November 2026](../../design/NEXT-GOAL-SWARM-GAME.md), which [the brief](../BRIEF.md) carries
as decision 7:

> - **A swarm survivor.** One or more players hold out against ever-growing crowds of enemies, with automatic
>   attacks and upgrades between waves. It's an original game: its own name, characters, art and rules.
> - **2D support in the engine.** The engine supports 2D games as well as 3D, and this game proves it.
> - **Multiplayer.** Several players play together over the network.

That page also sets a default, which is an agent's and not the owner's: "Thousands of enemies and projectiles on
screen at once". The module list turned the page into six modules and settled that
[the crowd is shared by causes, not by state](../ARCHITECTURE.md#mechanisms-settled-here): every peer steps the same
rules from the same inputs, and no enemy's position travels. That only works if the rules give the same bytes on
every device. This module is what makes them do so, and what lets a game find neighbours and hits among thousands
of bodies each tick. The module list's brief for this module, which [its entry](../ARCHITECTURE.md#sim-2d) now replaces
with a pointer here, left two choices to this planner: fixed point or restricted floats, and whether the queries
sit in `thrum-core` or beside it.

This design reconciles the code on main; it is not a blind study. Read for it: `thrum-core`'s `sim` module (the
runtime, the registered world, history, input), `examples/v1-game/src/simulation.rs` and its types, the contracts
above, the networking design and [a planning probe](../../evidence/SIM-2D/planning-probe.md) run for it.

## Purpose and scope

A game writes its rules once and every device that steps them from the same inputs holds the same world, tick
for tick. The game can ask "who is near this point" among 10,000 bodies and get the same answer, in the same
order, everywhere.

**Consumers.** `SWARM` writes its rules under the profile and uses the queries. `SESSION` compares peers by the
digest and pins the profile's revision in a match. `RENDER-2D` and `SESSION` reuse the reference crowd as their
diagnostic scene.

**What exists.** One atomic tick in canonical slot order, sparse undo, corrected replay and serial-against-parallel
equality on the Mac (G2S). The V1 game's rules use `f64` and the platform's maths freely, and its state hash goes
through `std::hash`, which hashes `usize` at the machine's width and type identities that differ per process, so it
cannot be compared between a Mac and a browser. There are no spatial queries.

**Commissioned.** The six mechanisms below, proved by a reference crowd on `mac-metal`, `chrome-webgpu` and
`ios-sim`.

**Not built.** Rigid-body physics (stacking, joints, rotation, friction); shapes other than circles as bodies;
navigation or path-finding; trigonometry in the profile; a baseline or join image (that is `SESSION`'s); any
change to the V1 game's rules, which stay outside the profile; drawing.

## What a consumer must be able to do

| Need | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **N1 Step the same state on every target** | 0 to 10,000 bodies; 36,000 ticks (ten minutes of play) at 10,000; spawn and despawn churn with slots reused; rewinds of up to 8 ticks with a corrected input; debug and release builds | Equality between the Mac and the simulator only, which share a processor and a maths library. Rejected by the Wasm row and by [a control](#6-the-check-that-enforces-the-profile) that must diverge there |
| **N2 Compare two worlds by one value** | Every committed tick; worlds built in different processes; a world that rewound against one that did not | A hash of positions alone. Rejected by changing each covered value in turn and requiring the digest to change |
| **N3 Be told when a rule breaks the profile** | The named classes in [mechanism 6](#6-the-check-that-enforces-the-profile) | A lint that passes because it was never switched on for the package. Rejected by one fixture per class that must fail |
| **N4 Find overlaps, neighbours and the nearest body** | Radii from 0 to any finite size; from an empty index to 10,000 bodies packed three deep; anywhere inside the extent; from serial and parallel rules | An index that returns the right set in an order that depends on its internals. Rejected by comparing the whole visit sequence with a brute-force search |
| **N5 Draw random numbers that need no saved state** | Any tick, entity and stream, in any order, on any thread, before or after a rewind | A generator with a stored cursor, which a rewind or a different thread order moves. Rejected by the rewinding trace |
| **N6 Know what a tick costs** | 2,000, 5,000 and 10,000 bodies, on the Mac and in Chrome | A figure for bodies spread thinly. The record names the neighbours visited per tick beside each time |

## Mechanisms

### 1. The profile number: `f32` held to exactly rounded operations

**Decision.** A rule computes with one engine-owned number type, here called the profile number. It is an `f32`
inside, and its only operations are the ones IEEE 754 defines as exactly rounded or exact: add, subtract, multiply,
divide, square root, negate, absolute value, comparison, floor, ceiling, truncate, round, conversion from and to
integers, and minimum, maximum and clamp written as comparisons. A 2-vector of profile numbers comes with it. It
offers no sine, power, fused multiply-add, sign-bit read, raw bits or total ordering, because those are either
left to the platform's maths library or can observe a NaN's sign.

**Why this gives equal bytes.** Rust documents these operations as following IEEE 754-2008 with one exception: the
sign and payload of a NaN are not fixed ([`f32`](https://doc.rust-lang.org/std/primitive.f32.html),
[RFC 3514](https://rust-lang.github.io/rfcs/3514-float-semantics.html)). `sqrt` is "guaranteed to be the rounded
infinite-precision result". WebAssembly allows only "limited, local, nondeterminism", and for these operators
the one case it lists is again the bits of a NaN
([nondeterminism](https://github.com/WebAssembly/design/blob/master/Nondeterminism.md)). Rust does not turn
`a*b + c` into a fused operation, because that would break the IEEE guarantee, and these types have no fast-math
mode. So the only values that can differ are NaNs, and:

- no operation of the profile number can read a NaN's sign or payload. A comparison with a NaN is false on every
  platform, and conversion of a NaN to an integer is zero;
- **a tick whose candidate would store a non-finite profile number is refused before commit**, with an error that
  names the component and slot, and state is unchanged. Whether a result is infinite or NaN is fixed by IEEE 754,
  so every peer refuses the same tick. It is a fault of the rule set, not a recoverable event.

Two groups of functions that look harmless are outside the type for a documented reason. `f32::max`, `min` and `clamp` may
return either zero "non-deterministically" when the inputs are `+0.0` and `-0.0`. `sin`, `powf` and `powi` have
precision that "varies by platform". The profile's own minimum and maximum are a comparison and a choice, so they
are exact.

**Evidence.** Box2D 3.1 holds cross-platform determinism on x64 and ARM across three compilers with floats, by
avoiding fast-math, disabling fused multiply-add and writing its own trigonometry, at "no noticeable performance
impact" ([Catto, 2024](https://box2d.org/posts/2024/08/determinism/)). Rapier's `enhanced-determinism` gives
the same results on platforms that "strictly comply to the IEEE 754-2008 floating-points standard", which
covers "most modern mainstream processors as well as WASM targets"
([guide](https://rapier.rs/docs/user_guides/rust/determinism/)). [The planning probe](../../evidence/SIM-2D/planning-probe.md)
ran a crowd of 2,000, 5,000 and 10,000 in exactly these operations for 3,600 ticks with churn: the digests were
equal on the Mac (debug and release), the iPhone simulator and Wasm in Node and Chrome 157. The same crowd with
the platform's `sin` or `powf` in one rule diverged between the Mac and Wasm.

**Alternative: fixed point.** Photon Quantum computes in Q48.16 integers
([manual](https://doc.photonengine.com/quantum/v3/manual/quantum-ecs/fixed-point)), which is exact on any
processor whatever the compiler does. It was built for C# runtimes, where float behaviour is not the language's
promise. Here it would add overflow rules for every multiply, a slower divide and square root, and a conversion at
every page read, to buy a guarantee the language and both platforms already give. Not chosen.

**Range.** A profile number keeps a resolution of 2⁻¹⁰ units or finer for magnitudes below 16,384 (an `f32` has 24
significant bits). With one unit as one metre (C01), that is a millimetre across a 32 km square. Positions beyond
it still step identically on every device; they only lose resolution. An endless map needs an origin that moves, which is not built.

**What would change the decision.** If M1's three-target run finds a difference that cannot be removed inside
these operations, the builder stops ([handoff](#proof-and-builder-handoff)). The profile number's inside is private
to `thrum-core`, so it can become fixed point without a rule package changing; page readers would gain a
conversion. If that fails too, [the module list's fallback](../ARCHITECTURE.md#mechanisms-settled-here) applies.

### 2. Everything else a rule may depend on

A rule's result may depend only on the committed state X[n], the admitted input frames for the tick, and the
rule set's constants. So the profile also fixes:

- **Order.** Rules that are order-sensitive run in canonical slot order, as the engine already does. A rule that
  sums profile numbers over neighbours does so in [the query's stated order](#5-the-index-and-its-queries).
  A sort's key is total, with the slot as its last term.
- **Width.** State holds fixed-width integers, booleans, profile numbers and arrays and structs of them. It holds
  no `usize`, pointer, raw `f32` or `f64`: [the state encoding](#4-the-world-digest) has no form for them, so such
  a component does not compile into a rule set.
- **No outside facts.** No clock, thread identity, environment, address or hash-map iteration order.
- **Input.** A rule reads an input frame's axes only through one engine conversion, which snaps each axis to the
  input wire's 16-bit grid by the wire's own rounding rule and then converts it exactly. A frame made on this
  device and its copy relayed from the host therefore give the same profile numbers.

### 3. Random numbers keyed by seed, tick, stream and entity

One engine function maps (match seed, tick, a caller's stream tag, an entity's stable identity) to 64 bits with
integer operations only, and a second turns the top 24 bits into a profile number in [0, 1) exactly. It is a
published counter-based generator (the family of
[Salmon et al., 2011](https://users.cs.utah.edu/~hari/teaching/bigdata/random123sc11.pdf)); the builder picks one
and keeps its test vectors. Nothing is stored, so nothing is rewound, saved or digested, and two threads asking in
either order get the same numbers.

### 4. The world digest

**What it is.** One 64-bit value for the committed endpoint, computed on request. Two worlds with equal digests
hold the same authority state, to within the chance of a 64-bit collision; the purpose is to catch an accident,
not an adversary.

**What it covers.** Everything G2S's corrected-replay equality covers, which is everything a later tick's result
can depend on: the tick; each slot's live flag, generation, stable identity and structural tick; the free-slot
list in its order; each registered component's presence, value and change tick; the identity index and the row
directory; and the game's own authority resources, including held controls.

**What it leaves out,** because it is local by contract: the world epoch, the timeline and revision counters that
a rewind moves, page and lease state, and process type identities (schema IDs stand in for them). Input frames
admitted for a tick not yet committed are not part of X[n] and are left out.

**Encoding.** Each component and resource type supplies its bytes through one trait: fixed-width little-endian
integers, one byte for a boolean, a profile number's bits, lengths as 64-bit, components in schema-ID order, slots
ascending. The mixing function is the engine's own, integer only, with test vectors.

**Diagnosis.** The same pass can report one partial digest per component, so two diverged worlds can be narrowed
to a tick and a component.

**For `SESSION`.** A baseline that restores less than the digest covers will never match it. The probe's digest
cost 0.18 ms at 10,000 bodies; M1 records the engine's, and `SESSION` chooses how often to ask.

### 5. The index and its queries

**The index** is a uniform grid of circles, rebuilt from scratch when the game asks, normally once at the start
of a tick from X[n] (C03: "Pre-step queries name X[n]"). The game feeds it (slot, centre, radius) in ascending
slot order from whichever of its components hold them; the engine names no game component. A counting sort puts
bodies into cells, so a build is linear in the body count and allocates nothing once warm. It needs no declared
arena: cells are found by hashing their coordinates. A game may keep several indexes, one for enemies and one for
pick-ups, in place of category masks.

This is the loose uniform grid of particle simulation: each body in the one cell that holds its centre, bodies
sorted by cell, and the whole structure "generated from scratch each time step", so its cost does not depend on
how the bodies moved ([Green, 2010](https://developer.download.nvidia.com/assets/cuda/files/particles.pdf), pages
5 and 6). A body whose radius is
larger than the cell goes on a short **oversize list** that every query also tests, so one boss does not widen
every query's search.

**It is derived state.** It is not in the undo journal or the digest. After a rewind the next tick rebuilds it as
any tick does.

**Queries,** all read-only and callable from parallel rules:

| Query | Returns | Order |
|---|---|---|
| Overlap with a circle, an axis-aligned box, or a capsule (a circle swept along a segment, for a fast projectile) | Every indexed circle that touches or intersects the shape | Oversize bodies by ascending slot; then cells by ascending row, then column; then ascending slot within a cell. A body's cell is the one holding its centre |
| Nearest `k` to a point, within a maximum distance | Up to `k` bodies | Ascending squared distance between centres; ties by ascending slot |

"Neighbours of a body" is the circle overlap around it, with the caller skipping its own slot.

**The order is part of the rule set.** It depends on the cell size and on nothing else: not the hash table's size,
the thread count or the history. The cell size is therefore a constant of the rule set, the same on every peer,
and never a quality setting. Handing results back in slot order would hide the cell size, and the probe measured
that at 25% to 37% more time per tick, so it was not chosen.

**The oracle.** A brute-force search over the same list with the same intersection predicates, sorted by the
stated key, must give the identical sequence. The predicates are shared on purpose: the oracle tests which
candidates the index finds and in what order, and the predicates have their own tests on hand-worked cases.

### 6. The check that enforces the profile

Two parts, because each misses what the other catches.

**Before a rule runs.** Rules live in a **rule package**: a package registered as one in
`.config/architecture.toml`, holding rules and nothing that draws. The repository guard, inside `./dev check`,
requires that it depends only on `thrum-core`, `thrum-sim2d`, other rule packages and an allow-list of outside
crates that starts empty; that it carries the profile's Clippy configuration, of which there is one source; and
that it suppresses none of it. That configuration refuses these classes, and the check is judged against this list
and not against "any violation":

1. arithmetic on a raw `f32` or `f64`, and any call to their methods;
2. `HashMap`, `HashSet` and the other containers whose iteration order is unspecified;
3. clocks, threads and the process environment;
4. a dependency outside the allow-list.

A fifth class, a `usize`, pointer or raw float in state, is refused by the compiler through mechanism 2. Clippy can
do the first three today: the probe's fixture was refused for `f32::sin`, `f32::max`, a raw multiply and
`HashMap`.

**When it runs.** The **conformance run** steps a rule package's recorded trace on a target, takes the digest of
every tick, and compares their fold and a checkpoint digest every so many ticks with retained ones. It is one
command with a target argument, and it takes any registered rule package, so `SWARM` runs its own rules through
it. It catches what no lint can: a sort with an incomplete key, a
defect in the engine's own tick, a compiler or browser that breaks its promise.

**The control.** A copy of the reference crowd with the platform's sine in one rule must produce digests in Chrome
that differ from the Mac's. If it ever stops diverging, the control is replaced, not removed.

### 7. The reference crowd

A rule package in `examples/`, never an engine crate. The programs that run it on each target are a separate
package, because a rule package holds rules and nothing else. Four scripted players move by input frames from
the trace. Enemies chase the nearest player and push apart from neighbours; players fire at the nearest enemy; a projectile
hits the first overlapping enemy in query order; enemies lose integer health, die and despawn; new ones spawn on
a ring to hold the count. It steps through `thrum_core::sim`'s registered runtime, with spawns and despawns as
structural edits and history recorded, and its trace schedules rewinds of up to 8 ticks, each replayed with one
corrected input frame.

It has two retained traces. **Crowd** (M1): radii no larger than a cell, at 10,000 bodies for 36,000 ticks and at
2,000 and 5,000 for 3,600 ticks each. **Mixed** (M2): the same sizes and lengths, with oversize bodies and
projectiles fast enough to cross an enemy in one tick. A debug build is far slower, so it runs the 2,000-body
traces only. Once retained, a trace's digests are an exact comparison owned by this module under
[rule 1 of exact baselines](../ARCHITECTURE.md#exact-baselines).

## The system as it runs

| State | Owner | Kind |
|---|---|---|
| The profile number, the random function, the state encoding, the digest, the refusal of non-finite values | `thrum-core` (this module) | Engine code; no stored state |
| The index and its queries | `thrum-sim2d`, a new crate that depends on `thrum-core` only (this module) | Derived, rebuilt each tick |
| The profile's Clippy configuration, the rule-package guard, the conformance command | `.config/`, `scripts/` (this module) | Tooling |
| The reference crowd (package `crowd-2d`), its traces and retained digests; its runners for the three targets in a package beside it | `examples/crowd-2d` (this module) | Game content |
| The tick, committed state, undo, pages, input admission | `thrum-core`, as G1 and G2S left it | Authority |

The queries sit beside `thrum-core`, not in it: they are scene queries (P3), not world state (W1), and a game
with no 2D crowd should not compile them.

**A tick, walked.** (1) The game admits the tick's input frames. (2) A rule registered early feeds X[n]'s circles
to the index. (3) Steering rules read X[n] and the index, in parallel chunks where the game asks, and stage new
velocities. (4) Hit rules query the index and stage integer damage and structural edits. (5) The engine validates
the candidate, refuses it whole if a stored profile number is non-finite, and otherwise applies it in stable
order, records undo and advances the tick once. (6) On request, the digest of X[n+1] is computed. A profile number
is laid out as an `f32`, so pages and sprite rows read positions without conversion.

**A rewind.** Undo restores X[n−k]; the timeline moves and the digest does not notice. Each replayed tick
rebuilds the index and draws the same random numbers, because both are functions of state and tick.

**A game's journey.** Register a rule package; write rules with profile numbers; record a trace; run the
conformance command on the three targets and keep the digests. Change a rule: the retained digests no longer
match on any target, and the game's owner refreshes them on purpose.

## Interfaces

Provided to other modules, as [the module list](../ARCHITECTURE.md#interfaces-track-3) names them:

| Interface | To | What it is |
|---|---|---|
| **Deterministic rules profile** | `SWARM`, `SESSION` | Mechanisms 1 to 3 and 6: the profile number, the fixed dependencies, the random function, the rule-package check and the conformance command |
| **World digest** | `SESSION` | Mechanism 4, with its coverage list, which a baseline must restore in full |
| **2D queries** | `SWARM` | Mechanism 5, with its order |
| **Reference crowd** | `RENDER-2D`, `SESSION` | Mechanism 7, as a diagnostic scene |

Results are promised equal across devices for one build of one rule set, which is what `SESSION`'s rule-set
identity pins. They are not promised across engine versions, so the digest's mixing function, the random function
and the query order may change between builds.

**No contract changes.** C02 and C13 promise no bit identity across platforms; a rule package earns it for itself.
The rule-set addition to C13 is `SESSION`'s proposal.

## Cost

No budget is set (brief, decision 3). The account, from [the probe](../../evidence/SIM-2D/planning-probe.md), which
is a single-threaded stand-in and not the engine:

| Work, per tick at 10,000 bodies | Probe |
|---|---|
| Index build | 0.05 ms |
| Rules, almost all of it the neighbour pass over about 144,000 accepted neighbours | 5.0 ms on the Mac, 5.6 ms as Wasm in Chrome |
| Digest | 0.18 ms |

The engine's own tick and page cost 61 µs at 10,000 simple movers (G2). So the neighbour pass is the cost, it grows
with how tightly the crowd packs, and Chrome pays it on the page's one thread. Two consequences are designed in:
queries are read-only so the Mac runs them in parallel chunks, and every cost row records neighbours visited
beside its time, so a slower row can be told from a denser crowd.

**For `SESSION`:** replaying a tick costs a tick. Eight ticks of this crowd at 10,000 would be about 40 ms in the
probe, which does not fit a frame. M1 records the engine's figure.

**Tripwires.** A median tick at 10,000 bodies that reaches the tick's own period of 16.7 ms cannot keep up and
fails the cost command. Later rows about a quarter slower than M1's at the same neighbour count are investigated.

## Risks

- **The engine's own tick differs between targets**, for example through a width-dependent order. Early sign: M1's
  first Wasm run differs before any rule is suspect. The partial digests name the component.
- **A convincing fake of N1:** digests compared within one process, or only at the final tick. The retained file
  holds checkpoints through the run, and each target is a separate process on a separate runtime.
- **The simulator row adds little.** It shares the Mac's processor and maths library, so it proves that the rules
  build, link and run there, and nothing about a different processor. A physical iPhone, an x86 PC and Android are
  not run; the profile is designed to hold there and that stays unproved.
- **A dense crowd in Chrome crowds the frame.** Early sign: M1's Chrome row near the tripwire. Then the game's
  rules cap density or crowd size for every peer, because one match has one crowd; the engine never thins a crowd
  on one device, which would change the result there.
- **A divergence lands unnoticed later.** The Mac slice of the crowd trace runs in `./dev check`, so any change to
  the result on the Mac fails at once. A change that only Wasm sees is caught at the next conformance run. If that
  ever happens, the Node run of the slice joins the test command.

## Proof and builder handoff

- **Targets.** The conformance command runs natively on the Mac, as Wasm in Chrome, and as a plain executable in
  the iPhone 18 Pro simulator through `simctl spawn`, which the probe confirmed works without an app bundle. None
  needs the GPU. Chrome and the simulator start outside the Codex sandbox, by a request for each run.
- **Retained digests** are made on the Mac by a straight run that has the corrected inputs from the start and
  never rewinds. Every target, rewinding, must then reproduce them. That is cross-target equality and G2S's
  corrected-replay equality in one comparison.
- **Stopping.** If the three targets cannot be made equal inside the profile, the builder keeps the first
  differing tick, the component and both values, and ends `BLOCKED`; a planner then chooses between fixed point
  and the fallback.
- **The V1 game** stays outside the profile. Its rules, tests and exact comparisons do not change.
- **Exact baselines.** This module edits `crates/` and `scripts/`, which are covered paths, and moves no retained
  image. Each milestone carries the verifier.

## Open questions

- Whether the neighbour pass at 10,000 fits beside drawing in Chrome. Settled by M1's and M2's cost rows.
- Whether a game needs an angle or a rotation inside a rule. None is built; if `SWARM` needs one, it is added to
  the profile number as a function written from its own operations, with vectors on the three targets.
