# APPLE-PRESENTATION plan

Reconstruction, display and the quality governor (gate G4), governed by
[the G3t boundary decision](../../design/decisions/G3T-METAL-BOUNDARY.md) and C04, C06, C07 and C12.
[Design](DESIGN.md) · [done-when file](DONE.json), which holds M1–M3 · [log](LOG.md) ·
[module list and seam rulings](../ARCHITECTURE.md#interfaces) · paired plan [APPLE-TILE](../APPLE-TILE/PLAN.md)

A design approval covers M1–M3 and seams 1–4 only. M4–M11 keep their drafted criteria in
[the milestone drafts](../../evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md) until a plan run migrates them into the
done-when file and settles seams 5–9. What each milestone waits on is in
[the dependency table](DESIGN.md#parallel-work-and-dependencies-on-g3t).

## Open for the owner

- Assumption: the Chrome witness covers the same rows as the Mac, the volume and phase-edit rows included. The review required only the invalid-history rows and the history identity in Chrome; the fixture is shared, and Chrome is where cloud and sunset demos will run.
- Assumption: ΔE is CIEDE2000 on the submitted 8-bit SDR output, since reconstruction is judged as displayed. The bound stays at 3 and binds until a plan revision changes it. This replaces the earlier assumption that the bound is "restated after M3's first run".
- Assumption: the per-frame history identity reaches Chrome through the game page's own report, so T2's capture record is not changed. A builder who needs a T2 record change proposes it here first.
- Assumption: seams 1–4 of the design are accepted as written, with the G4 builder making the motion-only edit in the atmosphere shaders (seam 4) and the third forward target across contributors (seam 3) itself.

## M1 Every pixel of the valley carries correct motion under jitter, including the sky and stars

Every M1 witness runs through the current portable output, offscreen, before G3t M3: the jitter-sequence and
record-layout unit tests (D1.1); motion-texture readback (it has `COPY_SRC`) for the static-scene zero-motion test
(D1.2) and the sky pan test (D1.3); and a motion mode in the existing `synthetic/display.rs` display pass for the route
review (D1.5). The first G4 witnesses that need G3t M3 are TAAU (M3) and the extent change (M4).

- [x] View-record fields and jitter sequence, with `pack_aux` carrying the subrectangle (→D1.1).
- [x] Unjittered clip motion in `synthetic.wgsl` (→D1.2).
- [x] Sky and star motion at infinity, delivered by the atmosphere owner under seam 4 (→D1.3).
- [x] Invalid-history flag (→D1.4).
- [x] Motion mode in `synthetic/display.rs` and the review sequence (→D1.5).
- [ ] MF1 Keep spatial presentation stable while retaining submitted jitter metadata, restore consecutive-frame regressions and repeat the reviewer's held-camera captures.

## M2 Depth, motion, reactive and exposure are stored for reconstruction, and history advances only with submitted frames

- [ ] Sampleable depth and the motion reader in the graph (→D2.1).
- [ ] Reactive target across synthetic, contribution, cooked, sky and star-pass pipelines, with the star pass's
      attachments and graph accesses, under seam 3 (→D2.2).
- [ ] Exposure resolve and p retention (→D2.3).
- [ ] `history.rs` and `runtime.rs` acceptance changes and the pipelined tests, under seam 1 (→D2.4).
- [ ] The witness fixture definition, read by both targets, and the Mac GPU tests for W1–W5 (→D2.6).
- [ ] The frame host's per-frame history identity in the game page's report; a reactive view or attachment
      readback that reaches Chrome's submitted frames; the `temporal` case with its unsubmitted view (→D2.5).
- [ ] The existing browser cases rerun (→D2.7).

## M3 TAAU reconstructs the valley at native scale in the game, on the Mac and in Chrome

This is the first in-game reconstruction on portable. It runs at the current 640×360 extent, after G3t M3, so
TAAU's linear output feeds G3t's pre-tone input; the existing SDR transform still runs once.

- [ ] `taau.rs` and `taau.wgsl`: dilation, YCoCg clip, luma weighting, reactive and depth rejection, linear-depth
      history, p rescale (→D3.1, D3.3).
- [ ] Registry-resolved `Reconstructor`, with TAAU as the default (→D3.1).
- [ ] `reference.rs`: the accumulation mode (the 32-phase cycle run twice, the two averages compared) and the
      CIEDE2000 comparison (→D3.2).
- [ ] Reset wiring (→D3.4).
- [ ] Timing row (→D3.5).

## M4 The valley fills the window at any size, and keeps presenting while the window is resized

Not in the done-when file yet. Starts after M3, `APPLE-TILE:M3` and seam 5. [Criteria and work as drafted](../../evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md#m4-surface-derived-extent-and-generation-transitions-pending).

## M5 Render scale steps during play with no hitch and no history reset

Not in the done-when file yet. Starts after M4. [Criteria and work as drafted](../../evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md#m5-dynamic-render-scale-pending).

## M6 Highlights go brighter than SDR white on the built-in XDR display

Not in the done-when file yet. Starts after `APPLE-TILE:M3` and seam 6. [Criteria and work as drafted](../../evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md#m6-edr-on-macos-pending).

## M7 The governor holds the display rate under forced load, then recovers step by step

Not in the done-when file yet. Starts after M5 and seam 7. [Criteria and work as drafted](../../evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md#m7-quality-governor-pending).

## M8 MetalFX reconstructs the valley on Apple tile, with TAAU as its prepared fallback

Not in the done-when file yet. Starts after `APPLE-TILE:M11`, seam 8 and M5. [Criteria and work as drafted](../../evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md#m8-metalfx-temporal-pending).

## M9 Surface, device and display changes create nothing during play, and launch-to-ready time is on record

Not in the done-when file yet. Its baseline (D9.1) can run after `APPLE-TILE:M0`; the rest starts after M4–M8. [Criteria and work as drafted](../../evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md#m9-launch-to-ready-split-and-prepared-paths-pending).

## M10 A warm launch compiles no pipelines: offline metallibs, pre-warm and the local archive are proved

Not in the done-when file yet. Starts after M9's baseline. [Criteria and work as drafted](../../evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md#m10-offline-metallibs-pre-warm-and-the-local-archive-pending).

## U1 The Metal pipeline-cache change is offered upstream by the owner

Later, and not part of V1. After M10 is proved; the owner writes and approves every public post. [As drafted](../../evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md#u1-upstream-metal-pipeline-cache-later-not-v1-gating-if-needed-pending).

## M11 Apple tile with MetalFX and EDR is compared with portable TAAU on the route, and G4 closes

Not in the done-when file yet. Starts after M1–M10 and `APPLE-TILE:M12`. [Criteria and work as drafted](../../evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md#m11-paired-proof-run-pending).
