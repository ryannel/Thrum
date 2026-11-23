# Review fixes in the renderer: confirmed bugs, GPU records, pass identity, errors and waste

14 November 2026, revised 15 November after the first design review, and on 18 November for
[the indirect-light query](#mechanisms) and after [its design review](../../../.orchestrate/evidence/REVIEW-RENDER/design-review.md).
[Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation) ·
[the code review](../../../.orchestrate/evidence/code-review-2026-11-14/CODE-REVIEW.md), slices S1 to S4 ·
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

**1. One writer for each light buffer (H09).** The light record is one 208-byte buffer with two writers. Bytes 0
to 48 are the directional light (direction, irradiance, half vector), which the scene renderer writes on the CPU.
Bytes 48 to 208 are the ambient term: a constant ambient irradiance, a mark, and nine diffuse coefficients. What
writes them depends on the lighting mode:

- With an atmosphere (`FrameLighting::Atmosphere`), its diffuse pass writes the coefficients and sets the mark on
  the GPU, and caches them by relying on the CPU not to write there.
- With none (`FrameLighting::Diagnostic` and `FrameLighting::Photometric`), the CPU writes all 208 bytes. A
  photometric environment's `ambient_lux` reaches the shader as the constant ambient, and the mark stays clear.

The shader reads the coefficients when the mark is set and the constant otherwise. The split is the literal `48`
in two modules, and the layout is declared in four places.

The buffer becomes two, divided by who writes:

| Buffer | Holds | Its one writer |
|---|---|---|
| The directional record | Everything the CPU authors: direction, irradiance, half vector, and the constant ambient irradiance (zero for diagnostic and atmosphere lighting, the photometric ambient for a photometric environment) | The scene renderer's upload, every frame, in every lighting mode |
| The environment light | The atmosphere's diffuse result: the coefficients and the mark that they are valid | The atmosphere's diffuse pass. With no atmosphere the buffer is bound, zeroed when created, and nothing writes it |

A reader takes the environment light when its mark is set and the directional record's constant ambient
otherwise: the choice the shader makes today, from the same mark. So every mode keeps its behaviour. A photometric
scene with no atmosphere shades by its `ambient_lux`. An atmosphere frame shades by the sky's diffuse light, and
before its diffuse pass has first run, by the constant, which is zero in that mode, as now. Each buffer has one
Rust record, one WGSL declaration and a layout test, and readers bind both. The alternative, one record with named
offsets, keeps two writers on one buffer, which the adopted shape rules out. The values read are the same in
every mode, so the image is.

*Indirect light is asked for by position, at a surface.* The split rewrites `ambient_irradiance`, and every
reader of it follows in that landing. [C08](../../design/CONTRACTS.md#c08-light-materials-and-transport--v3-with-e1-and-v4)
names two operations, and the same landing gives each its own function in `synthetic.wgsl`:

- **The source read**, `ambient_source(direction)`, is the body `ambient_irradiance` has today: the environment
  light's coefficients for that direction when its mark is set, the directional record's constant otherwise. It is
  the ambient light as a distant source, keyed by direction alone, and takes no position.
- **The surface query** is C08's one query, by world position, shading normal, view direction, roughness and pixel
  position. `ambient_irradiance` and `indirect_radiance`, in `synthetic.wgsl` and in both composed variants
  (`synthetic_environment.wgsl` and `synthetic_plain.wgsl`), take the world position and the pixel position of the
  surface point being lit, beside what they take now. `ambient_irradiance` returns `ambient_source` of its normal
  and reads neither new argument, so every value is what it was and no retained image moves. `shade` passes its
  fragment's interpolated world position and fragment position, on every entry that reaches it, and
  `indirect_radiance` hands the same two on to `ambient_irradiance`.

Whatever shades a surface asks the query. `ambient_source` is called by `ambient_irradiance` and by media, and by
nothing else.

The cooked volume is a medium, not a surface, so it reads the source. Its fragment is a point on the back face of
the volume's box, and the march lights every occupied sample along the ray through it: no one world position
belongs to that fragment's light, and the back face is none of the samples. The volume keeps the approximation it
has, uniform sky radiance from the upward irradiance, read once before the march, and reads it from
`ambient_source`. It calls neither surface function. Its one lookup and its pixels stay as they are, and an
answer by position added behind the query later cannot reach a march sample with the wrong point. Spatial
indirect light inside a volume is not planned here: it would be a new design that says which sample supplies the
position.

A test shader that stubs one of these functions follows the signature of the function its shader calls: the
source for the cooked march, the query for the reflection consumer. Their assertions do not change. The query
takes its arguments now because its callers are three today, and a traced occlusion (`RAY-TRACING` M5), a
world-space cache and a screen-space gather each need one of the two arguments
([the decision](../../design/decisions/POST-V1-FOUNDATIONS.md), which also says why the atmosphere's lent
`environment_diffuse` and `environment_reflection` do not change).

**2. A surface encode failure is kept (M41).** The runtime turns a failed surface encode into a dropped-frame
count. The failure is kept by kind on the drive report and the frame sample, and one that is not transient is
returned.

**3. No blanket error conversion (H11).** `From<String>` and `From<&str>` for `FrameError` classify a fault by
where it was raised. They are removed; each site names its variant.

**4. Records by reflection (T1).** Each record a shader reads is a `#[repr(C)]` `Pod` struct written whole. One
test composes every shader source production builds, parses it with naga, walks every struct bound as a uniform
or storage buffer, and fails unless a Rust record is registered for it with equal size and offsets. A second test
fails when a shared record has two authoritative definitions: two source files that each declare the struct a
registered record is bound as. A declaration composed into several shaders is one definition, counted by the file
it comes from, however many composed modules repeat it. Two shader-local structs that share a name but are
different records, as `Bounds` is in the visibility and the cluster shaders, are not duplicates. Constants used on both sides are generated into the shader
from the Rust value, as the atmosphere's spectrum profile already is. Because the test finds structs by
reflection, a shader added later by another module is held without anyone remembering to list it.

**5. Typed keys (T2).** One declaration for each pass carries its role, its name, its timing category and its
recorder, so the recorder is bound by lookup and the pass count comes from the compiled graph. GPU timing sums by
the category a pass declares; a pass that lands in no bucket fails a test, where today a renamed pass reads as
zero. An atmosphere stage is one key whose chart inputs are one table, from which both the bind group and the
graph accesses are built, so the barrier the graph emits cannot differ from what the shader samples. Pass names
stay exported constants: the frame timing feed gives `CREATOR`'s profiler the names it gave.

M6 checks the stage registry on the smallest production-graph fixture that exposes all fourteen stages. Check
graph reads, table bindings, distinct roles, validity bits and timing buckets on the CPU wherever the metadata
can be constructed without a device. Use a tiny target for any prepared timestamp capacity check that needs
Metal. The milestone runs all cases that together prove the regression.

**6. Errors (T3).** The render error moves to the crate's own error module with `Display` and `Error`; a bad
instance transform gets its own variant and the capacity variant is split into the limits it stood for. Leaf
error enums implement `Error`, `FrameError` carries typed sources, and the effect graph's validation returns an
enum. The constants the atmosphere imports from the scene renderer move to a neutral module, which removes the
module cycle the error caused.

**7. Waste (T12), and what is not waste.**

*Timing.* The review's M40 asks for two things: resolve timestamp queries in the frame's own encoder, to save a
submission, and stop allocating for every completed frame. Retained device evidence outranks the first.
[The M3 Max experiment of 3 November](../../../.orchestrate/evidence/render/R0-FXAA-1080P-TIMESTAMP-2026-11-03.md) found that
resolving in the render's own submission returns stale or zero end counters on Metal, with reused query slots and
with fresh ones, on the wgpu 30.0.1 this checkout still pins. Resolving in a later submission, after the render
had completed, passed all 432 pairs. Every method drew the same image, so no image check can see the fault. The
protocol chosen from that evidence is in
[the render foundation decision](../../design/decisions/M1-RENDER-FOUNDATION.md) and in `timing.rs`: resolve and
copy only in a later host-owned submission, after render completion.

So that submission is what makes the counters right, and it stays. What goes is the allocation: the copied pass
list, the cloned pass names and the fresh string a completed frame builds today. A completed frame's timing
reaches the frame sample without allocating and says which frame it measured. Publication keeps refusing what the
decision says it must, and now by counted reason: a batch whose counters are zero, the error value, inverted or
repeated from an earlier frame, one from a retired device generation, and one discarded before submission. A
refused batch is never published as a time.

*Presenter.* The standalone graph, which nothing calls and every resize recompiles, is removed.

The timing-demand switch `CREATOR` M2 built keeps its meaning.

## Parked, with the reason

- **M24, more than four local lights built on the CPU each frame.** The fix lifts the GPU cluster build to the
  general case, a new accelerated recipe. The brief puts tuning after the exit.
- **M31, every Sun change rebuilds the sky, the diffuse light and seven reflection mips.** Its fix is a cadence,
  and the cadence of the atmosphere's products is a quality knob that `APPLE-PRESENTATION` M7 gives the governor.
- **M40's resolve in the frame's own submission.** Rejected by the device evidence above. Encoding the resolve
  into a later frame's encoder would also save the submission and keep the completion boundary, but nobody has
  run it on Metal. Either waits for someone to qualify it with that experiment's evaluator first.
- **T7, shader source assembled four ways** (M18, M20, M27, M28): not in the review's order of attack. The
  reflection test of mechanism 4 runs over the composed sources, so a text edit that breaks a record is caught.
- **T9 and T10, names and splits**: [REPO-RULES](../REPO-RULES/DESIGN.md) M3, once, after this module.

## What it must not do

Move a retained image or loosen a comparison; change an item, a test assertion or a reference of an owning
module; rename `synthetic` or split files (that is one later landing); add a numeric budget.

## Proof

Each fix has a test that fails without it, written by the builder and read by the reviewer. Unchanged output is
held by the verifier in every milestone and by the owning modules' exact tests, unedited. Two things the verifier
cannot see have proofs of their own: a photometric scene with no atmosphere, which no registered baseline draws,
and the validity of GPU timing, which never changes an image.
