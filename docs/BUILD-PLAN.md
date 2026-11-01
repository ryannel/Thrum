# Building the spine

The first useful spine is **typed action → validated rules → committed world state → observable result**.
Local play and multiplayer must use the same rule path. Content admission supplies a pinned source;
presentation, save and replication consume owned committed state. No renderer, water solver or editor
sets the authority model.

The [construction plan](design/CONSTRUCTION-PLAN.md) owns the full I1–I5 dependency sequence. This file
is the active short queue for the next bounded work. The [C4 contracts](design/CONTRACTS.md),
[foundation specification](design/FOUNDATION-SPEC.md) and [network specification](design/NETWORK-SPEC.md)
govern it. Thirty design responsibilities do not mean thirty crates need to exist now.

## Completed setup

The repository has a pinned Rust workspace, a portable core scaffold, real contributor commands,
dependency/link/hash checks, focused skills and copied design/review history. Runtime maturity remains
`not_started_for_target`. Native/Wasm scaffold compile results do not qualify engine platforms.

## Performance ambition and immediate design work

We are building for shocking performance with stable behavior. [The investment map](design/AI-PERFORMANCE.md#capability-investment-map)
connects the major capabilities to existing owners and delivery milestones. Deep performance work begins with each
capability, including frequent cheap operations and creator iteration; it is not reserved for I5 or a final cleanup.

The next I1 commission first supplies a compact cost account for world access, action validation, state/result
publication, allocation/copying and authority/replica handoff. Derive scaling and compare viable layouts/scheduling
against the existing contracts before exposing APIs. Include representative scale/churn beyond the tiny game, without
inventing new product budgets or importing later MMO scope. Select the needed correctness oracles and observation
points. Then build the chosen path and qualify it; do not add a generic framework just to make the harness possible.

## Next: I1, headless authority and a game-owned rule

- [ ] Select the exact smallest I1 subset from Foundation and Network; record its contract mapping and
  acceptance fixture before exposing APIs. Start with one world, a controlled actor, a collectible and a door.
- [ ] Record the I1 cost account and selected representation, with expected work/bytes, meaningful latency boundaries,
  adverse cases and a scoped correctness/measurement fixture. Track unmeasured assumptions explicitly.
- [ ] Implement distinct identities, fixed-tick ownership and typed game-owned actions. Validate input;
  commit state and its action result together. Keep rendering and networking I/O out of this path.
- [ ] Exercise the same rules through local input and an in-memory authority/replica fixture. Include
  unauthorized, duplicate, late and stale-epoch input; verify no duplicate committed effect and coherent
  spawn/first-state visibility. Define the finite admitted scope instead of implementing a general framework.
- [ ] Qualify the selected I1 path across the declared scale/churn cases; assess recurring small costs, operation/byte
  counts, latency tails and measurement noise. Pursue justified improvements within the commission and integrate
  them with both local and replica consumers; record remaining opportunity and the reason for stopping.
- [ ] Inspect the full interaction and update the owning maturity entries with actual evidence. A passing
  crate build is insufficient; a game-owned rule and its affected consumers must work together.

Use one small core crate until real ownership/dependency pressure warrants splitting it. `hecs` is a selected
private storage candidate in the design; verify its selected API/version/license and need before adding it.
Do not publish storage implementation types as the engine API.

## Follow the interaction outward

I2a adds the minimal immutable admitted source/content/private-save/restart path. I3a adds one camera,
basic presentation and a development export. **I2n proceeds alongside I3a:** the same game over real WSS,
with bounded full groups, action outcomes, join/reconnect and fault handling. It is committed work before
broad I3b integration; the headless fixture does not replace it. Optional prediction, advanced water and
online account services are not prerequisites for that first network game.

Thin native mobile boot/input/suspend/surface-recovery shells start alongside the early game path, as
specified by I1; they do not wait for a desktop renderer to mature. Desktop/browser/mobile runtime tests
need their actual hosts and devices. Add broader authoring, streaming, animation and environment only
through the complete I2b/I3b contracts; retain explicit stopped loading until live transitions are designed
and qualified. The original visual-quality and flexible-provider ambitions remain in the brief.


## Cloud design handoff

The [cloud capability inventory](design/modules/CLOUD-CAPABILITIES.md) and [complete system design](design/modules/CLOUDS.md)
are D2 design work, aligned with E1/V3/V4/F4 and the existing baseline. The proposed RC volume extension follows the
foundation/content/render dependencies above; it does not reorder the active construction queue. The [Q1 source proof](evidence/cloud-source-2026-11-01/REVIEW.md) supplies bounded local-lifecycle, sampled optical and encoded-byte evidence. Consumer conformance and complete target performance remain unrun. Retain useful legacy renderer algorithms
through those interfaces; no Bevy/legacy host migration is implied. Storm progression remains optional after the base.
