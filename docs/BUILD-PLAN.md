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

## Next: I1, headless authority and a game-owned rule

- [ ] Select the exact smallest I1 subset from Foundation and Network; record its contract mapping and
  acceptance fixture before exposing APIs. Start with one world, a controlled actor, a collectible and a door.
- [ ] Implement distinct identities, fixed-tick ownership and typed game-owned actions. Validate input;
  commit state and its action result together. Keep rendering and networking I/O out of this path.
- [ ] Exercise the same rules through local input and an in-memory authority/replica fixture. Include
  unauthorized, duplicate, late and stale-epoch input; verify no duplicate committed effect and coherent
  spawn/first-state visibility. Define the finite admitted scope instead of implementing a general framework.
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
