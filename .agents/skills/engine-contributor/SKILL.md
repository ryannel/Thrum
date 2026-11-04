---
name: engine-contributor
description: Implement, refactor or review Thrum engine code toward the active build-plan milestone, and verify it. Use engine-architecture or engine-module-design for whole-engine or new-module design, and repo-structure for file placement and retention.
---

# Engine contributor

Thrum succeeds when engine code runs on named targets within its budget. Judge each change by
whether it moves the active milestone in [the build plan](../../../docs/BUILD-PLAN.md) forward,
not by how much it proves.

## Pick the work

- Take the next unchecked item in the build plan's active order. Prefer the smallest change that
  makes a path run end to end or removes a measured cost.
- Extend running paths before hardening inactive ones. Code with no consumer by the end of a slice
  does not land. Parked tracks stay parked unless the user reopens them.
- Research or design only to unblock a named decision. Keep the record to two pages under
  `docs/design/decisions/`.
- Before building a subsystem, look for a proven crate that meets the targets (Wasm, mobile,
  determinism where needed). Adopt it with a reuse record rather than reimplementing it for
  control you will not use.

## Match rigour to the state

| What the code touches | On violation |
| --- | --- |
| Authoritative world state, saves, wire input, security boundaries | Refuse with a typed error, leave state unchanged, test the failure path |
| Presentation, VFX, audio, cosmetic budgets, frame resources | Degrade: clamp or drop, count it, log it; `debug_assert!` in development. Never stall a tick or fail a frame |
| Authored content | Validate with tolerances; importers repair what they safely can (weld, drop degenerates) and warn |

Do not add exact multi-precision arithmetic, certificates, logical byte-credit accounting or
diagnostic-ordering contracts unless an observed failure needs them and a simpler bound failed.

## Shape the code

The lint policy and `./dev check` enforce suppression discipline, API reachability, test placement
and size backstops. When designing types, APIs, errors, decoders or test support, read
[references/rust-code.md](references/rust-code.md). The essentials:

- One path per behaviour. Express variants with generics, typestate or a builder. Do not add
  `_charged`/`_for_x` twins or report misuse at runtime that a type could prevent.
- Engine crates hold no game schema. A game declares its data; the engine stays generic.
- Keep internals `pub(crate)` behind a small facade, with one error enum per public boundary.
- When a size note or limit fires, split by responsibility, never by line count.

## Hot paths

For per-tick, per-frame and per-message code:

- No heap allocation in steady state. Reuse buffers, use frame arenas, move instead of cloning.
- Store iterated data as columns. Encode or validate once and pass the checked form on.
- Use a fast non-cryptographic hasher for internal maps.
- Account cost before choosing a representation, using
  [the cost account](../../../docs/design/AI-PERFORMANCE.md#the-cost-account-in-each-owning-design).
- Use the hardware the device has. Select accelerated paths (GPU family features, tile memory,
  SIMD, core classes, platform IO) from the capability registry, never by probing privately, and
  keep a portable fallback that the same tests exercise. See
  [the hardware-native decision](../../../docs/design/decisions/HARDWARE-NATIVE-ENGINE.md).

## Verify

- Tests cover behaviour and plausible failure. Authority work covers rejected, stale and
  duplicate input.
- Timing lives in `benches/` with stored baselines, never in ignored tests or examples.
  Allocation claims use the counting-allocator assertion.
- Every parser of untrusted bytes gets a fuzz target.
- Device, frame-time and visual claims come only from runs on named targets. Record the numbers
  in the maturity ledger, separately for implementation, correctness, visuals and device.
- A dedicated recipe or accelerated path records its win against portable on the same device and
  its image or result difference within tolerance. Without that, it is implemented, not qualified.
- Evidence per slice is at most one page: screenshots, one timing table and an oracle diff when
  relevant. Bulk output follows [the storage rules](../../../docs/ASSET-LIFECYCLE.md).
- Run `./dev check` before each commit.

## Before committing

- Did this add a second path for an existing behaviour? A registry-selected hardware path with a
  portable fallback and a measured win is intended; any other duplicate is not.
- Did the public API grow, and does each new public item need to be public?
- Is the rigour proportional to the state it protects?
- Is there an allocation, clone or repeated decode on a hot path?
- Did the slice move the milestone, and does the report separate measured results from open gates?
