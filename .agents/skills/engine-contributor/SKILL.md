---
name: engine-contributor
description: Implement, refactor or review Thrum engine code toward the active build-plan milestone, and verify it. Use engine-architecture or engine-module-design for whole-engine or new-module design, and repo-structure for file placement and retention.
---

# Engine contributor

Thrum is a showcase of AI-driven engineering: butter-smooth, high-end visuals and performance beyond what a human
team would build and maintain, first shown in teaching demos on the owner's website. Engine code succeeds when it
runs correctly and looks right on named targets. Judge each change by whether it moves the active gate in
[the build plan](../../../docs/BUILD-PLAN.md) forward, not by how much it proves.

## Pick the work

- Follow the build plan's current priority and permitted parallel work. Prefer a complete interaction or visible
  quality improvement; investigate measured regressions without turning target chasing into the work queue.
- Aim for showcase quality, not parity. Prefer named techniques, paper reproductions and specialisations agents can
  sustain (per-family kernels, specialization, prepared work). Foundation choices use best-practice review; claimed
  wins need measurements. Record actual sources and evidence, including unqualified results, in the
  [technique register](../../../docs/design/AI-PERFORMANCE.md#technique-register). Other engines are context, not gates.
- Native leads and uses every hardware feature; the browser runs the portable recipe at showcase quality and never
  limits native. Single-player fidelity and performance lead; G2S proves multiplayer headless and transport is parked
  until after V1.
- Extend running paths before hardening inactive ones. Code with no consumer by the end of a slice does not land.
  Parked tracks stay parked unless the owner reopens them.
- Research or design only to unblock a named decision. Keep the record to two pages under `docs/design/decisions/`.
- Before building a general subsystem, look for a proven crate that meets the targets (Wasm, mobile, determinism where
  needed) and adopt it with a reuse record. Build your own where a specialised path is the point and can show its win.
- Keep building. Stop and ask only to change purpose, the V1 exit or gate order, to spend money, to push, publish,
  license or deploy, or before an irreversible action, and continue unblocked work meanwhile. Never wait for a reply to
  a progress report. See [steering](../../../docs/design/AI-DEVELOPMENT.md#visual-review-and-human-steering).

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

- Design hot paths to reuse buffers/arenas and move values. Record dependency allocations; they are not admission gates.
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
- Device, frame-time and visual claims come only from runs on named targets: this M3 Max, the
  iPhone simulator (function only, never speed) and Chrome. Label Mac stand-ins for iPhone
  “proxy”. Record results in the maturity ledger, separately for implementation, correctness,
  visuals and device.
- Keep benchmarks as history and investigate visible hitches, about 25% regression from a comparable baseline, or
  unexpected scaling. Foundation choices use best-practice review; numeric tuning waits for the dedicated phase.
- A recipe keeps portable fallback and checked image/result tolerance. Claim performance wins only with measurements.
- Visible output passes agent visual review before the change lands; agents can and do inspect images:
  1. Capture fixed cameras and a short sequence from the exact build on the named target.
  2. Run automated image checks: a perceptual difference such as FLIP against the portable recipe
     or ground-truth oracle within the declared tolerance, and no NaN, black, clipped, banded or
     flickering output.
  3. Inspect the stills, a contact sheet and the sequence yourself against the gate's look target
     and its paper or reference images. Record findings.
  4. Give an independent reviewer agent the captures and look target, not your verdict, to hunt
     for artifacts and motion faults. Fix its findings or record them as open.
  5. Record `agent_reviewed` for experienced quality in the maturity ledger.
  6. When a gate closes or something first becomes visible, add a visual digest to the gate's
     evidence page and notify the owner: contact sheet, short clip, numbers and open look questions,
     each with your recommended answer. Then carry on with that recommendation. Owner feedback
     steers the current or next slice; owner `accepted` is recorded but never a gate condition.
- Evidence is at most one page per gate, including its visual digest: stills, one timing table and
  an oracle diff when relevant. Bulk output follows [the storage rules](../../../docs/ASSET-LIFECYCLE.md).
- Run `./dev check` before each commit.

## Before committing

- Did this add a second path for an existing behaviour? A registry-selected hardware path with a checked portable
  fallback is intended; ordinary behavior still has one body. Performance claims require separate evidence.
- Did the public API grow, and does each new public item need to be public?
- Is the rigour proportional to the state it protects?
- Is there an allocation, clone or repeated decode on a hot path?
- Did visible output pass agent visual review, with findings fixed or recorded?
- Did the slice move the gate, and does the report separate measured results from open gates?
