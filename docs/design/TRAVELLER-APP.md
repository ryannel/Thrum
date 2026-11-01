# Traveller forest app — proposal

**Status: assessment only, 28 October 2026.** No Traveller app, renderer implementation or app scaffolding was
created. The proposed next step is a separate runnable Rust app sharing the existing forest, so character movement
can be judged among the actual trees, ground, shadows and light.

The assessed source is branch `engine/rebuild`, commit `b6f06c8ae`, in
`tmp/worktrees/engine-rebuild/`; that worktree was clean during assessment. All Rust paths below are relative to
that branch, not the current TypeScript checkout. Recheck its current HEAD and instructions before implementation.

## Asset inputs

**Asset update, 29 October:** the [current modular walking candidate](../provenance/CONTEXT.md#source-8a3fb2f41ed7) now supplies a self-contained GLB, 55-bone skeleton including fingers, thirteen clips, metre-based sockets and a [delivery contract](../provenance/CONTEXT.md#source-850059c4267c). Use this instead of the rejected whole-character prototype below. It has been tested in the browser review, not this Rust renderer. Adopt through the branch's assets catalogue, phase-match movement, and consume the Jump vertical track only once. Scene implementation belongs to the scene owner; the original engine-gap assessment below has not been revalidated by this asset pass.

The following describes the earlier assessment inputs:

- [Current modular character production](../provenance/CONTEXT.md#source-62c8a2ebac4e): separate
  high-detail body, clothes, scarf and backpack sources are available. A static clothing fit preserves independently
  removable pieces and the complete body beneath them. The source body's head is temporary while dedicated head/cap
  jobs failed and exhausted their approved retries. No shared modular skeleton, deformation acceptance or runtime profile exists yet.
- [Earlier combined animated GLB](../provenance/CONTEXT.md#source-8956ed9a963a):
  retained as a movement prototype after the user rejected its likeness/charm. It has a 24-bone rig and Walk, Run,
  Idle, RegularJump and RunToWalk clips at a target height of 1.6 m. Its inherited skinned-parent warning requires
  consistent mesh/skeleton transforms. RunToWalk retains authored displacement; blending, gait, coat contact and
  terrain foot planting remain unaccepted. It is not the current modular character.
- [Separate lantern package](../provenance/CONTEXT.md#source-555b4182bb68): a generated high-detail GLB and editable
  Blender source now exist. The glass is opaque; emitter/material cleanup, final scale, hand grip and a runtime
  profile remain pending. Keep the prop independently attachable and switchable.

Factory files remain sources. A future runtime delivery must follow the asset storage contract and the Rust
branch's `assets/` catalogue; the app must not load directly from `factory/` or `tmp/`.

## Existing support and gaps

| Area | Already present | Required for this app |
| --- | --- | --- |
| Forest | `apps/forest/src/main.rs` composes flora, floor, stream, birds, audio and the engine's sky/air/shadows/display. `bw_engine::scene::ForestClearingPlugin` shares one immutable `SceneForest`. | Extract app-owned composition for reuse; preserve the existing forest app. |
| Movement | `crates/sim/bw_actors/src/walk.rs`: walk/run, normalized diagonals, terrain height, trunk/boulder collision, stream wading and footfalls. `apps/forest/src/walk.rs`: 60 Hz sim and replay. | Camera-relative input and facing toward travel. Existing input is first-person and active only in the `walk` view. |
| Camera | `crates/host/bw_engine/src/rig.rs`: tilted `BirdsEye` rig. `sim_driver.rs`: interpolated player/entity `FramePoses`. | Closer follow focus around the torso, stable azimuth, zoom/orbit. Request near flora residency (`detail_from_m: 0`). |
| Animation | `bw_asset` reads mesh attributes, images and static node placements. Wildlife uses rigid named parts and procedural gait. | glTF skin, inverse-bind and clip reader; clip sampling/blending; skinned drawing with shared illumination, shadows and current/previous poses for motion vectors. |
| Jump | None: existing contact places feet on terrain each tick. | Deterministic jump command, vertical velocity, grounded/landing state, airborne footfall suppression, then a matching clip. |
| Lantern | No local-light implementation in the assessed frame/surface path. | Deliver the prop and hand socket first; illuminating it requires a separate local-light slice. |

Bevy glTF/PBR/animation features are not enabled. Adding a Bevy PBR character would bypass the custom renderer's
forest light and display path. Extend the existing asset and surface owners instead. No new external dependency
is inherently required by this proposal.

## Proposed paths and implementation slices

1. **Share the clearing.** Add `crates/host/bw_forest/`, following the existing `bw_sea` sharing pattern. Move the
   reusable responsibilities of `apps/forest/src/{flora,floor,stream,walk,audio}.rs` there; keep
   `bw_engine::scene::ForestClearingPlugin` in its current owner. Make `apps/forest` a thin consumer. Apps must not
   depend on one another. Register the host DAG edge in `xtask/architecture.toml` through its required separate
   policy-change commit.
2. **Show the still traveller.** Add `apps/traveller/{Cargo.toml,app.toml,README.md,smoke.toml,src/main.rs,tests/smoke.rs}`
   consuming `bw_forest`; deliver the character through `assets/`. Add the minimum rig reader beside
   `crates/core/bw_asset/src/{glb,glb_scene,model}.rs`, frame records in `crates/core/bw_frame/src/`, a host adapter
   in `crates/host/bw_engine/src/`, and skinned drawing in `crates/render/bw_render_surfaces/`.
3. **Play and compare animation.** Add clip playback and transitions for Idle/Walk/Run, keeping animation phase
   consistent with travelled distance. Include an in-place review mode with clip selection and playback speed;
   it must be distinct from locomotion. Check shadows and temporal motion before controller work.
4. **Walk through the forest.** Add third-person input/follow rig, reusing resolved player poses and the existing
   walker/contact owner. Suggested controls: WASD move, Shift run, wheel zoom, drag orbit, R reset. Keep a small
   help strip and daylight/dusk comparison controls.
5. **Jump, then optional lantern.** Space jumps only after simulation and clip support exist. Add L to toggle the
   lantern when the independent prop, socket and local-light path are implemented. Do not postpone walk/run
   testing until lighting expansion is complete.

Settings for pace, turn response, follow distance and clip timing belong with their owning data/settings group.
Retain the forest app's existing view defaults. Reuse foundation appearance without retuning it to flatter the
character; integrated light/air acceptance remains pending in the foundation queue.

## Future commands and checks

These commands are for a future implemented app; `traveller` does not exist yet. Run from the Rust worktree with
its pinned toolchain and a build directory used only by that worktree:

```sh
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"
cargo xtask run traveller
cargo xtask build-web traveller --fast
cargo xtask serve
```

The generated launcher would expose `/traveller/` on the default `http://localhost:8000/`. Do not replace an
existing preview server to demonstrate this proposal. Validate parser/rig data and actor replay with scoped tests,
check native/wasm determinism after actor changes, preserve the existing forest smoke result, then inspect
traveller motion, ground contact and shadows in the real scene. Use `cargo xtask check` before commits and full
merge checks under the Rust skill. Captures prove implementation behavior, not user acceptance of the art or feel.

## Runtime provenance limit

The observed `:5320` listener served the older TypeScript `sunset-steps` worktree. Rust preview listeners `:8731`
and `:8732` served generated scratch forest payloads under `/private/tmp/`, with no proven link to the assessed
HEAD. Source and existing engine evidence were inspected; a running renderer was not visually requalified, and
no server was restarted or changed. Verify the exact served revision when implementation begins.
