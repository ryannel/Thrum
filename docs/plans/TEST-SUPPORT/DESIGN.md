# Test support: one GPU gate, one test device, shared fixtures

14 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
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

**1. A dev-only crate, `crates/thrum-test-support`.** It is registered in `.config/architecture.toml` and may be a
dev-dependency only; a rule refuses any other edge. It holds the gate, the test device and fixtures that more than
one package's tests need.

**2. The gate asks a question with one answer.** A GPU test starts by asking the gate for a GPU.

- With an adapter, the test runs.
- With none, the gate prints one line with the test's name and the reason, and the test returns. Rust's test
  harness cannot report a skip at run time, so the printed line is the report.
- With `THRUM_REQUIRE_GPU=1` and no adapter, the test fails. `./dev check` sets the variable on macOS, the one
  platform every target runs on, so on the Mac a missing adapter can never pass for a run. Test support is a
  process edge, so it may read the environment.
- A test that needs one backend says so to the gate (`Metal`, for the native segment), and is gated on the test,
  never on the file.

**3. The device comes from the recipe, and the gate is wgpu-level.** A crate that depends on `thrum-render` cannot
hand `thrum-render`'s own unit tests a value typed by `thrum-render`: Cargo builds the crate under test a second
time, and the two copies' types differ. So the gate itself deals only in wgpu types (instance, adapter, the skip),
and has no dependency on an engine crate. The device then comes from [HOST](../HOST/DESIGN.md)'s one recipe in
`thrum-render`: `thrum-render`'s tests call it directly, and every other crate gets gate and recipe together
through a `render` feature of this crate. Until `HOST` M1 lands the recipe, M1 here requests the device with the
features `DeviceCapabilities` already names, in this crate alone.

**4. Fixtures are plain data.** A fixture here is data and builders with no type of any game or engine crate: the
first is the mixed-mesh field that `thrum-render`'s visibility tests draw, which they compile from the game's
source today. [GAME-SPLIT](../GAME-SPLIT/DESIGN.md) M2 adds it. A fixture that is one crate's own data stays in
that crate behind its API.

**5. Rules hold the shape.** `device-request` (no adapter or device request outside the recipe and the gate),
`whole-file-gpu-gate` and `path-attribute` are [REPO-RULES](../REPO-RULES/DESIGN.md)' ratchets. This module brings
them to zero: first in the crates no builder is working in, then across the renderer and the examples.

## Interfaces

| Interface | For |
|---|---|
| The gate: ask for a GPU, optionally one backend; run, skip with a printed reason, or fail under `THRUM_REQUIRE_GPU=1` | Every GPU test and bench |
| The test device, from the recipe | Every GPU test outside `thrum-render` |
| Fixtures as plain data | `thrum-render`'s tests, the proving ground |

## Not built here

One unit-test layout for the whole repository. The adopted layout is in the patterns reference and a file moves
to it when a task already touches it; a mass move of test files would collide with every builder. Nor does this
module change what any test asserts.

## What it must not do

Change an assertion, a bound or a reference in any test; remove a test; turn a test that ran on this Mac into one
that skips; edit a registered baseline.

## Proof

The gate's own tests force the no-adapter path in both modes. Adoption is proved by the rules at zero and by test
lists taken before and after, which must match one to one.
