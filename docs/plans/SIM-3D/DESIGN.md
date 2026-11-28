# SIM-3D: 3D collision queries and a character move that agree on every device

27 November 2026, revised 28 November 2026 after [its first design review](../../../.orchestrate/evidence/SIM-3D/design-review.md). [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
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
| **N1 Get the same answer on every target** | Every test, query and move below; anywhere in [the query domain](#1-numbers-frames-and-the-query-domain); development and release builds; whatever was asked before | Equality between the Mac and the simulator only, which share a processor. Rejected by the Wasm row, and by a run with one retained value changed, which must fail |
| **N2 Ask of two shapes whether they touch, how deep one stands in the other, where a ray meets one and where a moving one is stopped** | Sphere, capsule and cuboid against each other and against a triangle; a radius from zero; a capsule of no length; a flat cuboid; a cuboid at any angle; a motion from nothing to 10,000 times the mover's size; a sweep that begins clear, touching or inside; one shape moving or both; anywhere in the query domain, 32,768 units from the origin | A sweep that tests places along the path, which a fast shape steps over. Rejected by a thin triangle crossed in one motion. A sweep whose reported place overlaps once it is rounded onto the world's grid. Rejected by the same cases at a corner of the query domain. A sweep that reports where it came out of a thing it passed through. Rejected by the hand-worked fraction of first contact |
| **N3 Find colliders near a point, along a ray and in a moving shape's path** | From an empty index to 10,000 colliders, [the representative scene](../VISION.md#goal-and-exit-rule)'s entity count; sizes from a point to larger than the whole crowd; all at one point; a query from a point to wider than everything; rebuilt whenever a layer changes; asked from serial and parallel rules | The right set in an order that depends on how the tree was built. Rejected by comparing the whole sequence with a brute-force search sorted by the stated key. A tree whose boxes, through rounding, skip a pair that just touches. Rejected by exactly-touching scenes at a corner of the query domain. A tree that narrows nothing and tests every collider. Rejected by a count of pair tests on a spread scene |
| **N4 Ask the same of a level's fixed collision, and the height of the ground** | Pieces from none to the 257 a level's recipe holds by default in the SDK (one terrain and 256 added), and more; a mesh piece from one triangle to the showcase terrain's 61,664, turned and scaled when placed, with slivers and triangles of no area; a heightfield of 2 by 2 heights upward; boxes; 64 layers; a piece added or removed between ticks | Fixed collision rebuilt whole when a gate opens. Rejected by a count of pieces built. A ground height that disagrees with what a ray straight down meets. Rejected by comparing the two |
| **N5 Move a character** | A sphere, a capsule or a cuboid; a motion from nothing to a fall of many times its height in one tick; ground from flat through the slope limit to vertical and overhanging; steps from nothing to the step height and taller; creases, corners and narrowing gaps; ground made of many triangles in one plane; a start that already overlaps; fixed collision alone, and with collider indexes | A move that ends inside a wall at speed, or stops dead on the seam between two triangles of a flat floor. Rejected by fast moves through thin pieces and by long slides across a floor of many triangles. A shape that creeps down a slope it stands on. Rejected by standing on one for 600 ticks. A path given as its two ends, which loses the crossing of a climbed step and the rest after a stop. Rejected by the legs and their times |
| **N6 Know what it costs** | 2,000, 5,000 and 10,000 colliders with up to 2,000 of them asking each tick, the representative scene's movers; the tracked meshes of 224, 8,192 and 61,664 triangles; on the Mac and in Chrome | A figure for a sparse world. Each row names what was visited beside its time. A record with no `SIM-2D` figure beside it, which the module list asks for. The figures are baselines: none decides whether a work package passes |

## Mechanisms

### 1. Numbers, frames and the query domain

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

**The query domain** is the cube 32,768 units from the origin on each axis. A shape with a point outside it, a
point or a motion's end outside it, a non-finite value and a negative radius are refused with an error that
names what was wrong. The refusal depends only on the values, so every peer makes it, and a rule set treats it
as a fault.

Inside the domain positions step by 2⁻⁹ units at the coarsest, about 2 mm at one unit to the metre (C01), and
every promise of closeness below holds everywhere in it. There is no outer range where answers are given and
contacts are coarser: the crate answers only where its contact tolerance holds, so the SDK and this module mean
one thing by a contact wherever a query is answered. The domain holds the largest world in the repository, the
showcase terrain, whose vertices reach 18,000 units. "A corner of the query domain" in an item means a case
moved to within 64 units of a corner of that cube, inside it.

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
| Depth | For two shapes that overlap, how far the first stands inside the second, and the normal of **the shallowest way out**: the unit direction along which the least movement of the first parts them. Zero for two that do not overlap |
| Ray | Where a point moved along a motion first meets a shape, by each shape's closed form |
| Sweep | Where a shape moved along a motion is stopped by another, which may have a motion of its own |

**Depth has two cases**, because the distance alone cannot say which way is out.

- *The hulls are apart and only the radii cross.* The depth is both radii less the distance between the hulls,
  along the line between the closest points. Two unit spheres with centres one unit apart along an axis stand
  exactly one deep, along that axis.
- *The hulls themselves cross*, as two cuboids do, or a capsule's line through a cuboid. The closest points
  coincide and give no direction. The hulls are a point, a segment, a box and a triangle, so the shallowest way
  to part two of them lies along one of a short fixed list of directions: the normals of each one's faces and
  the cross products of an edge of one with an edge of the other. The test tries each by closed form, takes the
  least, and adds both radii. Nothing iterates.

A tie goes to the first direction in an order the crate's documentation states, and so does a pair with no
shallowest way, such as a shape met with itself, so the normal is never zero. A triangle is a surface, so the
way out of one is to whichever side is nearer.

A ray that hits gives a **fraction** of the motion from 0 to 1, a point on what was struck, and a **normal** of
unit length that points out of what was struck, toward the mover. A sweep's hit gives those and a **depth**.
The mover's place at a hit is its start plus the motion times the fraction, one multiply and one add for each
coordinate, which is how every consumer and every test computes it. A triangle is met from either side, with
the normal toward the side the mover came from.

**What touching means, here and in the SDK.** Two shapes *touch* where the gap between them, or the depth of
one in the other, is no more than **the contact tolerance**, one constant of the rule set. That is
[the SDK's meaning](../GAME-SDK/DESIGN.md#7-space-and-the-level) word for word, and the distance and depth
tests are how either side measures it.

A sweep, of a pair or asked of an index or a level, is asked with **the gap to stop within**. It is the
tolerance, which the SDK's sweeps ask, or half the tolerance, which [the move](#6-the-kinematic-move) asks and
which takes a motion of at most 4,096 units. The half gap is the move's working distance and not a second
meaning of touching: every place a sweep reports with it is touching, and no sweep of the SDK asks it. A sweep
meets a thing in one of three ways, which asked with the tolerance are the SDK's three:

| The sweep | Fraction | Depth | Normal |
|---|---|---|---|
| **Begins inside it**, deeper than the tolerance | Zero | What the depth test gives | The shallowest way out |
| **Begins touching it**: within the gap asked for, which asked with the tolerance is every start that touches, or overlapping by no more than the tolerance | Zero | Exactly zero | The direction between the closest points, or the shallowest way out where the two overlap |
| **Meets it on the way** | Where the mover first stands within the gap asked for | Exactly zero | The direction between the closest points there |

- **A contact there from the start is reported whichever way the motion leads**: in, along, away, or nowhere.
  The mover's place is its start, and nothing moves it. A motion of no length reports exactly the contacts
  there from the start.
- **The point** is on what was struck, nearest the mover as it stands at its place plus the normal times the
  depth. A shape set there touches what it struck.
- **The overlap test keeps its exact meaning.** It is true where the distance is zero, so a pair that stands
  within the tolerance and apart touches and does not overlap.

**On the way, a sweep steps by the gap.** It asks the distance, moves the shape along its motion by the gap
divided by the speed at which the motion closes it, and asks again. A convex gap cannot close faster than that
speed, so no step passes the first contact. This is conservative advancement, the method of Box2D's and
Bullet's continuous collision. The rules a hit met on the way holds:

1. **Never late.** The mover at the reported place does not overlap what it struck. The sweep tests the place
   it is about to report, so this holds by check and not by arithmetic.
2. **Close.** The distance test at the reported place gives no more than the gap asked for, by the same check,
   anywhere in the query domain.
3. **First.** The reported place is short of the first contact along the motion, never beyond it. A sweep
   that passes right through a thing reports where it went in, not where it came out.
4. **Never through, and nothing from nothing.** A motion of any length that brings the two shapes to overlap
   reports a hit. A motion on which the distance test never gives as little as the gap asked for reports
   nothing. One that only grazes, coming within the gap asked for and never overlapping, may report or not,
   the same on every device.
5. **Bounded and definite.** The stepping takes a fixed number of steps. If they run out, the place reached
   shows neither that a contact exists nor that it is near one, so the sweep does not report it. It finishes by
   halving instead. Along a straight motion the gap between two convex shapes falls to one lowest place and
   rises again. Halving on whether the motion is still closing therefore finds a place where the two are
   within the gap asked for, or that no such place exists, which is a miss. From a place found, halving again
   between it and the last place known to be farther off finds the first, and rules 1 to 3 are tested there as
   for any hit. Each halving takes no more steps than a fraction has bits.
6. **Pure.** An answer depends on the arguments alone. No cache and no counter outlives a call.

Rule 2 rests on the numbers offering a place that is both clear and that close. Places along a motion step by
the world's grid, and by the motion's length times the step of a fraction, so the tolerance is set wider than
the widest such step, and the half gap takes only motions short enough for the same to hold.

**Both shapes moving.** Where the struck shape has a motion of its own, the sweep is that of the mover by the
difference of the two motions. The fraction, the normal and the depth are the same in the game's coordinates as
in that frame. Places are not. The mover stands at its start plus its motion times the fraction, the struck
shape stands at its start plus *its* motion times the fraction, and the point is on the struck shape where it
stands then. Rules 1 and 2 are tested with the two shapes at those two places. So two spheres of radius ½ at
x = 0 and x = 4 that move by +6 and −2 touch at fraction ⅜ with their centres at 2¼ and 3¼, and the point is
at 2¾, on the struck sphere where it has come to, not at the 3½ it would have standing still.

**The contact tolerance is measured, then set.** WP1's first item records the gap left at the reported place by
its sweep cases, at the origin and at the corners of the query domain, with a motion from corner to corner of
the domain among them. It sets the tolerance to the smallest power of two for which every case asked to stop
within the tolerance, and every case asked to stop within half of it, keeps rules 1 and 2. It may not exceed
1/64 of a unit, about the 1 cm gap shipped kinematic controllers keep
([Rapier's guide](https://rapier.rs/docs/user_guides/rust/character_controller) and Avian's default are both
0.01). If the record needs more, the mechanism has failed its range and the builder stops
([handoff](#proof-and-builder-handoff)).

**Checked before building on it, by a stand-in.**
[The probe](../../../.orchestrate/evidence/SIM-3D/planning-probe.md) swept spheres against triangles in Python
with each result rounded to binary32, in a local frame, for 20,000 cases at the origin, at 16,000 and at
32,700 units. Of the hits it reported, none overlapped; with the sweep stopping at 2⁻⁸ units the widest gap
was 5.1 mm, of which up to 1.2 mm is the world's grid at that distance, and none took more than six steps.
Those figures describe the stand-in and set nothing. Its closest-point routine chooses a triangle's feature
from products worked in binary64, which the engine may not do, and it counted the gaps at the hits it reported
without counting separately the crossings it missed. It did not try capsules, cuboids, the depth test, a start
in contact or the general routine. The constants are set by WP1's record in the engine.

**Evidence.** Jolt, Parry, Bullet and Box2D all run one closest-points routine (GJK) under their shape casts,
and Jolt, Bullet and Box2D reduce a shape to a hull and a radius as here. Box2D 3.1 holds its results equal
across processors and compilers with floats ([Catto, 2024](https://box2d.org/posts/2024/08/determinism/)), and
Jolt offers the same as a build option at about 8% cost
([build notes](https://jrouwe.github.io/JoltPhysics/md__build_2_r_e_a_d_m_e.html)), both by the means the
profile already enforces: no fused multiply-add, no platform maths, fixed orders. Parry's shape cast takes the
distance to stop at and whether a cast that begins in contact stops (`target_distance` and
`stop_at_penetration`), which is the gap to stop within and the contact from the start here. For depth, Parry
parts a cuboid from a cuboid, a triangle and a segment by a fixed list of separating directions (`query::sat`),
while Jolt and Bullet grow a polytope until it settles (EPA).

**Alternative: an expanding polytope for depth.** It serves any convex shape, and it iterates, with its end in
rounded numbers the known weak point of the method. The hulls here are four fixed kinds, so the list of
directions is short and closed. Not chosen.

**Alternative: depend on Parry.** It has every test here, and Rapier documents it as cross-platform
deterministic with a feature on. It computes in raw `f32` through its own maths crate, so the profile's guard
could not hold it, and its promise would rest on tests of the paths we happened to run. The module list asks
for "the rules profile's numbers". Not chosen; Parry (Apache-2.0), Jolt and Box2D (MIT) are sources a builder
may read and translate, recorded in `docs/provenance/SIM-3D.md` with revision and licence.

**Alternative: a closed form for each pair.** Exact to rounding and easy to check by hand, but twelve sweep
routines, several of them hard (a capsule against a turned cuboid). Rays use closed forms because four
suffice. A builder may give any pair a closed form where it meets the same rules, and says so in the log.

**What would change the decision.** If WP1 cannot make the one routine meet rules 1 to 6 for a pair at the
corners of the query domain, that pair takes a closed form. If no method meets them inside 1/64 of a unit,
the builder stops and a planner narrows the query domain or the shapes.

### 3. One tree, used three times

**Decision.** A bounding-volume tree over axis-aligned boxes is built from a list and answers "which of these
might the query touch". It is used for a layer's colliders, for the triangles of a mesh piece, and for the
pieces of a level.

- **A tree is a function of the list it was built from** and of nothing else: not what it held before, not the
  machine's word size, not a thread. How it is built is the builder's choice.
- **It only narrows.** Every answer comes from a pair test on the true shapes. The tree may skip only what
  could not change the answer: what that test would reject, and, where only the first hit is wanted, what the
  motion cannot reach before the hit already found. Its boxes are widened by more than the contact tolerance,
  so rounding in a box cannot hide a pair that just touches.
- **It does narrow.** A query the size of a few colliders, among colliders spread apart, tests few of them. A
  tree that visits every leaf gives right answers and is not this mechanism. Each query counts the pair tests
  it made ([mechanism 4](#4-the-collider-index)), and WP2 holds a spread scene of 10,000 to a count.
- **The order of answers never comes from the tree.** It comes from a stated key
  ([mechanism 4](#4-the-collider-index)) and nothing else: the answers are sorted by it, or, where only the
  first is wanted, the least key among the candidates visited is kept, which gives the same first answer with
  no list to sort. So a brute-force search with the same pair tests gives the same sequence, and the tree's
  build can be changed later without changing one answer of a rule set.
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
to allocate nothing; WP2 counts what it does. With its answers a query gives the number of pair tests it made,
which is how narrowing is judged and what a cost row names as visited.

| Query | Returns | Order |
|---|---|---|
| Overlap with a sphere, a capsule or a cuboid | Every collider that touches or crosses the shape | Ascending squared distance between centres, then ascending slot |
| Nearest `k` to a point, within a distance | Up to `k` colliders | The same |
| Ray | Every collider the ray meets, or the first | Ascending fraction, then ascending slot |
| Sweep of a sphere, a capsule or a cuboid, with the gap to stop within | Every collider that stops the shape; or the first; or the first met on the way, with every contact there from the start before it | The same |

A centre is a sphere's centre, the midpoint of a capsule's ends and a cuboid's centre, as `SIM-2D` orders by
the distance between centres. A game that wants everything within a reach of a point, by surface, asks the
overlap with a sphere of that radius.

A sweep's answers are [the pair sweep's](#2-shapes-and-the-pair-tests), so a contact there from the start
comes first, at fraction zero, whichever way the motion leads. The third way of asking is for a caller that
must look past those to what the motion meets next, as the move does, without paying for every later hit.

**Colliders that moved.** An index holds one place for each collider, so a sweep asked of it meets each where
it stands. Two things that both moved in a tick can meet where neither began nor ended: two small spheres on
paths at right angles that reach the origin together half-way through the tick, each of which begins and ends
far outside the box the other swept. No overlap asked at either's start or end finds that pair. Finding it
takes the paths, and the paths are the SDK's, kept as its tick's working memory. So:

- **The SDK supplies the paths, as boxes.** For a layer whose colliders it sweeps by their motion, the SDK
  builds a second index of that layer: one cuboid for each collider, the axis-aligned box of every place the
  collider's shape stood along its path this tick, widened by the contact tolerance.
- **Candidates are an overlap of boxes.** For a mover the SDK asks that index for the overlap with the mover's
  own path box, widened the same way. Two shapes that come within the tolerance of each other at some moment
  both lie in their path boxes at that moment, so the two widened boxes overlap with room to spare, and the
  pair is among the answers whatever the two paths were.
- **Each candidate is settled by the pair sweep with two motions**, leg against leg by time, which is the SDK's
  to drive. The pair tests are public for this and for any question the index has no name for.

This module adds nothing for it beyond the index and the pair tests it already gives, and holds no path.

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
of the steepest angle walked on; a step height; and a snap distance. A motion longer than 4,096 units is
refused with an error that names it: farther than that in one tick is a teleport, which is the game's to write.

**What it answers** (C03: "achieved displacement/contact constraints and residual motion"):

- where the shape ends, and whether anything kept it from its whole motion;
- **the path it took**: its legs in order, up to a fixed number the crate states. A leg is a straight line with
  where it began, where it ended, and the share of the step at which it began and at which it ended. The first
  begins where the shape stood, each begins where the last ended, and the last ends where the shape ends.
  [Legs and their times](#legs-and-their-times) says which legs a slide, a stop, a step and a snap leave. The
  path is what the SDK's `follow` takes;
- **each contact**, earliest first, up to a fixed number the crate states: what was struck (a piece and its
  triangle, or an index and a slot), the normal, the point, the depth, where the mover stood, the share of the
  step at which it was met and the motion it had left;
- each contact's **surface**: a floor where the normal is within the slope limit of up, a ceiling where it is
  within the limit of down, and a wall otherwise;
- **what it rests against when it ends**, moved into or not: the floor, a wall and the ceiling within **the
  skin**, which is twice the contact tolerance;
- a velocity with the parts that go into those surfaces taken out, by the rule the move itself used.

**Decision: sweep, then slide along every surface met, all of it inside the contact tolerance.** The move is
rounds of one sweep each, a fixed number of them. Its sweeps are asked to stop within half the tolerance.

1. **Begin where it stands.** Nothing pushes the shape out of what it begins inside. Each such thing, deeper
   than the tolerance, is a contact at the start of the step, with its depth and the shallowest way out, and
   these come first among the contacts. Asked for no motion, the shape stays where it is and reports them. Two
   unit spheres with centres one unit apart report the place the mover stood, a depth of exactly one and the
   normal along the line of centres.
2. **Sweep.** The shape is swept along what is left of its motion against everything it was told to meet, asked
   for the first met on the way with every contact there from the start. What stands in the way is the next
   contact, by one of two routes:
   - *It was already touching.* A thing within the tolerance of the shape as the round begins, which the
     motion would bring within half the tolerance, takes its part of the motion where the shape stands. So
     does a thing already nearer than half, and a thing the shape began inside, where the motion has a part
     into it. The contact is at the place the round began, and no leg is travelled to it.
   - *It is met on the way.* Otherwise the shape goes to the place the sweep reports, within half the
     tolerance of what stopped it and clear of it. That is a leg, and the contact is at its end.

   A touching thing that the motion leads along or away from stops nothing and is no contact of the move.
   If it is still there at the end, it is in what the shape rests against.
3. **Take out what goes into a surface.** From the motion left, the part that goes into every surface met so
   far in this move is removed: along one surface the shape slides, between two it follows their crease, and
   where three leave no way it stops.
   - *Into a floor, the part is taken out along up.* A shape that only falls onto a floor stops and does not
     creep down it, and a walking shape keeps its speed over the ground uphill and down.
   - *Into a wall or a ceiling, along the normal.*
   - *A wall that leans back does not lift a shape that stood on a floor:* a slope steeper than the limit
     stops it as an upright wall would. A shape not on a floor slides down such a slope.
   - *Out of a thing the shape began inside, only the part that leads deeper.* The shape leaves by its own
     motion, or by the game's line with the depth, and goes no deeper.
4. **Slide, riding at three quarters of the tolerance.** The next leg follows what is left of the motion. Where
   it begins nearer than three quarters of the tolerance to a surface met in this move, it gains along that
   surface's normal what brings the shape out to three quarters by the leg's end. Nothing is gained away from
   a thing the shape began inside.
5. **Step.** Where a wall stopped a shape that started on a floor, and the step height is more than nothing,
   the move tries the same motion from the step height up: up, along, and down again. It takes that result if
   it ends on a floor and went farther along the level.
6. **Snap.** A shape that started on a floor, is not moving up and ended off the floor is swept down by the
   snap distance and set on a floor found there. A shape moving up is never pulled down, so a jump is the
   same call.
7. **Rest.** A shape with part of its step unspent and no way left rests where it is. What lies within the
   skin of the end is gathered and classed.

**Every place is checked.** A leg's end is accepted only where the overlap test says the shape is clear of
everything it was clear of when the leg began, and the depth test says it is no deeper in anything it began
inside. A leg that fails is cut back by halving, a fixed number of times, and at worst the shape stays where
the leg began. So a move that starts clear ends clear anywhere in the domain, and one that starts inside a
thing ends no deeper in it. Every sub-step is a public sweep, overlap, distance or depth, so the move is one
more function of the same pair tests.

**Why three distances inside the tolerance.** A leg stops within half the tolerance, a slide rides at three
quarters, and touching ends at the whole.

- *Every contact the move reports is touching, as the SDK means it.* Met on the way, the mover stood within
  half the tolerance of what it struck and clear of it. Taken where the shape stood, it stood within the
  tolerance.
- *The shape ends within the tolerance of what it slid along and of what stopped it*, so the next move begins
  touching it and slides along it as any does. The skin is not a gap the shape keeps. It is only how far the
  report of what the shape rests against reaches.
- *Nothing catches on a floor of many triangles.* The shape rides at three quarters of the tolerance above the
  plane. The next triangle in that plane, or the next box of a floor made of boxes, is never nearer than the
  plane itself, and a leg stops only within half the tolerance. So a seam stops nothing and bends no leg,
  however many of them one motion crosses.
- *A walker's step is one leg.* Its weight presses a floor it already touches. The floor takes that part where
  the shape stands, and what is left is one straight leg along the floor, the path for which the SDK keeps
  nothing more than a written position gives.
- *Rounding has room.* At a corner of the domain places step by about 2 mm. With the tolerance at 1/64 of a
  unit, the quarter between where a leg stops and where a slide rides is about 4 mm, and so is the quarter
  between the ride and the tolerance. A motion of at most 4,096 units drifts by less than a millimetre from
  its line. A shape that rounding carries nearer than the ride is brought back by its next slide. One carried
  out past the tolerance no longer touches the floor, meets it on the way in its next step as a fall does, and
  rides again.

A sweep cannot pass a thin piece at any speed (rule 4). A round shape that crosses a ridge gets normals that
turn smoothly, because a normal is the direction between closest points. One limit is stated rather than
hidden: a round foot meets a low lip on its curve, where the normal is still within the slope limit, so it
rolls over a lip that low as a ball does, whatever the step height.

<a id="legs-and-their-times"></a>

**Legs and their times.** The share of the step is the SDK's time along the tick. One rule gives every leg its
time: **a leg takes the share of the step that it spends of the motion.**

- *A leg that ends at a contact* met a fraction of the way through the motion that was left takes that
  fraction of the share that was left. Taking out the part that goes into a surface spends nothing, so the
  slide that follows has all the share still left. A step of (3, 0, 3) that meets a wall a third of the way
  along and slides along it is two legs, the first over the first third of the step and the second over the
  other two thirds.
- *A rest* is a leg of no length. A shape with no way left rests where it stopped for the share still left,
  and a rest is always the last leg. A shape that a wall stops head-on a quarter of the way through its step
  has one leg over the first quarter and a rest over the other three.
- *A climbed step* is the leg to the wall, then the rise, the crossing and the descent. The rise and the
  descent spend none of the motion, so each begins and ends at one moment: the rise at the moment the wall
  was met, the descent at the moment the crossing ended. The crossing spends the motion that was left at the
  wall, as one leg or as several where it meets a thing and slides, each with its time by this rule.
- *A snap* is one leg straight down that spends none of the motion. It stands at the moment the travel before
  it ended, and before the rest where there is one.
- *A contact taken where the shape stood* ends no leg. The leg after it begins at the same place and moment.
- The first leg begins at 0 and the last ends at 1. A motion that met nothing is one leg from 0 to 1, and no
  motion is one rest from 0 to 1.
- Only the legs of the path taken are returned. A step that was tried and not taken leaves none.

So a trigger on the crossing of a climbed step lies on a leg of the path, and a sweep along the path meets it
where the straight line between the two ends passes beneath it. And a thing that crosses, in the last
three quarters of a tick, the line a stopped shape travelled in the first quarter is not met.

**Evidence.** Sweep, slide and step is what Rapier's controller, Godot's `move_and_slide`, Unreal's character
movement and Avian's move-and-slide do; Avian's sweeps with a skin of 0.01, gathers surfaces within
twice it and slides along all of them for four rounds. Jolt's `CharacterVirtual` "moves doing collision checks
only", walks stairs by a cast "up, forward and down again" and sticks to the floor with "an additional
collision check downwards" ([reference](https://jrouwe.github.io/JoltPhysics/class_character_virtual.html)).
Box3D's mover is the newest statement of it: cast, move, gather the surfaces as planes, solve for the nearest
place that satisfies them, clip the velocity ([guide](https://box2d.org/documentation3d/md_character.html)).
Photon Quantum, which shares a world by input as Thrum does, ships a deterministic 3D controller: a sphere moved
by overlap tests. This design sweeps instead, for N5's fast moves. Riding a small way out from
what the shape slides on is what Rapier's controller does with its `offset`.

**Alternative: move, then push out of whatever is overlapped.** Simpler, and what a character made of
overlap tests alone does. A fast shape passes through a thin piece before any overlap is seen, which N5 rules
out. Not chosen.

**What would change the decision.** If the plain removal in step 3 jitters in corners at the domain's edge,
its builder solves the gathered surfaces together, as Box3D and Jolt do. If a quarter of the tolerance proves
too little room at a corner of the domain, its builder moves the stop and the ride, keeping the stop below the
ride and the ride below the tolerance, and says so in the log. The answers and the items do not change.

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
indexes named. (4) It follows the move, which puts the character where the move ended and hands the SDK the
legs of the path, and keeps the clipped velocity. (5) The engine validates and commits. Nothing of this module is in the digest: it holds no state that is.

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
| **The pair tests** | [Mechanism 2](#2-shapes-and-the-pair-tests): shapes, distance, overlap, depth, ray and sweep, a sweep with one motion or two and with the gap to stop within, and the contact tolerance and the query domain as public constants |
| **The collider index** | [Mechanism 4](#4-the-collider-index), with its orders and each query's count of pair tests |
| **Fixed collision** | [Mechanism 5](#5-fixed-collision): pieces, a level assembled from them, its queries, the height of the ground, a heightfield's triangles, and the byte form of sources |
| **The kinematic move** | [Mechanism 6](#6-the-kinematic-move): the end, the path as legs with their times, the contacts with their depth, and what the shape rests against, with the skin, the longest motion and the numbers of contacts and legs as public constants |
| **The conformance command and the guide** | [Mechanism 7](#7-the-proof-on-three-targets), and `docs/SIM3D.md` from WP5 |

Results are promised equal across devices for one build of one rule set, as `SIM-2D`'s are, and not across
engine versions.

**What stays the SDK's.** `Collider3d` and its offset; turning an entity's rotation and scale into a cuboid's
half-edges, a capsule's ends and a mesh's placement; an `Angle` into a cosine; layers into indexes and a set;
ties by spawn order, by giving slots in that order; the level's recipe, `LevelPiece`, marks and which built
levels are kept; the path each entity took in a tick, which `follow` keeps from a move's legs;
`motion_of` sweeps over those paths, with their candidates from
[an index of path boxes](#4-the-collider-index) and each settled by a sweep with two motions; `approach_speed`,
from the two motions and the normal; `solid_at`, which this module answers as a point's overlap; and a camera
boom, which may ask the same queries from a frame function.

**For `GAME-SDK`'s planner.** The work package that delivers the move is `SIM-3D:WP4`. WP5 proves the whole
module on the three targets and writes the guide; a fault it finds is fixed here, behind the same interface.
Four things in this design are the SDK's to read when WP16 gets its items:

- The rise and the descent of a climbed step, and a snap, are legs that begin and end at one moment. They are
  in the path so a sweep meets what stands on them, and a speed along one has no meaning.
- A move's `contacts()` are what the shape began inside and what took part of its motion. A thing it only
  touched, and led along or away from, is in what it rests against and not among them.
- A motion longer than 4,096 units, and anything outside the query domain, is refused.
- Touching, depth and the three ways a sweep meets a thing are the SDK's own, measured by this module's public
  distance and depth tests.

**Used.** The rules profile as `SIM-2D` left it. No contract's text changes.

## Cost

No budget is set ([decision 3](../VISION.md#decisions)). The account, by design:

| Work | Account |
|---|---|
| A pair test | A handful of closest-points calls on hulls of at most eight points. A depth is one pass over a fixed list of directions. The stand-in's sphere sweeps took at most six steps; the engine's count is WP1's to record |
| An index build | Once for each layer after a rule changed it, linear or near it in the colliders, into kept storage |
| A query | The tree's depth, then a pair test for each candidate, then the answers put in key order, or the least key kept where only the first is wanted |
| A mesh piece | Built once when its level first needs it: its triangles placed and a tree made. 61,664 triangles is the largest tracked mesh |
| A level | Assembled from its pieces when the recipe changes: a tree over at most a few hundred boxes |
| A move | A fixed number of rounds, each one sweep against the level and the indexes named and a check of the place reached, with up to three more for a step and one for a snap |
| A capability left out | Nothing: a game with no 3D space does not link the crate |

WP5 records, on the Mac and in Chrome, from two runs with the build and the load: an index build and each
query at 2,000, 5,000 and 10,000 colliders with 2,000 asking; a piece built from each of the tracked meshes of
224, 8,192 and 61,664 triangles (`examples/starter`'s rock, `examples/mirror`'s hill and the showcase terrain)
and from a 65 by 65 heightfield; a level of 257 pieces assembled; and 2,000 moves a tick over each ground.
Every row names what it visited beside its time: for a query and a move, the pair tests made.

**Beside `SIM-2D`'s.** The module list asks for "the cost of a query and a move recorded beside `SIM-2D`'s as
a tripwire", so WP5's record carries `SIM-2D`'s figures on each target and is not complete without them. They
are read from `SIM-2D`'s retained cost records, `tests/fixtures/sim-2d/costs-mac-metal.json` and
`tests/fixtures/sim-2d/costs-chrome-webgpu.json`, as they stand when WP5 is built, and are not measured again:

| From the 2D record's `crowd` trace, at 2,000, 5,000 and 10,000 bodies | Set beside, at the same count of colliders |
|---|---|
| `index_median_ms`, one build of the 2D index | One build of the 3D index |
| `tick_median_ms` with `neighbours_per_tick`, and `tick_ms_per_neighbour` | The 3D overlap row: its time with its pair tests, and its time for each pair test |

Each pair of rows is labelled with what differs in what was timed, so no one reads it as like for like:

- the 2D figure is a whole tick of the crowd's rules, in which every body asks for its neighbours, and the 3D
  figure is the queries alone, with 2,000 asking;
- the 2D index is a hashed grid over circles on a plane, and the 3D one a tree over three kinds of shape;
- the 2D record names its own commit, date and load, which are not this record's.

The 2D record has no ray, no sweep and no move today. WP5's record says so against each of those 3D rows, or
carries the 2D row where the record it reads has gained one. The comparison is context for a reader and a
place to look when a tripwire trips. It has no threshold.

**Tripwires**, flagged in the record and investigated while building goes on. None is a pass condition beyond
a missing measurement or a missing `SIM-2D` figure:

- a row about a quarter slower than the last comparable one;
- cost that grows faster than what was visited;
- any allocation in a warmed build, query or move;
- 2,000 moves that take longer than the game clock's period, 16.7 ms at the initial 60 Hz. That goes in the
  log for the SDK's planner.

## Risks

- **The closest-points routine fails at touching and degenerate cases**, the known hard part of every such
  library. Early sign: WP1.2's flat, zero-length and collinear cases, and WP1.4's edge and corner cases at a
  corner of the query domain. The fallback is a closed form for the pair that fails.
- **The room inside the tolerance is too little at the domain's corners**, where places step by about 2 mm and
  the move wants a stop, a ride and the tolerance apart. Early sign: WP1.1's record for sweeps asked to stop
  within half the tolerance, and later the move's long slides at a corner of the domain.
  [What would change the decision](#6-the-kinematic-move) names the next step, and a tolerance that would pass
  1/64 of a unit stops the builder.
- **The SDK reads a leg of no duration as a division by nothing.** The rise, the descent and a snap take no
  time by the one rule for times. Early sign: `GAME-SDK` WP16's sweep of a followed step. The handoff below
  tells its planner.
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
- **Measure, then set.** The contact tolerance is set from a record made first, in WP1, and later items test
  against the constant. The query domain's bound, the half gap, the ride and the skin are fixed by this design
  in terms of it.
- **Stopping.** If a pair cannot meet the sweep's rules inside 1/64 of a unit, by the one routine or by a
  closed form, the builder keeps the failing case with its answer and ends `BLOCKED`. If the three targets
  differ, it keeps the first differing vector with both answers and ends `BLOCKED`. It does not raise the
  limit, shrink the query domain, drop a shape, compare fewer values or drop a target.
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

- **The contact tolerance.** Settled by WP1.1.
- **How many contacts and legs a move reports.** Settled by the work package that builds the move.
- **Whether a heightfield needs a coarser level for long rays.** Its builder decides from WP5's rows.
- **Whether the rebuild for 10,000 colliders fits beside a frame in Chrome.** Settled by WP5's rows.
