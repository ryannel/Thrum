# Patterns

Each section covers one recurring job:

- the shape the codebase has adopted;
- why it was chosen;
- the older shapes you will meet and should not copy;
- what to do where no shared helper exists yet.

The 14 Nov 2026 code review (`.orchestrate/evidence/code-review-2026-11-14/CODE-REVIEW.md`, in the working folder) found each split this file
describes. Its finding IDs are given so you can see the evidence. `./dev check` holds the mechanical shapes by rule
(`scripts/shape_rules.py`); `docs/plans/ARCHITECTURE.md` says which module fixes each
finding.

The rules count spelled-out text, not type aliases or macro expansion. Existing sites are inventoried in
`.config/shape-baseline.json`; after removing sites, run `python3 scripts/shape_rules.py tighten`. It records
the current counts but refuses growth above the lowest total already committed for that rule/version.
Moving a site carries its allowance. Editing the baseline cannot grant new allowance. `check --rule NAME
--zero PATH...` requires no sites under those paths; for `history-names` it lists names still pinned by an open
check, xtask or the exact-baseline registry. Without Git history only the tree counts are checked, with a notice.

Contents: [Errors](#errors) · [Data shared with WGSL](#data-shared-with-wgsl) ·
[Shader source](#shader-source) · [Naming GPU work](#naming-gpu-work) ·
[Device setup](#device-setup) · [Tests](#tests) ·
[Configuration](#configuration) · [Crossing crate boundaries](#crossing-crate-boundaries) ·
[Platform shells](#platform-shells) · [Names](#names)

## Errors

Rule: `string-errors` refuses a public `pub fn` returning `Result<_, String>`, including multiline returns.

**Shape.** Each public boundary has one error enum. `crates/thrum-asset/src/error.rs` is the model:

- each variant has a doc line saying what happened;
- `Display` writes a message a person can act on;
- `std::error::Error::source()` returns any wrapped error.

Variants name the decision the caller has to make (reject the input, retry later, treat it as a fault), not where
the error happened. Private helpers return the boundary's enum, or a private error converted at the edge.

**Why.** A `String` can't be matched, so callers end up classifying errors by guessing. `thrum-render` has a blanket
`From<String> for FrameError`, and as a result the same missing presenter is reported as a GPU error from
`resize_surface` but as `InvalidState` from `drive` (H11).

**Older shapes not to copy:**
- `Result<_, String>`, which is most of `thrum-app` and the games.
- Typed enums with no `Display` or `Error` impl, which is most of `thrum-core`, `thrum-network` and `thrum-render`.
- `Box<dyn Error>` outside binaries and tools.
- `synthetic::RenderError`. It is the crate-wide render error, but it lives in the synthetic module, so `atmosphere`
  has to import its error from its sibling (H06).

When you touch one of these at a boundary you are already changing, convert that boundary.

## Data shared with WGSL

**Shape.**
- A record that a shader reads is a `#[repr(C)]` struct deriving `bytemuck::Pod` and `Zeroable`, written whole with
  `bytemuck::bytes_of`. `crates/thrum-asset/src/layout.rs` shows the derive. `bytemuck` is already pinned at
  `=1.25.2` in the workspace.
- Each such record has a test that parses the WGSL with `wgpu::naga` and checks the struct's size and member offsets
  against the Rust side. Copy the test at the end of `crates/thrum-render/src/synthetic/view.rs`.
- A WGSL struct used by several shaders is declared once and included, not copied into each shader.
- Each buffer has exactly one writer.

**Why.** A drift between the Rust and WGSL layouts doesn't fail. It shades wrongly, silently. The review found:
- about 44 hand-written byte offsets in two synthetic records with no layout test (H07);
- a WGSL `Serials` struct whose copies already disagree (M06);
- one light buffer written by the CPU for bytes 0..48 and by a GPU pass for bytes 48..208, split by the literal
  `48` (H09).

**Older shapes not to copy:** `put(data, offset, value.to_le_bytes())` and `[u8; N]` records packed by literal
offsets. These are in `thrum-sea/src/upload.rs`, `synthetic/`, `atmosphere/` and `visibility/`.

**Constants shared with WGSL** are generated into a prelude from the Rust value, as `Profile::shader` does in
`crates/thrum-render/src/atmosphere/spectrum.rs`. A number typed into both a `.rs` and a `.wgsl` file will drift.

## Shader source

**Shape.**
- A `.wgsl` file sits beside the Rust module that owns it and is loaded with `include_str!`.
- Generated constants are prepended as above.
- A variant of a shader is a separate entry point or a generated constant, not an edited copy of the text.

**Older shapes not to copy:**
- Variants made by exact-text `String::replace` (synthetic, `thrum-asset-gpu`).
- WGSL cut out of another module's file at comment markers. `atmosphere/products.rs` builds its library by slicing
  `synthetic.wgsl` and renaming with `.replace` (M27).
- Tests that reassemble production shader lists by hand.

A text edit fails at run time, on the device, the first time anyone renames the text it matched.

Rule: `shader-text-edit` counts `.replace(` and `.replacen(` in non-test Rust files that mention WGSL. It is coarse
on purpose, and its count only falls.

## Naming GPU work

**Shape.**
- A pass name is a `const PASS: &str` in the module that owns the pass, as in
  `crates/thrum-render/src/visibility.rs`. The graph declaration, the GPU timing buckets and the benches all use that
  constant.
- Pipelines, stages and resources inside a module are named by a typed key, such as an enum or named struct fields.
  `thrum-asset-gpu` holds its resources in named fields (`GpuMesh`, `Bound`, `Pipelines`), and that is the shape to
  copy.

**Why.** A string or integer identity breaks quietly. Renaming a pass string zeroes its timing bucket. An atmosphere
stage index means different things in five tables, so if two of them drift the graph drops a barrier the shader
needs (H05, H08). `thrum-sea` binds 17 buffers and 13 passes by position (H17).

**Older shapes not to copy:**
- Role integers such as `0..9`, `0x5343` and `1000` in `PipelineKey.role`.
- Hex ASCII owner tags.
- Positional `Vec<wgpu::Buffer>` indexed by literals.
- A recorder found by reverse lookup in a long `match` (`synthetic/scene_graph.rs`).

## Device setup

Rule: `device-request` counts adapter/device calls outside HOST's capabilities module and the registered
test-support package. Use the recipe and send tests through the gate.

**Shape.** `DeviceRecipe` selects features and limits by profile and returns the device, queue, capability record
and fault watch. A host can supply a surface and a wake callback. A test uses the recipe on the adapter admitted
by the test gate. Pipelines are created through `DeviceSite::render` / `compute` (`runtime/creation.rs`), which
records each creation.

## Tests

Rules: `path-attribute` refuses `#[path]` (use normal modules); `whole-file-gpu-gate` refuses an OS condition
that removes tests from a harness: a file-level `#![cfg(...target_os...)]` on a test file, or an OS condition on any
module, product or test, whose declared subtree holds a test (gate the individual GPU tests). Use
`cfg(any(test, target_os = "..."))` to keep a portable module's tests everywhere. A platform shell whose tests drive
its own platform APIs may keep its condition with a `// platform-only tests: <reason>` comment directly above.
`test-readback` counts a buffer mapped for reading (`map_async` or `map_buffer_on_submit` with `MapMode::Read`) and
`test-deadline` a hand-written `assert!(Instant::now() < deadline)`, in test files outside the kit. Both counts only
fall: a new test reads back and waits through the kit.

**Shape.**
- Unit tests go in `#[cfg(test)] mod tests;`, with the body in `foo/tests.rs` once it passes a screen or two.
- `tests/` holds tests of the crate's public API.
- Test support shared by several test files lives in `tests/support.rs` or `tests/support/`, declared as a normal
  module.
- Production code carries no test-only functions, fields or branches. To inject a fault, pass a trait whose
  production implementation is zero-cost.

**GPU tests.** Ask `thrum_test_support::gpu(test_name, backend)` for admission. Return on `None`:
the gate writes a visible reason to stderr, and fails instead under `THRUM_REQUIRE_GPU=1`.
Use only a dev-dependency on this package. `test-crate-edges` enforces that boundary, and permits
normal dependencies on a registered fixtures package only from the registered proving ground.

Gate only the tests that need a GPU, and pass the specific backend for a backend-bound test. A test that asks for
any backend and then asserts Metal, or opens the `apple-tile` recipe, fails on a Linux or Windows adapter instead of
skipping; admit `Some(wgpu::Backend::Metal)`. A test that runs on any backend may assert Metal only under
`#[cfg(target_os = "macos")]`. The CPU graph
validation cases use portable limits and feature facts and compile on every target.

**The kit.** `thrum-test-support` writes once the jobs every GPU test repeats:
- `open(test, backend, profile, &restriction)` (the `render` feature) asks the gate, on `backend` where given, and
  returns the recipe's device, queue and capabilities, or `None` for a reported skip. `thrum-render`'s unit tests
  call `crate::test_device::open`, the same call typed by the crate.
- `readback::buffer(device, queue, &buffer, range)` and `readback::texture(device, queue, &texture, Mip, range)`
  submit their own copy and wait at most `readback::WAIT`; texture bytes come back packed, without row padding.
  A test that must read inside the encoder under test appends `Readback::buffer` or `Readback::texture` to it and
  calls `wait(device, submission)`.
- `target::new(&device, target::Depth::Without)` (or `Depth::With`) returns the one small render target a GPU test
  draws to: `target::SIZE` texels square in `target::FORMAT`, with a depth target where asked, and
  `Target::read(device, queue)` reads its colour back packed.
- `wait::until(what, limit, || -> Option<T>)` for anything else a test waits on, failing at its deadline.
- `Scratch::new(name)`, a temporary folder removed on drop; `ppm::write` for an image a person opens.
- `CountingAllocator` with `allocations::count(|| ..)`, counting allocations and bytes on the calling thread. Each
  counting test binary declares its own `#[global_allocator]` static of it.

**Test devices.** Integration tests can enable the support package's `render` feature and open the admitted
adapter with a portable or native profile. Restrictions remove features or lower limits. Renderer unit tests
call their own build of `DeviceRecipe` so they use the same renderer types as the code under test.

**Writing a test.** Five habits, which the skill's block links to. The limits they serve, the ten-second bound
and the tiers, are in [the test tiers](../../../../ORCHESTRATE.md#test-tiers); these entries state none of
their own.

<a id="the-smallest-world"></a>

*Prove a behaviour on the smallest world that shows it.* A world is what the test builds to run on: a grid, a
cooked store, a count of entities, ticks or frames. Choose the smallest one where the behaviour appears and each
assertion could still fail, such as a grid whose solve takes more than a few steps rather than the showcase
valley. Build it once in the test, not once per case. When the behaviour only appears at full size, keep the small
test that proves the behaviour itself, and treat the full-size run as [a main-tier test](#a-main-tier-test).

<a id="logic-without-a-device"></a>

*Test logic on the CPU, and open a device only for what a GPU alone can show.* A selection, a layout, a state
machine, a record's bytes, a refusal or arithmetic a model shares with a shader is a test that asks for no GPU. A
GPU test covers what only a device shows: a shader compiles and a pipeline is accepted under its limits,
submitted output or GPU state equals what the CPU side states, a device's lifecycle holds. It asks the gate, draws
to the kit's target for the fewest frames that show it, and reads back through the kit:

```rust
let Some(gpu) = thrum_test_support::gpu("draws_the_quad", None) else { return };
let (device, queue) = gpu.portable(None); // or `open(..)`, or the crate's own test device
let target = thrum_test_support::target::new(&device, thrum_test_support::target::Depth::Without);
// draw to target.colour.view; a test driving a host passes target::SIZE to it
let texels = target.read(&device, &queue);
```

A test whose subject is a size, such as a resize or a tiled recipe, keeps its own target and says why in a
comment. A plan item that adds a GPU test says what only the device shows.

<a id="simulated-time"></a>

*Step simulated time, never wait on the clock.* The engine's clock takes time as an argument, so a test that
needs the world to advance passes the times it wants: `TickClock::new(config, Duration::ZERO)` then
`clock.advance(Duration::from_millis(300), budget, step)`, or `ClockedWorld::advance(now)`, or a fixed number of
ticks or frames. It never sleeps or reads `Instant::now()` to make time pass. A wait on the device or another
thread is not simulated time: a readback waits through `readback::buffer` or `readback::texture`, and anything
else through `wait::until(what, limit, attempt)`, which fails at its deadline.

<a id="an-existing-test-binary"></a>

*Add to an existing test binary.* Each file under `tests/` is a binary that macOS checks the first time it runs.
Put a new test in a test file that already exists for its subject, or in the unit tests of the module it tests
(`#[cfg(test)] mod tests;`), which add no binary. Where a package has the one test target `it`, a new test file is
a module of it, declared there and run as `cargo test -p <package> --test it <file>::`. When neither fits,
because the test must run apart, such as behind a Cargo feature or with `harness = false`, the item that asks for
it says so.

<a id="a-main-tier-test"></a>

*A full-size or long run is a main-tier test and needs a plan's say.* It needs a done-when item that says what
only it can show, which the smallest world cannot. A builder with no such item does not add it: write the
proposal, with the fault it would catch, in the module's `LOG.md` for a planner, and keep the small test. With an
item, mark the test `#[ignore = "long: ..."]`, name its binary in `.config/long-tests.txt` with the fault it
leaves unguarded before a merge, and run it with `./dev long-tests`, which the main-branch check `long-tests`
runs on main.

**Older shapes not to copy:**
- Support shared by `#[path = "..."]` includes, which rely on implicit `super::` names.
- `#[path = "x_tests.rs"]` sibling files.
- Test files named after milestones (`g2_runtime.rs`, `g2s_chunked_kernel.rs`). Name a test file for its subject.

## Configuration

Rule: `library-env` refuses `std::env::{var,var_os,vars}` calls in non-test library source. Parse at the process
edge and pass typed settings. Binaries and build scripts are process edges; only a package registered with
`test_role = "support"` is exempt as a library. A fixtures role grants no exemption.

**Shape.** A host parses its settings once, at startup, into a typed struct, and passes the values down. A setting
read from the environment uses a public constant for the variable's name, as `CAPTURE_VARIABLE` does in
`thrum-app`.

**Why.** There are about 87 `THRUM_*` variables, read at the point of use in 41 files (M16). That includes library
crates: `thrum-render` reads `THRUM_RECIPE` and `THRUM_MAC_PATTERN` (M08). A setting read deep inside a library can't
be found, typed or tested. A library crate takes its settings as arguments and doesn't read the environment.

## Crossing crate boundaries

Rule: `package-reach` refuses literal path attributes/includes leaving their own package and every kind of
engine-to-example manifest dependency, including target-specific and renamed dependencies. Expose a public API
or use a registered shared fixture package instead.

An engine crate depends only on the crates `.config/architecture.toml` allows. These all create a hidden dependency
that neither check sees:

- `#[path]`;
- `include_str!` or `include_bytes!` that leaves the crate's own folder;
- a dev-dependency on an example.

The review found these:

| Hidden dependency | Where |
|---|---|
| Render tests compile three `examples/v1-game/src` files | H02 |
| Six sites read `thrum-render/src/synthetic/temporal/witness.json` | M72 |
| `thrum-asset-gpu` reads render's private `cloud.wgsl` | H02 |
| `thrum-cook` dev-depends on `examples/asset-loop` | M73 |

A fixture two crates need lives in the crate that owns the data, exposed through its API (behind a feature if it is
test-only), or in a shared dev-only crate registered in `architecture.toml`.

Qualification and diagnostic code for the engine belongs to the engine, not inside the showcase game's public
library (H20).

## Platform shells

The macOS shell (`thrum-app/src/app`), the browser shell (`thrum-app/src/web`) and the iOS shell
(`examples/v1-game-ios`) each implement frame stepping, device loss and recovery, and pause and overload policy on
their own. The copies have drifted (H15, M02, M03).

- **A fix to shared behaviour:** make it in every shell, or tell the planner that this behaviour needs one owner.
- **A new shell feature:** look for the same feature in the other shells first, and match it.
- **Policy that isn't about the platform:** pause and overload decisions belong beside `ClockState` in `thrum-core`,
  not in a shell.

## Evidence

Evidence is not committed. Run records, review captures and logs go to the working folder,
`.orchestrate/evidence/<MODULE>/`; data a check reads is test data, tracked in `tests/fixtures/<module>/` or
beside the crate's tests; a provenance record is tracked at `docs/provenance/<MODULE>.md`. The merge refuses a
new or changed file under `docs/evidence/`. Keep large captures in `tmp/<task>/`; if a reviewer needs the bytes,
`python3 scripts/evidence_store.py put SOURCE --path docs/evidence/MODULE/FILE` keeps them in the local store
under that logical name and records them in `.config/evidence-manifest.json`, the only file to commit. The store
is shared across worktrees on this Mac, with no remote backup. Restore an ignored copy with
`python3 scripts/evidence_store.py restore docs/evidence/MODULE/FILE`.
See [the storage rules](../../../../docs/ASSET-LIFECYCLE.md#bulk-review-evidence).

Rule: `evidence-path` counts lines of source under `crates`, `examples`, `scripts` and `tools` that name the
tracked evidence folder, except the evidence store and its suite. Run records go to the working folder,
`.orchestrate/evidence/<MODULE>/`, and data a test reads goes in `tests/fixtures/<module>/` or beside the
crate's tests.

## Scripts and benches

Rules: `script-caller` counts files under `scripts/` that nothing names, and `bench-runner` counts `[[bench]]`
targets that nothing runs. Each count only falls.

- **A new script** is named by what runs it: `./dev` or `tools/xtask`, a CI workflow, a `Main-branch check` line,
  Rust source, another named script, or a tracked document outside the plans folder. A proof tool is named by the
  done-when item whose check runs it, whether that milestone is open or Done. A suite `test_X` lives with the
  script `X` beside it.
- **A new bench** is run by a script, a `./dev` command, a CI workflow or a document, with its name spelled out,
  as in `cargo bench --bench NAME`.
- **A landing that removes a last caller** deletes what it orphans, or names it elsewhere, in the same landing.
- Plans prose and records under the evidence folder never count as callers.

## Names

Rule: `history-names` counts milestone/gate segments in file paths, environment variable names in source,
Cargo target names and descriptions. Use subject names such as `asset_reload`, not `g5_reload`; `rgba8` and
`vec3` are ordinary names. Only `--zero` exempts dynamically discovered pinned names; the baseline inventories
all existing names so an unrelated plan's status change does not change its counts.

Name things for what they do. Many file names, environment variables and some identifiers carry milestone and gate
codes (`g2s_`, `g2r`, `t2_`, `G3c`, `M3b`, `G5`). A reader without the history can't tell what they mean. Keep codes
in plans, logs and evidence.

When you rename, rename completely:

- the item;
- its file;
- its doc comments;
- every caller.

A half-rename leaves two names for one thing. `synthetic` is the production scene renderer, and renaming it is
planned work. Don't rename it piecemeal.
