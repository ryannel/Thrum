# Coupled proof review and disposition

1 November2026. **Bounded coupling demonstrated; production admission blocked by cost.**
The canonical [cloud design](../../design/modules/CLOUDS.md) retains the source and joint closure.
This is a standalone Q2/Q3 instrument. It does not implement Thrum, water, full weather, temporal histories,
HDR environment/probe publication, dynamic opaque-object motion or engine provider lifecycle.

## Actual results

The main volume, matte receiver/object and perfect mirror consume one source age/world transform/light state.
The [normal image](images/body.png) agrees closely with the [full-resolution direct reconstruction reference](images/full-resolution.png):
mean0.0236/255 display-channel difference; p99one level;0.0321% pixels differ by more than8levels.
The [raw comparison](images/reconstruction-comparison.json) is a spatial reconstruction check with the same
light caches, not a full physical reference. Earlier reflected-edge streaks were corrected through complete
path-compatible filter footprints and direct fallback. No horizon strip or opaque-object leak appears in the
inspected samples. The coarse image still inherits the soft128³ source and authored lighting approximation.

The independent float64 oracle has13 passing tests, including a wrong-layering counterexample, source-off,
HG normalization and mirror composition. [Actual default readbacks](oracle/probe-1793566096053/query-validation.md)
are compared with1m integration using actual GPU caches: composed radiance differs by at most0.620% on the
selected rays. Mirror near/far differs by at most0.0753%. These are selected finite paths, not domain maxima.
[Cache readbacks](oracle/probe-1793566096053/cache-validation.md) have zero default-density/mip mismatches with
Q1 RN-half values and zero array mismatches with the reviewed specialized checkpoint. The physical cache floor
remains: sampled |T error|0.02859 and45.1% relative direct incident error at one opaque interior point; ambient
sampled required-node error is below1%. Authored distant-cloud appearance and strict physical accuracy are separate.

[Rejection-case probes](oracle/rejection-case-validation.md) verify all32 paths valid, geometry masking direct
light inside cloud, main omission retaining cloud in the secondary view, exactly one camera-to-mirror foreground
operator, and exactly zero S/terminal/composed radiance under zero illumination with extinction still present.
The wrong double-foreground operator differs by475–1949cd/m² in its witness; the actual affine composition differs
by at most0.000227cd/m². The [native still matrix](images/still-contact.png) includes low Sun, sky-only, dark,
foreground fog and upper geometry. The bounded fog spheres are deliberate rejection fixtures, not weather visuals.

## Motion actually inspected

| Witness | Source seconds | Wall seconds | Evidence |
| --- | --- | --- | --- |
| Normal growth1x | 40.00–61.76 | 21.759 | [movie](motion/motion-1793565823441-1x.webm), [sampled frames](motion/review/motion-1793565823441-1x-contact.png) |
| Normal breakup1x | 200.00–219.14 | 19.142 | [movie](motion/motion-1793565848275-1x.webm), [sampled frames](motion/review/motion-1793565848275-1x-contact.png) |
| Complete16x | 0.08–240 | 14.976 | [movie](motion/motion-1793565868709-16x.webm), [sampled frames](motion/review/motion-1793565868709-16x-contact.png) |
| Complete64x | 0.13–240 | 3.717 | [movie](motion/motion-1793565903636-64x.webm), [sampled frames](motion/review/motion-1793565903636-64x-contact.png) |

Parent inspected all four contact sequences and selected native growth/breakup witnesses; the independent reviewer
inspected all four plus12native witnesses and final reference images. These are sampled motion inspections, not a
claim of continuous video viewing or a measured lag bound. Growth adds real volume/lobes and corresponding receiver
shadow; breakup separates fragments and their reflected/shadow structure. The receiver becomes shadowed as the
source crosses it and brightens again. No independent view clock or whole-cloud wobble/fade substitutes for lifecycle.
Forward and reflected optical paths naturally reveal different surfaces/brightness of the same source.

The clean capture path first renders the requested age, freezes fixture controls, and records each submitted frame's
latched age. Earlier pre-record callbacks are excluded by construction. Unclamped source-rate errors are below
4e-12for1x/16x/64x. Callback p95is33.3–33.4ms with recorder load, so stable60Hz is not qualified; the24fps movies are
not a display-rate measurement. The faint early scaffold, soft detail and rounded late cores remain known Q1 content
limitations. No new source family, buoyant fluid solve or approaching storm has been demonstrated.

## Independent challenge and next gate

The [final independent review](review/final-review.md) closes the concrete transport/reconstruction/evidence findings
and validates conservative support bounds. Both density and manifest hashes now gate those content-specific bounds;
ambient is unavailable outside its expanded cloud-query domain. Full provider removal and zero-consumer cost remain
unrun; the main-cloud checkbox is an independent-consumer rejection fixture, not complete modularity evidence.

[Performance](PERFORMANCE.md) rejects production admission:600fresh complete graph samples give12.845ms median,
18.809ms p95GPU elapsed bracket against a5ms cloud allocation and16ms complete-frame envelope. Logical resources fit
the88MiB proposed service reservation, but actual process/driver/complete512MiB peaks, native/M2/mobile and integrated
Thrum costs are unmeasured. The next bounded risk is cost qualification with these image/optical witnesses retained;
do not expand weather or assume unproved histories will close the budget.

## Reproduction

From this isolated Thrum checkout, `python3 scripts/cloud_coupled_proof/serve.py` serves
`http://localhost:8142/scripts/cloud_coupled_proof/`. The source packet/master recovery and license records remain in
[Q1](../cloud-source-2026-11-01/REVIEW.md); the old8137lab is not this proof. The live instrument is paused when handed off.

On this machine the optical tools use `/opt/homebrew/bin/python3.14` with NumPy/Pillow; server uses standard Python.
Run `-m unittest scripts/cloud_coupled_proof/test_oracle.py`. Reproduce the cache report with
`scripts/cloud_coupled_proof/validate_cache.py --probe docs/evidence/cloud-coupled-2026-11-01/probes/probe-1793566096053 --out tmp/cloud-coupled-proof/recheck --ambient-active-local 112 32 80 400 256 432`;
then `validate_queries.py` with the same `--probe`/`--out`. Readers transparently decompress retained `.f16.gz` files.
`review_motion.py --movie <webm> --receipt <json> --out <directory>` regenerates packet/cadence and sampled witnesses.
Final loaded code snapshots hash-match the receipt; historical intermediate timing variants are diagnostic records.
