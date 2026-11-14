# M3 builder evidence

Implementation checkpoint, 14 November 2026. No item is qualified yet.

The portable registry selects TAAU. Its compute pass reads jittered forward colour,
reversed-Z depth, dilated motion, reactive and GPU-resolved exposure; it writes linear
colour through APPLE-TILE's retained pre-tone seam. The existing SDR transform runs
once. Colour and linear-depth history pairs are shared by both game frame slots and
selected with the submitted transform-history cursor. Cancelled work cannot switch
their read side. Capture-stage replacement retains the same history pair.

The separate reference shader sums each 32-phase cycle in RGBA32F and exposes its
average through the same pre-tone seam. The explicit game-host oracle records held
world/camera/solar state and actual GPU exposure on every submitted frame, compares
the two cycles, and includes history-disabled and suppressed-cut-reset controls.
`scripts/reconstruction_difference.py` compares every SDR pixel in D65 CIELAB using
CIEDE2000; its published supplementary-pair checks pass.

Verified so far: focused native Clippy, both WGSL modules' Naga validation, native
compilation and pinned-toolchain Wasm compilation. These establish no target image
quality, motion quality or GPU cost.
The real Metal alternating-slot/reset execution test now passes, as do the
authoritative tick-60 replacement and sampled moving-light fixture tests. The
game-reference proof is queued under the shared `gpu` lock. Chrome execution, motion sequences, timing,
exact baselines, the full gate and integration remain unqualified.

Technique source: [Karis, High-Quality Temporal Supersampling, SIGGRAPH 2014](https://advances.realtimerendering.com/s2014/index.html).

First target attempt at `55ae06ee`: [complete metrics](attempt-1/run.json),
[noon TAAU](attempt-1/noon-taau.png), [independent reference](attempt-1/noon-reference.png).
All six p99 values are below 3, but each fails the required improvement over no AA.
The two reference cycles agree exactly, the history-disabled controls are rejected,
and the cut equals its independently reset render while the suppressed-reset control
differs. These are historical results, not passes for the changed kernel.
The follow-up corrects depth-history jitter addressing, preserves exact unchanged
colour during clipping/blending, and rescales from full-precision retained exposure.
Its target rerun remains pending.
