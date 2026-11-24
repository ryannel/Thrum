---
name: thrum-engineering
description: "How Thrum's engine code is laid out and written: where each responsibility lives, the shapes the codebase has adopted for recurring jobs and why, and the engine rules the orchestrate skills leave to the project. Use when writing, changing or reviewing Rust, WGSL, tests or tools in this repository, or when planning, building or reviewing a Thrum module."
---

# Thrum engineering

Thrum is a reusable, native-first Rust 3D engine. Agents write and maintain nearly all of it, so it stays workable
only if an agent landing cold in any file can guess where things are and how they work. This skill gives you that
map, the shapes the codebase has adopted for recurring jobs, and the engine facts you could not work out alone. The
orchestrate skills (lead, planner, builder and reviewer) cover how work runs. `ORCHESTRATE.md`
holds this project's settings for them.

## Extend the shape that exists

Most of the code's problems come from one habit: each work package copied the nearest working example, so the same
job is now done three to five ways. GPU data is packed by hand in every GPU crate. Errors come in five shapes. Device
setup is written once per host. Each copy then drifted from the others. One drift means the sea can never prepare
on iPhone: the iOS shell asks for fewer storage buffers than the sea needs.

So, before you write code that does a recurring job (the table below lists them), find how the codebase does it
now, starting from the reference file. Extend that. When you find two shapes for one job, use the adopted one, and
note the other in the plan log so the planner can schedule the cleanup. Don't add a third shape. If the adopted
shape really doesn't fit, write why in the plan log before you depart from it; a quiet new variant becomes the next
agent's template.

Clean up older shapes only inside the code your task already touches. A repo-wide migration is planned work, not a
side effect of a fix.

## Where things are

| Path | What lives there |
|---|---|
| `crates/thrum-core` | Simulation: world, schedule, clock, publication pages, snapshot ring, world lanes. No rendering or I/O |
| `crates/thrum-render` | GPU runtime (`DeviceSite`, frame graph, `FrameHost`, presentation, timing, capabilities), the scene renderer, atmosphere and GPU visibility |
| `crates/thrum-asset` | Cooked product format and loading: mapped, validated, content-addressed |
| `crates/thrum-asset-gpu` | GPU residency for cooked products: staged uploads, meshlet culling, the cooked-mesh draw |
| `crates/thrum-cook` | Authoring-only cooker, cache, watcher and dev server. Never a runtime dependency |
| `crates/thrum-sea-model`, `thrum-cloud-model` | CPU models in `f64`, with no graphics dependency: what a game queries headless, and a diagnostic for the GPU paths |
| `crates/thrum-sea` | The GPU sea, on the scene effect graph |
| `crates/thrum-app` | Platform shells: the native macOS window and render thread, the browser WebGPU shell, the sky kit |
| `crates/thrum-controls`, `thrum-network` | Demo controls, network codecs |
| `examples/v1-game` | The V1 test game, which uses the engine only through public APIs. Its iOS shell is `examples/v1-game-ios` |
| `examples/starter`, `stones`, `lighthouse` | Small kit scenes that teach the public API |
| `tools/xtask`, `dev`, `scripts/` | `./dev` commands (xtask) and the Python check and capture tools they call |
| `.config/architecture.toml` | Registered crates and the workspace dependency edges each may have |
| `docs/plans/<MODULE>/` | Each module's governing `DESIGN.md`, `PLAN.md`, `DONE.json` and `LOG.md`. These win over older docs |
| `docs/design/` | Contracts (`CONTRACTS.md`), the maturity ledger (`maturity.json`), decisions and specifications |
| `tmp/<task>/` | Ignored scratch work. Bulk output follows `docs/ASSET-LIFECYCLE.md` |

Two names mislead. `thrum-render`'s `synthetic` module is the production V1 scene renderer, even though its docs
still call it diagnostic. `docs/design/modules/*.md` are earlier family designs; where they disagree with
`docs/plans/<MODULE>/DESIGN.md`, the plan folder wins.

Engine crates never depend on a game, an example or a tool. That includes compile-time reach: a `#[path]`,
`include_str!` or `include_bytes!` that points outside the crate's own folder is a hidden dependency that neither
Cargo nor `architecture.toml` sees. The same goes for one engine crate reading another's private files.

## The adopted shapes

Each row names the shape to extend and a file to copy from. [Patterns](references/patterns.md) gives the reason for
each, the older shapes you will meet and should not copy, and what to do where no shared helper exists yet. Read
the matching section before you write code that does one of these jobs.

| Job | Adopted shape | Reference |
|---|---|---|
| Errors | One enum per public boundary, with `Display` and `std::error::Error`, a doc line per variant, and `source()` for wrapped errors. No `Result<_, String>` in a `pub fn` | `crates/thrum-asset/src/error.rs` |
| Data shared with WGSL | A `#[repr(C)]` `bytemuck::Pod` struct written whole, and a naga test that checks the WGSL struct's size and offsets | `crates/thrum-asset/src/layout.rs`; test in `crates/thrum-render/src/synthetic/view.rs` |
| Constants shared with WGSL | Generated into a WGSL prelude from the Rust value | `Profile::shader` in `crates/thrum-render/src/atmosphere/spectrum.rs` |
| Pipelines | Created through `DeviceSite::render` / `compute`, which records each creation | `crates/thrum-render/src/runtime/creation.rs` |
| Pass names | A named `const PASS: &str` in the owning module, used by the graph, timing and benches | `crates/thrum-render/src/visibility.rs` |
| Device request | `DeviceRecipe` selects the profile, applies restrictions, and returns the device, capability record and fault watch | `crates/thrum-render/src/capabilities/device.rs` |
| Modules | `foo.rs` plus a `foo/` folder; no `mod.rs` (lint-enforced) | any crate |

