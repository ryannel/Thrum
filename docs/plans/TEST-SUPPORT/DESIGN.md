# Test support: one GPU gate, one test device, shared fixtures

14 November 2026, revised 15 November after [the first design review](../../../.orchestrate/evidence/TEST-SUPPORT/design-review.md),
and on 20 November, when [the kit](#the-kit) and WP3 and WP4 were added from the cleanup review, and again that day
after the kit's design review and the hardening pass, when WP5 was added ahead of them. Revised once more that day
after WP2's third failed review: [other platforms](#other-platforms) is new, and WP2's items changed with it. Nothing
WP1 is held to changed. Revised 21 November, when [writing a test](#writing-a-test) and WP6 were added, and again
that day after WP3's first review: [Proof](#proof) says how a test another module removed counts.
[Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation) ·
[the code review](../../../.orchestrate/evidence/code-review-2026-11-14/CODE-REVIEW.md), theme T6 ·
[patterns: tests](../../../.agents/skills/thrum-engineering/references/patterns.md#tests)

## The request

The owner asked for the review's fixes to be planned and driven. The lead's task for this module: "test support
and one GPU gate (T6)", after the confirmed bugs and the typed records, and before the device recipe.

On 21 November the owner approved a set of test guard rules for planning: "Yes, plan it." Their purpose is that
the fast-test habits (small worlds, CPU first, few binaries, long runs off the merge path) hold "without anyone
remembering them". The rules a merge's check enforces are [GATE-TIME](../GATE-TIME/DESIGN.md#the-guard-rules)'s.
This module's part is the lead's task for the engineering skill: "one short block on writing a test that
replaces the "Unit tests" and "GPU tests" rows rather than adding to them, with pattern entries the new rules'
messages link to". The block's five lines are given: "prove a behaviour on the smallest world that shows it;
test logic on the CPU and open a device only for what a GPU alone can show, at the test kit's small target; step
simulated time, never wait on the clock; add to an existing test binary; a full-size or long run is a main-tier
test and needs a plan's say." And: "The owner keeps always-loaded text short: the block stays about that size
and AGENTS.md gets nothing." The kit's small target did not exist, and is planned here.

## Purpose and who it is for

Tests are four fifths of the renderer's code, and they are where a new builder looks for a shape to copy. Today
they hold about 130 adapter requests and 15 local device helpers. GPU tests are gated four ways: a whole file
behind a macOS condition, which hides 13 CPU-only graph tests from Linux and Windows; a condition on each test; no
gate at all, which panics on a machine with no adapter; and a silent early return, which passes. `./dev check` runs
`cargo test --workspace` on every platform, and CI lists Ubuntu, macOS and Windows. This module gives tests one
answer. Its users are builders, and reviewers who need to know that a green run ran.

## Mechanisms

**1. Two packages, split by who may depend on them.**

| Package | Holds | May be depended on |
|---|---|---|
| `crates/thrum-test-support` | The GPU gate and the test device. It links wgpu | As a dev-dependency only, by any package |
| `crates/thrum-fixtures` | Fixtures: scene data that more than one package draws. It depends on nothing, wgpu and the workspace's packages included | As a dev-dependency by any package, and as a normal dependency by the proving ground alone |

The split is needed because the first fixture has a consumer that is not a test: the proving ground hosts the
mixed-mesh field in its library, on the Mac and in the browser, and `thrum-render`'s visibility tests draw the same
field. A dev-dependency cannot reach a library build, and the GPU gate must never reach one. Two packages make the
rule a fact about Cargo edges that manifests alone decide; one package with a feature for the gate would let
feature unification pull wgpu test code into a normal build.

Both packages are registered in `.config/architecture.toml` with a `test_role` (`"support"` or `"fixtures"`), and
the proving ground is named once under `[boundaries]` as `proving_ground`. One rule, `test-crate-edges`, reads that
registration and refuses every other edge: a normal or build dependency on the support package from anything; a
build dependency on the fixtures package; a normal dependency on it from any package but the proving ground. The
game gate's `boundary` runs the same rule on a game, so a game may use either in its tests and ship neither.

**2. The gate asks a question with one answer.** A GPU test starts by asking the gate for a GPU.

- With an adapter, the test runs.
- With none, the gate reports the skip and the test returns. The report is one line,
  `thrum-gpu-skip: <test>: <reason>`, written to the process's standard error handle and not through the print
  macros. Rust's test harness cannot report a skip at run time and it captures the print macros of a passing test,
  but it does not capture the handle: [a probe](../../../.orchestrate/evidence/TEST-SUPPORT/planning/skip-line-probe.md) on the
  pinned toolchain shows the line in an ordinary run. So a skip is visible in `cargo test`, `./dev check` and every
  pinned check, with no flag.
- With `THRUM_REQUIRE_GPU=1` and no adapter, the test fails with the same reason. `./dev check` sets the variable
  on macOS, the one platform every target runs on, so on the Mac a missing adapter can never pass for a run, and a
  run that passes with the variable set is a run in which no gated test skipped. The support package is a process
  edge, so it may read the environment; [REPO-RULES](../REPO-RULES/DESIGN.md)' `library-env` rule exempts a package
  registered with `test_role = "support"` and no other.
- A test that needs one backend says so to the gate (`Metal`, for the native segment), and is gated on the test,
  never on the file.

**3. The device comes from the recipe, and the gate is wgpu-level.** A crate that depends on `thrum-render` cannot
hand `thrum-render`'s own unit tests a value typed by `thrum-render`: Cargo builds the crate under test a second
time, and the two copies' types differ. So the gate itself deals only in wgpu types (instance, adapter, the skip),
and has no dependency on an engine crate. The device then comes from [HOST](../HOST/DESIGN.md)'s one recipe in
`thrum-render`: `thrum-render`'s tests call it directly, and every other crate gets gate and recipe together
through a `render` feature of the support package.

A test asks for its device as the recipe defines it: by profile, optionally with a **restriction**, which lowers
limits or removes features from what the profile asks and can add nothing. Restricted devices are test inputs that
exist today and must survive: `thrum-sea`'s `portable_limits_gpu` builds a device with seven storage buffers a
stage to prove that the sea refuses it before shader preparation, and the sea's harness runs once on WebGPU's
default limits and once on the adapter's. Those are the native profile restricted to seven buffers, the portable
profile, and the native profile.

Until `HOST` WP1 lands the recipe, WP1 here answers the same request (profile and restriction) from a stand-in in the
support package, with the features `DeviceCapabilities` already names. WP2 replaces the stand-in's body with the
recipe; no test changes again.

**4. Fixtures are plain data, and each exists once.** A fixture is data and the functions that build it, with no
type of any game or engine crate; each consumer converts it to its own types at its edge. This module owns the
fixtures package, its rule and what a fixture is. The first fixture is the mixed-mesh field: its entities,
materials, camera path, prop meshes and the day cycle its tests light it with, which `thrum-render`'s visibility
tests compile from the game's source today. [GAME-SPLIT](../GAME-SPLIT/DESIGN.md) WP2 creates the package with that
fixture, because the visibility tests are held until then; after it, the field's data is defined in
`thrum-fixtures` and nowhere else. A fixture that is one crate's own data stays in that crate behind its API.

**5. Rules hold the shape.** `device-request` (no adapter or device request outside the recipe and the gate),
`whole-file-gpu-gate` and `path-attribute` are [REPO-RULES](../REPO-RULES/DESIGN.md)' ratchets. This module brings
them to zero: first in the crates no builder is working in, then across the renderer and the examples.

<a id="the-kit"></a>

**6. The kit: a job every GPU test repeats is written once.** Added 20 November 2026 from section 5 of the
cleanup review, which is kept in the main checkout at `/Users/ryannel/Workspace/thrum/tmp/cleanup-2026-11-20/`
(`REPORT.md`, with each finding and its verifier's note in `results.json`). After WP2 every GPU test goes through
the gate, but each file still writes the same jobs by hand: about 44 staging readbacks, a gate-and-device
prologue at about 286 sites, five reflection tests of WGSL records, and its own frame waits, scratch folders and image
writers. The kit is in `thrum-test-support`, which every engine test already depends on. It covers tests under
`crates/` and the kit scenes; the game's own tests keep their helpers, because they leave with the game and may
use only the public API.

The kit is built before it is adopted. The copies were multiplying while the plan waited: hand-written readbacks
in test files went from 10 to 34 in a week, and hand-written deadline asserts from 64 to 145. So WP5 builds the
kit's functions and adds two rules through [REPO-RULES](../REPO-RULES/DESIGN.md)' rule engine, as WP1 added
`test-crate-edges`, each entered at the count of the commit it lands on: `test-readback`, a buffer mapped for
reading in a test file outside the kit, and `test-deadline`, a hand-written deadline assert in a test file. The
engine holds a count that can only fall, so from then on a new test uses the kit or fails `./dev check`, and WP3
converts what is already there.

- **Readback** (R4-01, L7-06, L3-01, R5-08), in wgpu types only: read a buffer, a whole mip of a 2D, 3D or array
  texture or one layer of it, or a byte range of either, with one bounded wait. It has two forms, because some
  tests must read inside the encoder under test: one submits its own copy, and one appends the copy to the
  caller's encoder and waits on that submission. When WP3 takes `test-readback` to zero under `crates/` and the kit
  scenes, a site that does a different job keeps its code and is exempted by name in the rule, with its reason: a
  test that yields to a browser timer on wasm, one that maps a range the engine copied into itself inside the
  host's retirement loop, and those that poll the host between submissions on purpose.
- **The prologue** (R4-03, R5-06): one call asks the gate for a GPU, on Metal where the test says so, opens the
  recipe's device by profile and restriction, and returns the device and queue, with the capabilities through
  the `render` feature. By mechanism 3 a value typed by `thrum-render` cannot reach `thrum-render`'s own unit
  tests, and those are 160 of the 286 prologues. So they get the same one call from the crate's own
  `test_device`, typed by the crate: it asks the kit's gate, which deals only in wgpu types, and opens this
  build's recipe. A local wrapper that differs for a real reason (a fresh adapter, a hardware override, a
  generation) shrinks to a call and stays. A test that asserted Metal keeps failing without it on this Mac: the
  prologue never turns that failure into a skip here.
- **Waiting, scratch and images** (G2-06, L1-08): one wait with a deadline, one scratch folder that removes
  itself, one PPM writer, and a counting allocator type. The wait serves a frame's completion and the deadline
  loops tests write by hand. WP3 converts the frame waits; the hand-written deadline asserts, in about 61 files,
  are not swept here. They stay under the frozen `test-deadline` count, a new test uses the kit's wait, and a
  conversion that removes one lowers the count. The allocator type counts allocations and bytes on the calling
  thread, behind a switch the test turns on, because the six copies it replaces count one or the other: a
  converted test counts what it counted before. A `#[global_allocator]` cannot live in a crate every test
  links, so each binary that counts declares its own static of the kit's type. Quantiles are not shared, because
  one rule would move recorded tripwire percentiles by a rank. A test that fails because the clock refuses under
  overload is not made to pass by a longer deadline, and is left as it is.
- **Record reflection** (L6-01, L3-02): one check that a crate's WGSL records match its Rust structs in member
  offset, size and scalar kind, built from the renderer's checker. It takes a declarations-only source and
  member aliases, because water's and the asset crate's shaders do not parse alone and name members differently.
  The renderer's own record test, the sea, the asset crate, water and clouds call it, and the three copies of
  the bound-struct walk and the sea's second layout type go. Each crate's `wgsl_records` test target keeps its
  name, which open and Done checks run. A check that does more stays: the view record's literal offsets, and naga
  validation of the cloud and weather shaders. The shared check is stricter than the sea's and the asset crate's
  were, and may find a record whose size or scalar kind is wrong. Fixing a record is engine code, which this
  module must not change: that crate's test keeps the comparison it made before for that record, and the log
  names the record and the module that owns it.
- **The cooked game** (G2-07): the proving ground's cooked tests build the same cooked game about twenty times,
  in two shapes. One fixture in the proving ground's test support makes the scratch store, the cook, the device
  for a draw path and the host. Each case keeps its assertions and the device it had: the generation it passed
  to the recipe and its assertion on the execution path. The fixture shares code, not a device or a lock, so no
  test that ran beside others now waits for one: the cleanup's purpose is a shorter check. The cooked fault
  test, which is behind a Cargo feature, is left as it is.

<a id="other-platforms"></a>

**7. Linux and Windows are checked from this Mac, by type.** Nothing runs Thrum's tests on Linux or Windows: no
target in the project settings is there, and CI is started by hand. But taking an OS condition off a test hands its file
to those platforms' compilers, and a compile of a few named targets, or one for another Apple target, does not
show what they will make of it: an Apple-only dependency or a macOS-only import is found only by compiling
everything for them. So the claim is made checkable here. One tracked command, `scripts/check_cross_target.sh`, type-checks every target
of every workspace package (libraries, binaries, examples, tests and benches, with default features) for
`x86_64-unknown-linux-gnu` and `x86_64-pc-windows-msvc`, and fails on an error in either. It links nothing. The
workspace's C and C++ build scripts cannot cross-compile from a Mac, so they get stand-in compilers that write
empty outputs, and the one binding generator among them gets the SDK's own headers.

It shows that the tests build on those platforms: each `cfg`, import and dependency resolves. It does not show
that they link, that the C and C++ build there, that doc tests build, or that anything passes when run. Warnings
do not fail it: helpers that only macOS-gated tests use are unused elsewhere, and a lint-clean build there is not
claimed. The two things a type-check cannot see are held by reading: that no OS condition hides a test that
needs no GPU, which `whole-file-gpu-gate` covers for files and modules, and that every GPU test asks the gate
first and asks for Metal where it needs Metal. A GPU test passing on another backend is not claimed anywhere,
because nothing is qualified there.

The check takes about a minute for both targets from cold, with no lock. It belongs to the main tier of
[the test tiers](../../../ORCHESTRATE.md#test-tiers) all the same. The merge tier holds what would stop every
builder, and a break here stops none: nobody builds for these platforms. The risk accepted is that a change which
breaks the Linux or Windows build of a test reaches main and sits there for up to an hour and one run, until the
lead gives its module the fix. WP2 adds this line under the settings' Rules, in these words:

```text
- **Main-branch check cross-target:** `bash scripts/check_cross_target.sh`; every 1h; timeout 15m  *(planner, 20 Nov 2026: the Linux and Windows type-check of the tests runs on main, not in a merge's check)*
```

<a id="writing-a-test"></a>

**8. How to write a test is said once, in five lines, and the kit has the target they name.** WP6 builds both. A
habit that lives only in the project settings is read by planners and reviewers; a builder about to write a test
reads the engineering skill. So the skill carries the habits, and the settings stay the authority for them.

- **The small target.** A GPU test that draws needs something to draw to, and each file makes its own today:
  the widths written in test files run from 1 texel to 640. The kit's `target` module gives it in one call, in
  wgpu types alone as mechanism 3 requires: a colour target, with a depth target where the test asks, at the
  kit's one small size. The size is a constant, so a test that drives a host passes the same size to it. The
  kit chooses the size once: 64 texels a side is, after a single texel, the commonest width in the test files
  today, and small enough that a clear, a draw and a readback cost milliseconds while a mip chain still has
  levels. A test whose subject is a size, such as a resize or a tiled recipe, says so and keeps its own. Existing tests are not moved to the target: that would
  change what they assert, which this module must not do. `GATE-TIME` WP9 and WP3 may use it where they make a GPU
  test's target smaller.
- **The block.** In the skill's adopted-shapes table the rows "Unit tests" and "GPU tests" go, and one block
  stands in their place with the five lines the owner approved, each a link to its entry in the patterns
  reference. What the two rows said, the unit-test layout and the GPU gate, is already in the patterns'
  Tests section and is one link from the block. The skill is loaded whenever an agent writes code here, so the
  block is about the size of the two rows it replaces, which were 57 words: the skill grows by no more than
  about forty words from its 1,657. `AGENTS.md` and `CLAUDE.md` get nothing.
- **The entries.** The Tests section of the patterns reference gains one entry for each line, under fixed
  anchors, because `GATE-TIME` WP12's rule messages link to them: `the-smallest-world`, `logic-without-a-device`,
  `simulated-time`, `an-existing-test-binary` and `a-main-tier-test`. An entry says what to write, with the call
  or command to copy, and what a builder does when the habit cannot hold. For a full-size or long run that is: it
  needs a done-when item that says what only it can show, so a builder with none writes the proposal in its
  module's log for a planner. The entries state no limit of their own. The ten-second bound, the tiers and the
  guard rules are in [the test tiers](../../../ORCHESTRATE.md#test-tiers), and an entry links there, so one
  place changes when a limit does. The entry on an existing test binary holds before and after `GATE-TIME` WP11:
  a test goes in a test file that exists, and where a package has the one test target `it`, a new file is a
  module of it. `GATE-TIME` WP12 adds each rule's name to the entry its message links to.
- **Simulated time.** The entry on time separates two waits. A test that needs the world to advance steps the
  engine's clock by ticks or frames, and never sleeps or polls the wall clock for it. A test that waits on the
  device, for a readback or a submission, uses the kit's `wait::until`, which `test-deadline` already holds.

WP6 is built before WP3, as WP5 was: it converts no test, so it can run beside any milestone, and `GATE-TIME` WP12
waits for it.

## Interfaces

| Interface | For |
|---|---|
| The gate: ask for a GPU, optionally one backend; run, skip with a `thrum-gpu-skip:` line on standard error, or fail under `THRUM_REQUIRE_GPU=1` | Every GPU test and bench |
| The test device, from the recipe, by profile and optional restriction | Every GPU test outside `thrum-render` |
| `thrum-fixtures`: fixtures as plain data, one definition each | `thrum-render`'s tests (dev-dependency), the proving ground (normal dependency) |
| `test_role` and `proving_ground` in `.config/architecture.toml`, and the `test-crate-edges` rule | `REPO-RULES`' `library-env`, `GAME-SPLIT`'s boundary |
| The kit: readback, the prologue, the wait, scratch folders, the PPM writer and the counting allocator type; the record reflection check through the `render` feature | Every GPU test under `crates/` and in the kit scenes |
| The rules `test-readback` and `test-deadline`, each a count that can only fall | Every builder who writes a GPU test: read back and wait through the kit |
| The small target, `thrum_test_support::target`: a colour target, and a depth target where asked, at the kit's one small size, with the size as a constant | Every new GPU test that draws, and `GATE-TIME` WP9 and WP3 where they shrink a target |
| The block on writing a test in the engineering skill, and the five pattern entries under the anchors `the-smallest-world`, `logic-without-a-device`, `simulated-time`, `an-existing-test-binary` and `a-main-tier-test` | Every builder who writes a test. `GATE-TIME` WP12, whose rule messages link to the entries and which adds each rule's name to its entry |
| The cross-target type-check, `scripts/check_cross_target.sh`, and the main-branch check `cross-target` that runs it | Every module that writes a test: a change that breaks the Linux or Windows build of one is its module's to fix. `GATE-TIME`, which owns the tiers, names the check with the main tier's others |

## Files each milestone touches

The lead confirms these against the builders at work before it starts a milestone. A milestone touches nothing
outside its row; a builder that finds it must writes the proposal in the log and ends `BLOCKED`. Three files are
everyone's: `.config/shape-baseline.json`, `docs/design/maturity.json` and `Cargo.lock`. Change them when your
landing moves them, whatever your row says. The rows are in build order: WP5 and WP6 are built before WP3.

| Milestone | Files | Waits for | Who else is near |
|---|---|---|---|
| WP2 | Besides the conversion, which is in the tree: the type-check's script and its stand-in compilers, under `scripts/`; the test files and dev-dependencies of whichever package the type-check or WP2's questions name, which today are `crates/thrum-asset-gpu` and `examples/water/tests/`; one line of `ORCHESTRATE.md` | Nothing more | `GATE-TIME` WP1 edits `examples/water/tests/floating_gpu.rs` and the same list in `ORCHESTRATE.md`, and `WATERS` builds in the water scene's tests: whichever lands second rebases |
| WP5 | `crates/thrum-test-support/` (the kit's functions, their tests, the manifest); `crates/thrum-render/src/test_device.rs`; the two rules in `scripts/shape_matchers.py` and `scripts/shape_rules.py`, their fixtures in `scripts/test_shape_rules.py`; the Tests section of the engineering patterns | Nothing outside this module | It converts no test, so it can run beside any milestone. `DEAD-TOOLS` WP6 adds four rules to the same rule files: whichever lands second rebases. Both scripts are imported through a pinned comparison file, so each keeps its name and what that file uses |
| WP6 | The new `target` module of `crates/thrum-test-support/` with its test; the adopted-shapes table and the new block of `.agents/skills/thrum-engineering/SKILL.md`; the Tests section of that skill's `references/patterns.md` | Nothing | It converts no test, so it can run beside any milestone. `GATE-TIME` WP12 edits the same Tests section after it, and WP5 of that module the contributor guide, which says the same things for a person |
| WP3 | Test files only: the integration tests and unit-test modules of every crate under `crates/`, and the `tests/` folders of the kit scenes (`examples/water`, `mirror`, `starter`, `lighthouse` and `stones`); `crates/thrum-test-support/` and `crates/thrum-render/src/test_device.rs` where a conversion needs the kit extended; the exemptions of `test-readback` in `scripts/shape_matchers.py` with their fixtures | `GATE-TIME` WP2 and `DEAD-CODE` WP2, which delete tests and stubs in these files; `GATE-TIME` WP6, because some of these files hold tests behind a Cargo feature that no merge compiled before it | The widest set in track 6, so it runs beside milestones that touch no test file (`DEAD-TOOLS` WP1, WP2 and WP4, `DEAD-CODE` WP5, `GATE-TIME` WP5 and M7) and not, where the lead can choose, beside one whose row names test files: `GATE-TIME` WP3 (`foam_scene_gpu.rs`, `support/island.rs`) and WP4 (the mirror's pool tests, `draw/tests.rs`, `scene_view_absent_gpu.rs`), `DEAD-TOOLS` WP3 (`atmosphere/gpu/tests/aerial_ground.rs`, a readback site), `DEAD-CODE` WP3 (two renderer test files), WP4 (the water scene's proof tests) and WP6. Where one of those is building anyway, whichever lands second rebases. `REVIEW-RENDER` and `HOST` build in the renderer's tests |
| WP4 | The reflection check in `crates/thrum-test-support/`; `crates/thrum-render/tests/wgsl_records.rs`; the `wgsl_records` tests and bound-struct walks of `thrum-sea`, `thrum-asset-gpu`, `thrum-water` and `thrum-clouds`; `examples/v1-game/tests/cooked_assets_gpu.rs`, `cooked_material_gpu.rs` and the test support they share | `REVIEW-RENDER` WP2, which owns the renderer's checker | `GATE-TIME` WP5 narrows the draw-probe pass that runs `cooked_assets_gpu`, in the check command only. `DEAD-CODE` WP5 edits `cooked_rows_gpu.rs`, which this leaves alone |

## Not built here

One unit-test layout for the whole repository. The adopted layout is in the patterns reference and a file moves
to it when a task already touches it; a mass move of test files would collide with every builder. Nor does this
module change what any test asserts.

## What it must not do

Change an assertion, a bound or a reference in any test; remove a test; turn a test that ran on this Mac into one
that skips; edit a registered baseline. Add test guidance to `AGENTS.md` or `CLAUDE.md`, or state a limit in the
skill that the project settings own.

## Proof

The kit is proved as adoption was: by its rules' counts, by the tests' results from the same command before and
after with a GPU required, and by a reviewer reading that no assertion, bound or device changed in a converted
test. The kit's own functions have tests in the support package. `cargo test --workspace` does not run a long
test or a test behind a Cargo feature, so for those the proof is the main-branch check `long-tests`: its run
that covers WP3's commits, compared test by test with its last run on a commit that held none of them.

Both comparisons span other modules' commits, and the cleanup deletes tests on purpose: `GATE-TIME` WP2 and
`DEAD-CODE` WP5 did between WP3's two `long-tests` runs. So WP3 is held to its own commits. A test that passed in
the earlier run and is missing from the later one counts against WP3 unless a commit of another module took it
out of the run, by deleting it, renaming it or taking it off what the command runs. WP3's record names that commit
for each such test, and a reviewer confirms from its diff that it removes the test and that it is another
module's. Whether the removal was right is a question for the review of the milestone that made it, so WP3's
review does not ask it again, and WP3 never restores such a test. A test that is missing with no such commit to
name is WP3's loss.

The small target is proved by the kit's own test, which draws to it and reads it back, and which a reviewer
reads. The block and the entries are proved by a check that the two rows are gone, the five links and anchors
exist and the skill has not grown past about forty words, and by a reviewer who reads each entry beside the test
tiers.

Linux and Windows are proved as [mechanism 7](#other-platforms) says: the type-check passing for both targets,
and two questions a reviewer answers by reading. No item claims a run there.

The gate is proved from outside: its tests run a child test process with the adapter forced absent and read the
child's exit code and output, in both modes. Adoption is proved by the rules at zero, and by what ran: the tests'
results from the same command before and after, the later run made with `THRUM_REQUIRE_GPU=1` and holding no skip
line. A list of test names shows that none was removed; it does not show that one ran, so it is never the only
evidence.
