# SIM-3D: 3D collision queries and a character move that agree on every device

27 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-7-the-game-sdk) · [the vision and scope document](../VISION.md). C01, C02,
C03 and C13 in [CONTRACTS](../../design/CONTRACTS.md), [the rules profile](../SIM-2D/DESIGN.md#mechanisms) and
[what the game SDK asks](../GAME-SDK/DESIGN.md#3d-space-and-a-kinematic-character) govern.

## The request

The owner asked for the game SDK on 25 November 2026: "a high end, intuitive, expressive, readable, SDK that game
developers can use to build new games". Its design put eleven scope questions to the owner. The first was "3D
collision queries and the 3D character move in the first release?", with the planner's recommendation: "yes, as
queries and a kinematic move only, with no rigid bodies". On 27 November 2026, asked "Approve the core and take
the recommended scope answers?", the owner said: "I approve the sdk plan."

The lead's task for this design: "Plan module SIM-3D. It needs: SIM-2D WP3. It serves the approved Game SDK
(owner approved 27 Nov; scope question 1: 3D collision queries and a kinematic character move, no rigid bodies,
deterministic for input-only networking)". [The module list](../ARCHITECTURE.md#track-7-the-game-sdk) gives
the module's row, what it owns and the interface the SDK asks of it, and leaves the index and the tests to this
planner, "from the field's current practice".

This design is not a blind study. Read for it: `thrum-core`'s `profile` module, `thrum-sim2d`, `SIM-2D`'s
conformance and profile scripts, the game SDK's design, the tracked meshes under `examples/`, and
[a planning probe](../../../.orchestrate/evidence/SIM-3D/planning-probe.md) run for it.

## Purpose and scope

A 3D game's rules ask what is near a point, what a ray meets, where a moving shape is stopped and how high the
ground is, and they move a character over hills, steps and solid models. Every device that steps those rules
from the same inputs gets the same answers, bit for bit, so a match shares its world by input alone and a
player's own character can be run ahead against the ground it stands on.

**Consumer.** The game SDK, in [`GAME-SDK` WP16 and WP17](../GAME-SDK/DESIGN.md#the-work-packages): its
`Space3d` is this module's collider index, one for each layer, its `Level3d` is this module's fixed collision,
and its `move_shape` is this module's kinematic move. Every 3D game reaches the module through the SDK.

**What exists.** [The rules profile](../SIM-2D/DESIGN.md#mechanisms): one number type, `Number`, whose
operations are exactly rounded, proved equal on the Mac, in Chrome and on the iPhone simulator, with a guard
that holds a registered rule package to it. `thrum-sim2d` indexes circles on a plane. Nothing answers a
question in three dimensions.

**Commissioned.** The seven mechanisms below, in a new crate, `thrum-sim3d`, which depends on `thrum-core`
alone.

**Not built**, each a limit on what a builder may add:

- **No rigid bodies** (the owner's answer to question 1). Colliders detect and never push: nothing here has
  mass, velocity, stacking, joints or friction, and no shape turns by itself.
- **No world state.** Positions stay `thrum-core`'s. Everything here is a pure function of its arguments or is
  derived state, rebuilt from them.
- **No change to `thrum-core`, `thrum-sim2d`, the profile's lints or `SIM-2D`'s scripts.** The module list
  says this module "uses the rules profile as `SIM-2D` left it".
- No trigonometry, no navigation, no one-way surfaces, no holes in a heightfield, no drawing, no game schema.
  A mover does not turn during a move, and nothing carries it: a platform's motion is its game's to add.

## What a consumer must be able to do

| Need | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **N1 Get the same answer on every target** | Every test, query and move below; anywhere in [the query domain](#1-numbers-frames-and-the-two-ranges); development and release builds; whatever was asked before | Equality between the Mac and the simulator only, which share a processor. Rejected by the Wasm row, and by a run with one retained value changed, which must fail |
| **N2 Ask of two shapes whether they touch, where a ray meets one and where a moving one is stopped** | Sphere, capsule and cuboid against each other and against a triangle; a radius from zero; a capsule of no length; a flat cuboid; a cuboid at any angle; a motion from nothing to 10,000 times the mover's size; within [the working range](#1-numbers-frames-and-the-two-ranges), 32,768 units from the origin | A sweep that tests places along the path, which a fast shape steps over. Rejected by a thin triangle crossed in one motion. A sweep whose reported place overlaps once it is rounded onto the world's grid. Rejected by the same cases at a corner of the working range |
| **N3 Find colliders near a point, along a ray and in a moving shape's path** | From an empty index to 10,000 colliders, [the representative scene](../VISION.md#goal-and-exit-rule)'s entity count; sizes from a point to larger than the whole crowd; all at one point; a query from a point to wider than everything; rebuilt whenever a layer changes; asked from serial and parallel rules | The right set in an order that depends on how the tree was built. Rejected by comparing the whole sequence with a brute-force search sorted by the stated key. A tree whose boxes, through rounding, skip a pair that just touches. Rejected by exactly-touching scenes at a corner of the working range |
| **N4 Ask the same of a level's fixed collision, and the height of the ground** | Pieces from none to the 257 a level's recipe holds by default in the SDK (one terrain and 256 added), and more; a mesh piece from one triangle to the showcase terrain's 61,664, turned and scaled when placed, with slivers and triangles of no area; a heightfield of 2 by 2 heights upward; boxes; 64 layers; a piece added or removed between ticks | Fixed collision rebuilt whole when a gate opens. Rejected by a count of pieces built. A ground height that disagrees with what a ray straight down meets. Rejected by comparing the two |
| **N5 Move a character** | A sphere, a capsule or a cuboid; a motion from nothing to a fall of many times its height in one tick; ground from flat through the slope limit to vertical and overhanging; steps from nothing to the step height and taller; creases, corners and narrowing gaps; ground made of many triangles in one plane; a start that already overlaps; fixed collision alone, and with collider indexes | A move that ends inside a wall at speed, or stops dead on the seam between two triangles of a flat floor. Rejected by fast moves through thin pieces and by long slides across a floor of many triangles. A shape that creeps down a slope it stands on. Rejected by standing on one for 600 ticks |
| **N6 Know what it costs** | 2,000, 5,000 and 10,000 colliders with up to 2,000 of them asking each tick, the representative scene's movers; the tracked meshes of 224, 8,192 and 61,664 triangles; on the Mac and in Chrome | A figure for a sparse world. Each row names what was visited beside its time. The figures are baselines: none decides whether a work package passes |

## Mechanisms

### 1. Numbers, frames and the two ranges

**Decision.** Every computation is in the profile's `Number`, through its exactly rounded operations and
nothing else. So the proof `SIM-2D` made for those operations, equal bits on the three targets, carries to every
answer here. The crate is registered in `.config/architecture.toml` with `rule_package = true`. It holds no
rules, but that entry makes the profile's guard hold it, tests included, to the profile's lints on native, Wasm
and simulator builds, in development and release: no raw-float arithmetic or method, no unordered container,
no clock or thread, no dependency but `thrum-core`. The guard reads the registry, so no script changes.

**A point** is three `Number`s. If `thrum_core::profile` holds a 3-vector when WP1 starts (`GAME-SDK` WP2 adds
one under [its ruling 1](../ARCHITECTURE.md#seam-rulings-for-game-sdk)), this crate uses it. Otherwise the crate
defines its own, with public fields, and the SDK converts field by field. It needs no rotation and no angle: a
turned box is given by its edges, and a slope limit by its cosine.

**The local frame.** A test first subtracts one reference point, the mover's start or the query's point, from
every coordinate it reads, and multiplies only after. The difference of two nearby binary32 numbers is exact, so
a shape 30,000 units from the origin is tested with the precision it was stored with, and no product of large
coordinates is formed.

**The query domain** is a cube about the origin. A shape, a point or a motion's end outside it, a non-finite
value and a negative radius are refused with an error that names what was wrong. The refusal depends only on
the values, so every peer makes it, and a rule set treats it as a fault. The domain's bound is the largest
power of two for which every intermediate of every test stays finite, for any shapes inside it. **WP1's first
item measures it**; it is at least 32,768 units.

**The working range** is 32,768 units from the origin on each axis, where positions step by 2⁻⁹ units, about
2 mm at one unit to the metre (C01). It holds the largest world in the repository, the showcase terrain, whose
vertices reach 18,000 units. Inside it the promises of closeness below hold. Beyond it, out to the domain's
bound, every answer is still equal on every device and a move still never ends overlapping; contacts only get
coarser. "A corner of the working range" in an item means a case moved to within 64 units of a corner of that
cube, inside it.

### 2. Shapes and the pair tests

**Shapes.** A sphere is a centre and a radius. A capsule is two ends and a radius. A cuboid is a centre and
three half-edges, each a vector, so a box turned or scaled is the same shape and no rotation type is needed.
A triangle is three corners; it is what fixed collision is made of and is never a mover. Each is **the set of
points within a radius of the convex hull of a few points**: one, two, eight and three, with a radius of zero
for a cuboid and a triangle.

**Decision.** One routine finds the closest points of two such hulls, and the tests are built on it:

| Test | Answer |
|---|---|
| Distance | The gap between two shapes: the distance between the hulls less both radii, and zero when they overlap |
| Overlap | Whether they touch or cross: true exactly where the distance is zero, so shapes that just touch overlap |
| Ray | Where a point moved along a motion first meets a shape, by each shape's closed form |
| Sweep | Where a shape moved along a motion is stopped by another, which may have a motion of its own |

A ray or a sweep that hits gives a **fraction** of the motion from 0 to 1, a point on what was struck, and a
**normal** of unit length that points out of what was struck, toward the mover. The mover's place at the hit is
its start plus the motion times the fraction, one multiply and one add for each coordinate, which is how every
consumer and every test computes it. A triangle is met from either side, with the normal toward the side the
mover came from.

**A sweep steps by the gap.** It asks the distance, moves the shape along its motion by the gap divided by the
speed at which the motion closes it, and asks again. A convex gap cannot close faster than that speed, so no
step passes the first contact. This is conservative advancement, the method of Box2D's and Bullet's continuous
collision. The rules it holds:

1. **Never late.** A sweep that starts clear reports a place where the mover does not overlap what it struck.
   The sweep tests the place it is about to report and steps back until it is clear, so this holds by check
   and not by arithmetic.
2. **Close.** Inside the working range, the gap at the reported place is at most **the contact tolerance**, one
   constant of the rule set.
3. **A hit needs closing.** A motion along a surface or away from it reports nothing, however near it starts.
4. **A start in contact** reports fraction zero, with the normal of the shallowest way out: the direction in
   which the least movement separates the two shapes. It is never zero, and a tie goes to the first in an
   order the guide states.
5. **Never through.** A motion of any length reports what it crosses.
6. **Bounded.** A sweep takes a fixed number of steps. If they run out it reports the place it has reached,
   which is short of the contact and never past it.
7. **Pure.** An answer depends on the arguments alone. No cache and no counter outlives a call.

A path that passes within half the tolerance is a hit, and one that stays twice the tolerance clear is not.
Between the two either answer is allowed, the same on every device.

**The contact tolerance is measured, then set.** WP1's first item records the gap left at the reported place
for its sweep cases at the origin and at the corners of the working range, and sets the tolerance to a power of
two no smaller than the largest. It may not exceed 1/64 of a unit, about the 1 cm gap shipped kinematic
controllers keep ([Rapier's guide](https://rapier.rs/docs/user_guides/rust/character_controller) and Avian's
default are both 0.01). If the measurement needs more, the mechanism has failed its range and the builder
stops ([handoff](#proof-and-builder-handoff)).

**Checked before building on it.** [The probe](../../../.orchestrate/evidence/SIM-3D/planning-probe.md) swept
spheres against triangles in emulated binary32, in a local frame, for 20,000 cases at the origin, at 16,000
and at 32,700 units. No reported place overlapped. With the sweep stopping at 2⁻⁸ units the widest gap was
5.1 mm, of which up to 1.2 mm is the world's grid at that distance, and no case took more than six steps. It
did not try capsules, cuboids or the general routine, which is why WP1 measures first.

**Evidence.** Jolt, Parry, Bullet and Box2D all run one closest-points routine (GJK) under their shape casts,
and Jolt, Bullet and Box2D reduce a shape to a hull and a radius as here. Box2D 3.1 holds its results equal
across processors and compilers with floats ([Catto, 2024](https://box2d.org/posts/2024/08/determinism/)), and
Jolt offers the same as a build option at about 8% cost
([build notes](https://jrouwe.github.io/JoltPhysics/md__build_2_r_e_a_d_m_e.html)), both by the means the
profile already enforces: no fused multiply-add, no platform maths, fixed orders.

**Alternative: depend on Parry.** It has every test here, and Rapier documents it as cross-platform
deterministic with a feature on. It computes in raw `f32` through its own maths crate, so the profile's guard
could not hold it, and its promise would rest on tests of the paths we happened to run. The module list asks
for "the rules profile's numbers". Not chosen; Parry (Apache-2.0), Jolt and Box2D (MIT) are sources a builder
may read and translate, recorded in `docs/provenance/SIM-3D.md` with revision and licence.

**Alternative: a closed form for each pair.** Exact to rounding and easy to check by hand, but twelve sweep
routines, several of them hard (a capsule against a turned cuboid). Rays use closed forms because four
suffice. A builder may give any pair a closed form where it meets the same rules, and says so in the log.

**What would change the decision.** If WP1 cannot make the one routine meet rules 1 to 6 for a pair at the
corners of the working range, that pair takes a closed form. If no method meets them inside 1/64 of a unit,
the builder stops and a planner narrows the working range or the shapes.

### 3. One tree, used three times

**Decision.** A bounding-volume tree over axis-aligned boxes is built from a list and answers "which of these
might the query touch". It is used for a layer's colliders, for the triangles of a mesh piece, and for the
pieces of a level.

- **A tree is a function of the list it was built from** and of nothing else: not what it held before, not the
  machine's word size, not a thread. How it is built is the builder's choice.
- **It only narrows.** Every answer comes from a pair test on the true shapes. The tree may skip only what
  that test would reject, so its boxes are widened by more than the contact tolerance, and rounding in a box
  cannot hide a pair that just touches.
- **The order of answers never comes from the tree.** Answers are sorted by a stated key
  ([mechanism 4](#4-the-collider-index)). So a brute-force search with the same pair tests gives the same
  sequence, and the tree's build can be changed later without changing one answer of a rule set.
- **A pair test is called with the shapes and the whole motion as given**, also when only the first hit is
  wanted and a nearer one is already known. Its answer therefore cannot depend on what else was found.

**Evidence.** Jolt keeps a tree for each broad-phase layer. Unity Physics is "stateless" and by default builds
its tree from scratch each step, one for fixed bodies and one for moving
([simulation pipeline](https://docs.unity3d.com/Packages/com.unity.physics@1.3/manual/concepts-simulation.html)).
Avian replaced sweep-and-prune with a tree in February 2026 and reuses it for its queries.

**Alternative: `SIM-2D`'s hashed grid in three dimensions.** It builds in linear time and suits bodies of one
size. A long ray walks every cell it crosses, bodies larger than a cell need a second list, and fixed
collision needs a tree over triangles in any case. One structure for the three uses was chosen.

**What would change the decision.** If WP5's record shows the rebuild for 10,000 colliders taking a large part
of a tick in Chrome, the index keeps its tree and refits it, or takes a grid, behind the same answers.

### 4. The collider index

One index holds the colliders of one layer, as one `thrum-sim2d` index does: a game keeps several in place of
category masks, so asking for the nearest of four heroes looks at four colliders with 10,000 enemies around.

**Build.** The game gives it a list of (slot, shape) in ascending slot order, with spheres, capsules and
cuboids mixed, and the index replaces what it held. A bad entry refuses the build with an error that names the
slot, and the index still holds and answers what it held before. It keeps its storage between builds. It is
derived state: it is in no digest and no undo journal, and after a rewind the next build makes it again.

**Queries**, all read-only and usable from parallel rules. Each may leave one slot out, for a shape that asks
about its own layer. A query writes its answers into storage its caller keeps, so a warmed query is designed
to allocate nothing; WP2 counts what it does.

| Query | Returns | Order |
|---|---|---|
| Overlap with a sphere, a capsule or a cuboid | Every collider that touches or crosses the shape | Ascending squared distance between centres, then ascending slot |
| Nearest `k` to a point, within a distance | Up to `k` colliders | The same |
| Ray | Every collider the ray meets, or the first | Ascending fraction, then ascending slot |
| Sweep of a sphere, a capsule or a cuboid | Every collider that stops the shape, or the first | The same |

A centre is a sphere's centre, the midpoint of a capsule's ends and a cuboid's centre, as `SIM-2D` orders by
the distance between centres. A game that wants everything within a reach of a point, by surface, asks the
overlap with a sphere of that radius.

The pair tests are public, so a consumer can make a question the index has no name for, such as two shapes
that both moved this tick, from an overlap for the candidates and a sweep with two motions.

**The oracle.** A brute-force search over the same list with the same pair tests, sorted by the same key, must
give the identical sequence. The pair tests are shared on purpose: the oracle tests what the index finds and
in what order, and the pair tests have hand-worked cases of their own in WP1.

### 5. Fixed collision

A level's fixed collision is a list of **pieces**, each on one of 64 layers. A query names the set of layers
it meets.

| Piece | Made from | What it is |
|---|---|---|
| Mesh | Corners, triangles and a placement of three axis vectors and an origin | The triangles, moved to where the placement puts them when the piece is built, under a tree. A turned and scaled model is placed with profile numbers, the same on every device |
| Heightfield | A grid of heights along y over x and z, with its cell size and origin | Two triangles for each cell, split along the diagonal from corner (i, j) to corner (i+1, j+1). Cells are found by position, with no tree |
| Box | A cuboid | Solid through |

- **A mesh and a heightfield are surfaces.** A shape overlaps one where it touches a triangle, from either
  side, and a point under the ground is inside nothing. A triangle of no area stops a sweep as the segment or
  the point it is, and no ray meets it.
- **A piece is built once and shared.** It is immutable. A level is assembled from pieces in an order, with a
  tree over their boxes, and assembling costs the number of pieces and not their triangles. Two levels that
  differ by one gate share every other piece. That is what lets the SDK keep the collision of every recent
  level recipe for a rewind and for a step run ahead, and what makes a gate that opens one small piece's work.
- **It answers what an index answers:** overlap, ray and sweep, in the same orders, with an answer naming its
  piece, and its triangle where it has one, in place of a slot. A tie goes to the earlier piece in the level's
  order and then the lower triangle.
- **The height of the ground** at (x, z) on a set of layers is the highest point of those pieces on the
  vertical line through it, or nothing where none lies under it. It is what a ray cast straight down from
  above the level first meets, by the same closed forms, so the two agree to the bit.
- **The heightfield gives out its triangles** in a stated order. The SDK cooks the terrain's drawn mesh from
  them ([ruling 9](../ARCHITECTURE.md#seam-rulings-for-game-sdk)), so the ground drawn is the ground stood on.
- **Sources have a byte form.** A mesh's corners and triangles, and a heightfield's grid, encode to
  fixed-width little-endian bytes with a version, as the profile encodes state. Decoding refuses a wrong
  version, a short buffer, an index out of range, a non-finite value and a point outside the domain, each with
  an error that names it. The bytes hold sources only: trees are built on load, the same on every device. The
  container around the bytes, and the cooking step that writes them, are the SDK's.

**Evidence.** Jolt, PhysX and Parry each keep a triangle mesh under a tree and a heightfield as an implicit
grid. Baking a placement into the triangles is what a static mesh collider is in each. The other choice,
one tree for each model with queries turned into its space, cannot sweep a sphere through a scaled model
without bending it.

### 6. The kinematic move

**What it is asked.** A shape (a sphere, a capsule or a cuboid, which does not turn), where it stands, a
motion, and what to meet: a fixed collision with a set of layers, and any number of collider indexes, each with
at most one slot left out. And four settings: which way is up, as a unit vector; the slope limit, as the cosine
of the steepest angle walked on; a step height; and a snap distance.

**What it answers** (C03: "achieved displacement/contact constraints and residual motion"):

- where the shape ends, and whether anything kept it from its whole motion;
- **each contact**, earliest first, up to a fixed number the guide states: what was struck (a piece, or an
  index and slot), the normal, the point, where the mover stood and the motion it had left;
- each contact's **surface**: a floor where the normal is within the slope limit of up, a ceiling where it is
  within the limit of down, and a wall otherwise;
- **what it rests against when it ends**, moved into or not: the floor, a wall and the ceiling within the skin;
- a velocity with the parts that go into those surfaces taken out, by the rule the move itself used.

**Decision: sweep, then slide along every surface met.** The move is rounds of one sweep each, a fixed number
of them:

1. **Get clear.** A shape that starts overlapping is pushed out along the shallowest way out, a fixed number
   of times. If it is still overlapping it stays where it is and the answer says so.
2. **Sweep and go.** The shape is swept along what is left of its motion against everything it was told to
   meet, and goes to the hit, keeping **the skin**: a gap wider than the contact tolerance, so a later sweep
   along the surface starts with room and is not stopped by rounding.
3. **Take out what goes into a surface.** From the motion left, the part that goes into every surface met so
   far in this move is removed: along one surface the shape slides, between two it follows their crease, and
   where three leave no way it stops. Then the next round.
   - *Into a floor, the part is taken out along up.* A shape that only falls onto a floor stops and does not
     creep down it, and a walking shape keeps its speed over the ground uphill and down.
   - *Into a wall or a ceiling, along the normal.*
   - *A wall that leans back does not lift a shape that stood on a floor:* a slope steeper than the limit
     stops it as an upright wall would. A shape not on a floor slides down such a slope.
4. **Step.** Where a wall stopped a shape that started on a floor, and the step height is more than nothing,
   the move tries the same motion from the step height up: up, along, and down again. It takes that result if
   it ends on a floor and went farther along the level.
5. **Snap.** A shape that started on a floor, is not moving up and ended off the floor is swept down by the
   snap distance and set on a floor found there. A shape moving up is never pulled down, so a jump is the
   same call.
6. **Rest.** What lies within the skin of the end is gathered and classed.

**Every place is checked.** The move accepts a place only when the overlap test says the shape is clear there.
So a move that starts clear ends clear anywhere in the domain, and the skin sets how near it comes, not
whether it is safe. Every sub-step is a public sweep or overlap, so the move is one more function of the same
pair tests.

**The skin is measured, then set**, in the work package that builds the move, as the tolerance is in WP1: a
power of two wider than the contact tolerance and at most 1/32 of a unit.

**Why this reaches the range.** A sweep cannot pass a thin piece at any speed (rule 5). On a floor of many
triangles in one plane the shape rides a skin above the plane, where no edge between two triangles is nearer
than the faces, so nothing catches. A round shape that crosses a ridge gets normals that turn smoothly,
because a normal is the direction between closest points. One limit is stated rather than hidden: a round foot
meets a low lip on its curve, where the normal is still within the slope limit, so it rolls over a lip that
low as a ball does, whatever the step height.

**Evidence.** Sweep, slide and step is what Rapier's controller, Godot's `move_and_slide`, Unreal's character
movement and Avian's move-and-slide do; Avian's sweeps with a skin of 0.01, gathers surfaces within
twice it and slides along all of them for four rounds. Jolt's `CharacterVirtual` "moves doing collision checks
only", walks stairs by a cast "up, forward and down again" and sticks to the floor with "an additional
collision check downwards" ([reference](https://jrouwe.github.io/JoltPhysics/class_character_virtual.html)).
Box3D's mover is the newest statement of it: cast, move, gather the surfaces as planes, solve for the nearest
place that satisfies them, clip the velocity ([guide](https://box2d.org/documentation3d/md_character.html)).
Photon Quantum, which shares a world by input as Thrum does, ships a deterministic 3D controller: a sphere moved
by overlap tests. This design sweeps instead, for N5's fast moves.

**Alternative: move, then push out of whatever is overlapped.** Simpler, and what a character made of
overlap tests alone does. A fast shape passes through a thin piece before any overlap is seen, which N5 rules
out. Not chosen.

**What would change the decision.** If the plain removal in step 3 jitters in corners at the range's edge,
its builder solves the gathered surfaces together, as Box3D and Jolt do. The answers and the items do not
change.

### 7. The proof on three targets

**Retained vectors.** The crate's cases, hand-worked and seeded, each with every bit of its answer, are kept
as test data. One command, `scripts/sim3d_conformance.py`, runs them on a target and compares: natively on the
Mac in release and development builds, as Wasm in Chrome, and as a plain executable in the iPhone 18 Pro
simulator. The programs it runs live in `examples/sim3d-run`, a package that is not held to the profile and so
may read a file and count allocations. The command is this module's own. It may call `SIM-2D`'s helpers as
they are (`scripts/simulator_lifecycle.py`, the browser driver) and changes none of them. A run with one
retained value changed must fail and name the vector, so a comparison that cannot fail is caught.

**The reference walk** (WP5). A rule package under `examples/`, stepped through `thrum_core::sim`'s registered
runtime: shapes of the three kinds walk a small level of a heightfield, turned and scaled meshes, steps and
walls of boxes and a gate that is added and removed, steered by a recorded trace, with rays and overlaps among
them and rewinds of up to 8 ticks. Its world digests are retained from a straight run on the Mac and
reproduced on the three targets, with witnesses that each behaviour happened.

`SIM-2D`'s control already shows that this kind of comparison tells a platform's own maths from the profile's;
this module adds none.

## The system as it runs

| State | Owner | Kind |
|---|---|---|
| `Number`, the state encoding, the digest, the keyed random function, the profile's lints and guard | `thrum-core` and `SIM-2D`'s tooling, as built | Used, not changed |
| The pair tests, the collider index, pieces and fixed collision, the move | `thrum-sim3d` (this module) | Engine code and derived state; none of it world state |
| The vectors, the conformance command, the programs it runs, the reference walk | `scripts/sim3d_conformance.py`, `examples/sim3d-run` and the walk's package (this module) | Tooling and test data |
| Colliders as components, layers, which index holds what, the level's recipe, marks, a terrain as content, the cooked container | The game SDK | World state and content, the SDK's |
| Positions, the tick, undo, pages | `thrum-core` | Authority |

**A tick, walked.** (1) A rule changes colliders on a layer. (2) The next rule that asks of that layer has its
index built from X as it now stands, in slot order. (3) A control function calls the move with a character's
collider, its motion and the level: sweeps against the level's tree of pieces, each piece's triangles, and any
indexes named. (4) It writes the position the move returned and keeps the clipped velocity. (5) The engine
validates and commits. Nothing of this module is in the digest: it holds no state that is.

**A rewind.** Undo restores positions and the SDK's recipe. Indexes are built again when asked. The fixed
collision of the restored recipe is assembled from pieces that are still held, or built again, to the same
bits.

**A step run ahead.** A predicted character's move reads the level and the character's own state, and the
move is a pure function of them, so the guess and the host's confirmed step are the same call.

**A game maker's path.** The SDK's cooking step writes a model's triangles and a terrain's heights in this
module's byte form. At load the SDK decodes them, builds pieces and assembles the level. Rules ask through
`Space3d` and `Level3d`.

## Interfaces

**Provided**, to the game SDK, as
[the module list's row](../ARCHITECTURE.md#interfaces-track-7) names them:

| Interface | What it is |
|---|---|
| **The pair tests** | [Mechanism 2](#2-shapes-and-the-pair-tests): shapes, distance, overlap, ray and sweep, with the contact tolerance and the query domain as public constants |
| **The collider index** | [Mechanism 4](#4-the-collider-index), with its orders |
| **Fixed collision** | [Mechanism 5](#5-fixed-collision): pieces, a level assembled from them, its queries, the height of the ground, a heightfield's triangles, and the byte form of sources |
| **The kinematic move** | [Mechanism 6](#6-the-kinematic-move), with the skin and the number of contacts as public constants |
| **The conformance command and the guide** | [Mechanism 7](#7-the-proof-on-three-targets), and `docs/SIM3D.md` from WP5 |

Results are promised equal across devices for one build of one rule set, as `SIM-2D`'s are, and not across
engine versions.

**What stays the SDK's.** `Collider3d` and its offset; turning an entity's rotation and scale into a cuboid's
half-edges, a capsule's ends and a mesh's placement; an `Angle` into a cosine; layers into indexes and a set;
ties by spawn order, by giving slots in that order; the level's recipe, `LevelPiece`, marks and which built
levels are kept; `motion_of` sweeps, from an overlap and a sweep with two motions; `solid_at`, which this
module answers as a point's overlap; and a camera boom, which may ask the same queries from a frame function.

**For `GAME-SDK`'s planner.** The work package that delivers the move is `SIM-3D:WP4`. WP5 proves the whole
module on the three targets and writes the guide; a fault it finds is fixed here, behind the same interface.

**Used.** The rules profile as `SIM-2D` left it. No contract's text changes.

## Cost

No budget is set ([decision 3](../VISION.md#decisions)). The account, by design:

| Work | Account |
|---|---|
| A pair test | A handful of closest-points calls on hulls of at most eight points. The probe's sweeps took at most six steps |
| An index build | Once for each layer after a rule changed it, linear or near it in the colliders, into kept storage |
| A query | The tree's depth, then a pair test for each candidate, then a sort of the answers |
| A mesh piece | Built once when its level first needs it: its triangles placed and a tree made. 61,664 triangles is the largest tracked mesh |
| A level | Assembled from its pieces when the recipe changes: a tree over at most a few hundred boxes |
| A move | A fixed number of rounds, each one sweep against the level and the indexes named, with up to three more for a step and one for a snap |
| A capability left out | Nothing: a game with no 3D space does not link the crate |

WP5 records, on the Mac and in Chrome, from two runs with the build and the load: an index build and each
query at 2,000, 5,000 and 10,000 colliders with 2,000 asking; a piece built from each of the tracked meshes of
224, 8,192 and 61,664 triangles (`examples/starter`'s rock, `examples/mirror`'s hill and the showcase terrain)
and from a 65 by 65 heightfield; a level of 257 pieces assembled; and 2,000 moves a tick over each ground.
Every row names what it visited beside its time.

**Tripwires**, flagged in the record and investigated while building goes on. None is a pass condition beyond
a missing measurement:

- a row about a quarter slower than the last comparable one;
- cost that grows faster than what was visited;
- any allocation in a warmed build, query or move;
- 2,000 moves that take longer than the game clock's period, 16.7 ms at the initial 60 Hz. That goes in the
  log for the SDK's planner.

## Risks

- **The closest-points routine fails at touching and degenerate cases**, the known hard part of every such
  library. Early sign: WP1.2's flat, zero-length and collinear cases, and WP1.4's grazing cases at a corner of
  the working range. The fallback is a closed form for the pair that fails.
- **A convincing fake of N1:** vectors compared inside one process. Each target is a separate process on its
  own runtime, and a changed value must fail.
- **Word size.** Wasm's `usize` is 32 bits. An order or a key that passes through one could differ there.
  Answers are sorted by stated keys, which removes the common cause; WP1 and WP2 each run Chrome to see it
  early.
- **The simulator row adds little**, as in `SIM-2D`: it shares the Mac's processor. It proves the code builds
  and runs there.
- **The tree's rebuild is the cost of a busy layer in Chrome.** Early sign: WP5's Chrome rows. It is a finding
  to report, with [the change named above](#3-one-tree-used-three-times), not a stop.
- **A character sticks or jitters where surfaces meet.** Early sign: the move's slide cases across many
  triangles and into corners. [What would change the decision](#6-the-kinematic-move) names the next step.
- **A profile addition by another module lands on this crate.** `GAME-SDK` WP2 adds refusals of files, sockets
  and processes to the profile's lints, and the crate's tests are held to them. So a test that reads a file
  lives in `examples/sim3d-run` from the start.

## Proof and builder handoff

- **Tests are CPU tests on small worlds.** Nothing here needs a GPU. Each test fits the merge tier's ten
  seconds; the one full-size scene in an oracle asks a handful of questions of 10,000 colliders.
- **Expected answers are written by hand** from geometry whose coordinates the profile holds exactly: whole
  numbers and halvings, and turned boxes with whole-number edges such as (3, 4, 0), (−4, 3, 0) and (0, 0, 5).
- **Seeded cases** draw from the profile's keyed random function, so they are the same on every target.
- **Measure, then set.** The domain's bound, the contact tolerance and the skin are each set from a record
  made first, in the work package that needs them, and later items test against the constant.
- **Stopping.** If a pair cannot meet the sweep's rules inside 1/64 of a unit, by the one routine or by a
  closed form, the builder keeps the failing case with its answer and ends `BLOCKED`. If the three targets
  differ, it keeps the first differing vector with both answers and ends `BLOCKED`. It does not raise the
  limit, shrink the working range, drop a shape, compare fewer values or drop a target.
- **Other modules' files.** `thrum-core`, `thrum-sim2d`, the profile's lints and `SIM-2D`'s scripts stay as
  they are. Where one falls short, the builder writes the proposal in this module's log and ends
  `BLOCKED needs_dependency`.
- **A retained vector changes only when its test changes on purpose**, with the cause in the log.
- **No time and no allocation count stops the build.** A row that trips a tripwire is recorded as measured.
- **Chrome and the simulator** are taken by the conformance command itself, one turn at a time, as `SIM-2D`'s
  does, so no item names a lock. They start outside the sandbox. If a launch is refused, the builder leaves the
  item open and says so, and does not substitute Node or another target's result.
- **Provenance.** Code translated from another project is listed in `docs/provenance/SIM-3D.md` with its
  source, revision and licence. No GPL or share-alike code.

## Open questions

- **The domain's bound and the contact tolerance.** Settled by WP1.1.
- **The skin, and how many contacts a move reports.** Settled by the work package that builds the move.
- **Whether a heightfield needs a coarser level for long rays.** Its builder decides from WP5's rows.
- **Whether the rebuild for 10,000 colliders fits beside a frame in Chrome.** Settled by WP5's rows.