**Writing a test.** Each line links to its entry. [Patterns' Tests](references/patterns.md#tests) also gives the
unit-test layout and GPU test admission.
- Prove a behaviour on [the smallest world](references/patterns.md#the-smallest-world) that shows it.
- Test [logic on the CPU](references/patterns.md#logic-without-a-device) and open a device only for what a GPU alone
  can show, at the test kit's small target.
- [Step simulated time](references/patterns.md#simulated-time), never wait on the clock.
- Add to [an existing test binary](references/patterns.md#an-existing-test-binary).
- A full-size or long run is [a main-tier test](references/patterns.md#a-main-tier-test) and needs a plan's say.

| Job | Adopted shape | Reference |
|---|---|---|
| Construction | `new(..) -> Result<Self, E>` that validates up front; a documented `*Config` struct for many settings | `CookConfig` in `crates/thrum-cook/src/cooker.rs` |
| Names | Say what the thing does (`transmittance`, `PublicationPage`). Planning IDs (`G2S`, `G5`, `WP7`) stay in plans and evidence | — |
| GPU labels | Spaced plain words: `"camera visibility"` | `crates/thrum-render/src/visibility.rs` |
| Maths | `glam` for vectors and matrices | `crates/thrum-render/Cargo.toml` |

## Engine rules

**One owner for each kind of state.** Before you add a producer, look for the existing owner: the one sky and
atmosphere, the frame contract, the capability registry, cooked products, the maturity ledger. A shared invariant
belongs in `docs/design/CONTRACTS.md`. Change a contract with its producers and consumers before you build on the
change; a builder proposes the change in the plan log and leaves it to the planner.

**Engine crates hold no game schema.** A crate under `crates/` never names a game's controls, scenes or types. A
game reaches the engine through public APIs, and the engine reaches a game only through traits the game implements.

**Match rigour to what the code touches.**

| What the code touches | On a violation |
|---|---|
| Authoritative world state, saves, wire input, security boundaries | Refuse with a typed error, leave state unchanged, and test the failure path |
| Presentation, effects, audio, frame resources | Degrade: clamp or drop, count it, and `debug_assert!` in development. Never stall a tick or fail a frame |
| Authored content | Validate with tolerances; importers repair what they safely can (weld, drop degenerates) and warn |

**Use the hardware, keep a fallback.** Native leads and uses every feature the device has; the browser runs the
portable recipe and never limits native. Accelerated paths are selected from the capability registry, keep a
portable fallback that the same tests exercise, and are promoted only by a measured win
(`docs/design/decisions/HARDWARE-NATIVE-ENGINE.md`).

**Plausible and fast, proved in proportion.** The owner's bar (17 November 2026, in `ORCHESTRATE.md`) is the most
plausible result as fast as possible, not physical accuracy. Rendering and visual work passes when it looks as
good as a shipped high-end game against its inspection list and runs fast. A physical or `f64` reference is a
diagnostic for output that looks wrong, never a pass mark. Correctness is proved where a failure is silent and
costly: networking, simulation determinism, cooked formats and the contracts between modules. Match the weight of
a proof to the risk it covers.

**Cost is designed, then measured.** The owner's policy is best-practice decisions up front, tripwires while
building, tuning later. Use measured performance tripwires. Per-frame and per-tick code reuses its buffers, uploads
only what changed, and doesn't read back from the GPU to make progress.

**Dependencies.** A new crate or external dependency gets an entry in `.config/architecture.toml` and an exact
`=version`. Look for a proven crate before building a general subsystem; build your own where a specialised path is
the point and can show its win.

**Done.** A code change is done when `./dev check` passes; documentation-only work uses `./dev check-docs`. Claims
about devices, frame time or visuals come only from runs on the targets in `ORCHESTRATE.md`, and status goes in
`docs/design/maturity.json` with implementation, correctness, visuals and device recorded separately.

## Which reference to read

- [Patterns](references/patterns.md): before you write code that does one of the jobs in the table above.
- [Rust code](references/rust-code.md): when you design a public type, an error enum, a decoder or test support.
- [Building](references/building.md): when you build a work package: proving it in the game, capture, and what to
  deliver.
- [Performance](references/performance.md): when you design or change performance-sensitive code, or make a
  performance claim.
- [System diagnosis](references/system-diagnosis.md): when a visual or behavioural cue stays wrong after a local fix.
- [Design](references/design.md): when you research or write a module design, or reconcile existing code with one.
- [Architecture](references/architecture.md): when you plan at project level: the module map, planning inputs and proof
  order.
- [Review](references/review.md): when you review a design, a work package or a claim, including the failures this
  engine has already paid for.
