# Cirrus repeated-stripe correction

2 November2026. Bounded Ci source/transport correction, build `62167671ff19b5a3`. The old all-patch sine-power source produced the regular comb visible in the user report. It is replaced with eight finite seeded3D filament bundles and localized ice heads. This fixes the repeated bars; it does not promote complete CF05/P1b art or production performance.

## Reproduce

Copy the exact [base instrument](../cloud-billowing-2026-11-02/instrument/) (`ebbb321b0ed2b253`) and apply the [delta](instrument.patch):

```sh
cp -R docs/evidence/cloud-billowing-2026-11-02/instrument tmp/cloud-cirrus-replay
git apply --directory=tmp/cloud-cirrus-replay docs/evidence/cloud-cirrus-2026-11-02/instrument.patch
node --test tmp/cloud-cirrus-replay/tests/*.test.mjs
python3 tmp/cloud-cirrus-replay/serve.py --port 8147
```

The reconstructed optical files were verified byte-for-byte against [identity](identity.json), then all five Node test files passed. Active development server remains8146. Select Cirrus, pause, enable Full resolution, Seeded start, Inspect billows; advance180s for the bounded evolved check. Frame cell is a farther comparison. Reset restores the far camera, so choose Inspect billows again for a close check.

## Mechanism and evidence

[WMO cirrus species](https://cloudatlas.wmo.int/en/species-cirrus-ci.html) describe fine irregular filaments, including tails/hooks. [The proof card](PROOF.md) derives the chosen finite optical recipe and rejection conditions; exact widths, supply/fall and wind scaling are authored approximations, not a meteorology solver.

- Eight deterministic controls vary length, width, curvature, altitude and spacing. Seeded tails taper in3D and lower along their length. Ongoing source is at the heads, not every old stripe. The existing source buffer is reused; other cloud recipes are unchanged.
- Initial peak ice proxy is .28; the earlier nearly unit-density repeated ribbons were too opaque. The material, phase/light/receiver/reflection path is unchanged and all readers still sample native q.
- Ci gets faster upper-level horizontal transport, weak cross/vertical eddies and1.5m/s fall in the same velocity used for q/carrier. A first finite-bundle trial still used lower-cloud-strength curl and evolved into puffs; actual accelerated frames rejected it before selection. The final anisotropic profile preserves elongated tails.
- [x] CPU deterministic controls, finite parameters, varied dimensions, bounded default seed support and reset/replay checks pass with existing weather/source/thermal tests.
- [x] GPU checks pass, including the actual Ci packed-control layout: nonzero ice-only finite seed, empty support beyond the finite tail, and no ice birth with zero lift. Existing transport, phase, no-phantom-birth, air and material checks still pass.
- [x] Native before/after, close/normal-camera and0/180s pictures, source probes, original1x/16x canvas clips with fixed-fixture receipts are in [observations](observations/). Primary inspected native decoded frames at0/5/10s; no claim of continuous human video playback or independent art approval. Close1x180.25→192.25s; accelerated4.25→196.25s. An additional far1x0.25→12.25s is retained with its distinct camera.
- [x] At0s liquid0/ice257.911; at180s liquid0/ice338.590, all six occupied faces0. At242.25s supply-off command (moist/lift0,dry.7), existing q is retained; at302.25s ice190.875 versus352.590 before, measured last-step new source0, liquid0 and faces0. CPU forcing relaxes over12s; this is not instant deletion or a global opacity fade.
- [ ] Full Ci family quality/fine fibrous detail, strong/changing winds, long-run region exchange, target hardware and full-sky budget admission remain open. These bounded observations do not close the entire cloud system.

The reproduced original comb is build `ebbb321b0ed2b253` and uses the retained base source above. Final screenshots/receipts identify each camera/Sun/time; do not compare the far and close views as an identical-framing A/B.

## Costs and limitations

No new GPU textures, buffers or passes. One1024-byte CPU control table is prepared at a Cirrus reset; existing24 emitter upload slots are reused. The source reaction evaluates up to eight bounded bundle masks, replacing the global sine mask and skipping the unused generic32-pulse loop for Ci. It does not add per-pixel head work to lighting/view rays or CPU voxel work. Available-host logical inventory30.8MiB after lazy numerical/probe resources, unchanged in order from the previous96×80×80 family path. No new quiet benchmark or full performance acceptance is claimed. Whole-GPU timestamp totals remain unqualified.

Native pitch135.4×58.75×150m limits fine fibres, and the present material/lighting calibration remains a pilot. Default bounded source faces are clear; this is not indefinite support or strong-wind admission. No paid asset or tool is used.
