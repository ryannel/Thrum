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
