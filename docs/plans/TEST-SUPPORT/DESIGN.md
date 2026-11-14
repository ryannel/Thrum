# Test support: one GPU gate, one test device, shared fixtures

14 November 2026, revised 15 November after [the first design review](../../evidence/TEST-SUPPORT/design-review.md).
[Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation) ·
[the code review](../../evidence/code-review-2026-11-14/CODE-REVIEW.md), theme T6 ·
[patterns: tests](../../../.agents/skills/thrum-engineering/references/patterns.md#tests)

## The request

The owner asked for the review's fixes to be planned and driven. The lead's task for this module: "test support
and one GPU gate (T6)", after the confirmed bugs and the typed records, and before the device recipe.

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
  but it does not capture the handle: [a probe](../../evidence/TEST-SUPPORT/planning/skip-line-probe.md) on the
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

Until `HOST` M1 lands the recipe, M1 here answers the same request (profile and restriction) from a stand-in in the
support package, with the features `DeviceCapabilities` already names. M2 replaces the stand-in's body with the
recipe; no test changes again.

**4. Fixtures are plain data, and each exists once.** A fixture is data and the functions that build it, with no
type of any game or engine crate; each consumer converts it to its own types at its edge. This module owns the
fixtures package, its rule and what a fixture is. The first fixture is the mixed-mesh field: its entities,
materials, camera path, prop meshes and the day cycle its tests light it with, which `thrum-render`'s visibility
tests compile from the game's source today. [GAME-SPLIT](../GAME-SPLIT/DESIGN.md) M2 creates the package with that
fixture, because the visibility tests are held until then; after it, the field's data is defined in
`thrum-fixtures` and nowhere else. A fixture that is one crate's own data stays in that crate behind its API.

**5. Rules hold the shape.** `device-request` (no adapter or device request outside the recipe and the gate),
`whole-file-gpu-gate` and `path-attribute` are [REPO-RULES](../REPO-RULES/DESIGN.md)' ratchets. This module brings
them to zero: first in the crates no builder is working in, then across the renderer and the examples.

## Interfaces

| Interface | For |
|---|---|
| The gate: ask for a GPU, optionally one backend; run, skip with a `thrum-gpu-skip:` line on standard error, or fail under `THRUM_REQUIRE_GPU=1` | Every GPU test and bench |
| The test device, from the recipe, by profile and optional restriction | Every GPU test outside `thrum-render` |
| `thrum-fixtures`: fixtures as plain data, one definition each | `thrum-render`'s tests (dev-dependency), the proving ground (normal dependency) |
| `test_role` and `proving_ground` in `.config/architecture.toml`, and the `test-crate-edges` rule | `REPO-RULES`' `library-env`, `GAME-SPLIT`'s boundary |

## Not built here

One unit-test layout for the whole repository. The adopted layout is in the patterns reference and a file moves
to it when a task already touches it; a mass move of test files would collide with every builder. Nor does this
module change what any test asserts.

## What it must not do

Change an assertion, a bound or a reference in any test; remove a test; turn a test that ran on this Mac into one
that skips; edit a registered baseline.

## Proof

The gate is proved from outside: its tests run a child test process with the adapter forced absent and read the
child's exit code and output, in both modes. Adoption is proved by the rules at zero, and by what ran: the tests'
results from the same command before and after, the later run made with `THRUM_REQUIRE_GPU=1` and holding no skip
line. A list of test names shows that none was removed; it does not show that one ran, so it is never the only
evidence.
