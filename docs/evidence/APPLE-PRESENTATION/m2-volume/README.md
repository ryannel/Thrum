# M2 D2.6: production cooked-volume W3 repair

Finding `mac-volume-witness-bypasses-forward-pass` is addressed by
[`metal_cooked_volume_w3_uses_production_forward_reactive_attachment`](../../../../examples/v1-game/tests/temporal_volume_gpu.rs).
The target is this Mac's Metal backend, asserted by the test, with both portable and Apple-tile recipes.
[Source, binary and fixture identity](identity.json): `56469803aa6d16cc00c4bebfd7bca7ebcad70010`.

The shared [W3 fixture](../../../../crates/thrum-render/src/synthetic/temporal/witness.json) is cooked
into a real store and loaded by `CookedAssets`. A paused game host submits normal frames to a 65×65
offscreen presentation receiver. Its camera looks along the unit cube's central unit-length ray;
the odd viewport puts that ray at the centre of a pixel. The contribution uses `Pipelines::draw`,
`cooked_volume_vertex`, the production front-face culling, reversed-Z depth test, premultiplied colour
blend and third R8Unorm attachment. The test creates no substitute shader or volume pipeline.

The existing graph witness uses `textureLoad` on that actual reactive attachment. The temporal display
diagnostic copies its red lane unchanged, without tone mapping, gamma encoding or dither. Its fp16
intermediate preserves every R8Unorm code on conversion back to RGBA8. T2 copies the ordinary submitted
frame, not an endpoint side render. The test waits for a drawn cooked publication and two steady
submissions before judging W3.

| Recipe | Central reactive byte | Independent alpha | Uncovered corner bytes |
| --- | --- | --- | --- |
| portable | 161 | `1 − exp(−1) = 0.6321205588285577`, rounded to R8 code 161 | 0, 0, 0, 0 |
| apple-tile | 161 | same | 0, 0, 0, 0 |

[Restored passing run](restored.log.gz), [portable values](portable.json), [Apple-tile values](apple-tile.json).
Both frames are byte-identical to the [pre-mutation passing run](positive.log.gz).
The exact RGBA8 data is retained for [portable](portable.rgba8.gz) and [Apple-tile](apple-tile.rgba8.gz),
with lossless native-size PNGs for [portable](portable.png) and [Apple-tile](apple-tile.png).

The negative control temporarily disabled only the production volume pipeline's third attachment
write mask. The same test then failed its W3 assertion with actual 0 and expected 161.
[Exact mutation](negative.patch), [mutation identity](negative-identity.json), [failing run](negative.log.gz).
The production file was restored byte for byte before the final passing run. No production change
from this mutation is committed. The permanent test runs in `./dev check` on macOS.

W1, W2, W4 and W5 retain the accepted real-forward/star tests and [attachment evidence](../m2/metal-inputs.log.gz).
The [old isolated fragment proof](../m2/metal-volume.log.gz) remains supplementary. This repair proves
production-pass alpha composition; it makes no display-cadence or performance claim.

The integration gate retains its [exact-image record](exact-baselines.json) and
[verifier output](exact-baselines.log.gz). The module log records the full milestone check and merge.
