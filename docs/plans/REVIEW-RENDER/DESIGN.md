# Review fixes in the renderer: confirmed bugs, GPU records, pass identity, errors and waste

14 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation) ·
[the code review](../../evidence/code-review-2026-11-14/CODE-REVIEW.md), slices S1 to S4 ·
[patterns](../../../.agents/skills/thrum-engineering/references/patterns.md)

## The request

The owner asked for the review's fixes to be planned and driven. The lead's order: "confirmed bugs first (H01, H09,
M41, H11, H14), then typed Rust-WGSL records and typed pass keys (T1, T2), … error convention (T3), performance
(T12)", sequenced "so it does not rewrite files that running builders hold".

## Scope

`crates/thrum-render`: the scene renderer (`synthetic/`), the atmosphere, visibility, the frame runtime, graph,
presenter and timing. Five modules own that code and all five have builders in it, so nothing here starts until
the milestones they hold are Done (`ATMOSPHERE` M9, `APPLE-TILE` M10, `APPLE-PRESENTATION` M3, `EFFECT-GRAPH` M3,
`CULLING` M3). Each change is built here under [the rulings](../ARCHITECTURE.md#seam-rulings-for-track-4) and
belongs to its owner once built. Later milestones of those modules are built on what has landed.

Its users are every builder who adds a pass, a record or a stage to the renderer next: `WATERS`, `CLOUDS`,
`SCENE-VIEWS`, `RENDER-2D`, `GAME-UI`, and the owning modules themselves. Each of them will copy the shape they
find.

## The constraint that shapes everything: no retained pixel moves

Every change here is a change of shape. The renderer's output is pinned by five registered exact baselines, on the
Mac and in Chrome, and by the owning modules' exact tests. So each milestone must leave every submitted byte as it
was, and the verifier is an item in each. A change that cannot keep them is not built here: the builder writes the
proposal in the log and ends `BLOCKED`, as [the exact-baseline rules](../ARCHITECTURE.md#exact-baselines) say.

## Mechanisms

**1. One writer for each light buffer (H09).** The scene renderer writes bytes 0 to 48 of its directional buffer
on the CPU; the atmosphere's diffuse pass writes bytes 48 to 208 on the GPU and caches them by relying on the CPU
not to write there. The split is the literal `48` in two modules, and the layout is declared in four places. The
buffer becomes two: the directional record, which only the scene renderer's upload writes, and the environment
light (the diffuse result and its coefficients), which only the atmosphere's pass writes. Each has one Rust
record, one WGSL declaration and a layout test. Readers bind both. The alternative, one record with named
offsets, keeps two writers on one buffer, which the adopted shape rules out. The values read are the same, so the
image is.

**2. A surface encode failure is kept (M41).** The runtime turns a failed surface encode into a dropped-frame
count. The failure is kept by kind on the drive report and the frame sample, and one that is not transient is
returned.

**3. No blanket error conversion (H11).** `From<String>` and `From<&str>` for `FrameError` classify a fault by
where it was raised. They are removed; each site names its variant.

**4. Records by reflection (T1).** Each record a shader reads is a `#[repr(C)]` `Pod` struct written whole. One
test composes every shader source production builds, parses it with naga, walks every struct bound as a uniform
or storage buffer, and fails unless a Rust record is registered for it with equal size and offsets. A second test
fails when one struct name is declared in two sources. Constants used on both sides are generated into the shader
from the Rust value, as the atmosphere's spectrum profile already is. Because the test finds structs by
reflection, a shader added later by another module is held without anyone remembering to list it.

**5. Typed keys (T2).** One declaration for each pass carries its role, its name, its timing category and its
recorder, so the recorder is bound by lookup and the pass count comes from the compiled graph. GPU timing sums by
the category a pass declares; a pass that lands in no bucket fails a test, where today a renamed pass reads as
zero. An atmosphere stage is one key whose chart inputs are one table, from which both the bind group and the
graph accesses are built, so the barrier the graph emits cannot differ from what the shader samples. Pass names
stay exported constants: the frame timing feed gives `CREATOR`'s profiler the names it gave.

**6. Errors (T3).** The render error moves to the crate's own error module with `Display` and `Error`; a bad
instance transform gets its own variant and the capacity variant is split into the limits it stood for. Leaf
error enums implement `Error`, `FrameError` carries typed sources, and the effect graph's validation returns an
enum. The constants the atmosphere imports from the scene renderer move to a neutral module, which removes the
module cycle the error caused.

**7. Waste (T12).** Timestamp queries are resolved in the frame's own encoder and mapped on that submission, so
timing adds no submission and no frame of latency; the completed timing reaches the sample without allocating.
The presenter's standalone graph, which nothing calls and every resize recompiles, is removed. The timing-demand
switch `CREATOR` M2 built keeps its meaning.

## Parked, with the reason

- **M24, more than four local lights built on the CPU each frame.** The fix lifts the GPU cluster build to the
  general case, a new accelerated recipe. The brief puts tuning after the exit.
- **M31, every Sun change rebuilds the sky, the diffuse light and seven reflection mips.** Its fix is a cadence,
  and the cadence of the atmosphere's products is a quality knob that `APPLE-PRESENTATION` M7 gives the governor.
- **T7, shader source assembled four ways** (M18, M20, M27, M28): not in the review's order of attack. The
  reflection test of mechanism 4 runs over the composed sources, so a text edit that breaks a record is caught.
- **T9 and T10, names and splits**: [REPO-RULES](../REPO-RULES/DESIGN.md) M3, once, after this module.

## What it must not do

Move a retained image or loosen a comparison; change an item, a test assertion or a reference of an owning
module; rename `synthetic` or split files (that is one later landing); add a numeric budget.

## Proof

Each fix has a test that fails without it, written by the builder and read by the reviewer. Unchanged output is
held by the verifier in every milestone and by the owning modules' exact tests, unedited.
