# Packed-field implementation closure

1 November 2026. Verified the actual corrections against PACK-R1–PACK-R4 by reading the current packing harness,
shared Beer function, immutable proof host, scalar-field/readback helpers, independent reference and final receipt.
No GPU rerun, renderer/canonical edit or commit occurred. No Cu files or family-quality evidence are covered.

## Bound evidence identity

Receipt: `tmp/cloud-sky-2026-11-01/captures/packing-1793573030049.json`.

Build ID: `8f079cbfe6d19a718ca96585fcc4d2ad6f3afff9c8a240a784d395b518e18adf`.
Reference bytes: `e115614d546d1de12b4328127d17f62fa308d5d0247be1c84dbd13e1dd09b1c9`.

I independently recomputed the build ID from the receipt's sorted manifest and regenerated the compact reference
JSON from its Python producer. Both match. Current packing-proof.js, scalar-field.js, gpu.js, transport.js,
serve.py, reference_packing.py, test_reference_packing.py and fixtures/packing.json hashes match the receipt manifest.
The relevant changed files are packing-proof.js `3516bf8618e8356e5fd562ff6a0e4ddc5c5f0d890f3b4c4285dbf6f9ed686991`,
serve.py `117a98c028f8bcccaef6ff20d9fc7e1afd0d8e09151af3c9df1b45aa6d1a4907` and
transport.js `032bb334e2ab09dd43985f13b7eacae70f556a92db2f7eb2d73e913c8747e610`.

## Finding dispositions

| Finding | Verification of correction | Result |
| --- | --- | --- |
| PACK-R1: S/Lout were only reported; slabs not run | Per-query acceptance now includes S<=0.002 and Lout<=0.006 alongside coefficient/T and finite Lout. The shared homogeneousScalar function is used by sampled queries and an independent nine-case operator pass. Operator cases compare tau/T/S/Lout directly against CPU values, with every absolute error <1e-5 required. The actual receipt has nine passing cases, including zero sigma and zero/one/100 m lengths; maximum operator-channel error is 9.346008322808075e-8. | **Closed for the named finite operator domain.** |
| PACK-R2: actual texture/mip ties and subnormals | The regenerated receipt includes rounding_store_edges-4x4x5 at all three levels with zero byte mismatches. Its actual field/mip stores therefore match the reference's deliberate both-parity normal/subnormal midpoint, normal-boundary and odd/group-edge expectations. Separate 16 codec cases also pass. | **Closed for the finite actual-store fixtures.** Sampling retains its stated numeric envelope; this is not a claim of bit-exact arbitrary subnormal interpolation. |
| PACK-R3: rejected capture fetch skipped device/button cleanup | The per-run device is destroyed before the receipt request. Receipt failures are caught while retaining the serialized report; an inner finally always restores the run button after request resolution/rejection. This directly removes the identified early exit. | **Closed by code inspection.** I did not inject a network failure or rerun the UI. |
| PACK-R4: current file hashes could misidentify cached executed modules | serve.py captures immutable in-memory bytes and one manifest/build ID at startup; module/fixture GETs use that snapshot. HTML receives its build ID, and Run compares it with the current host identity before GPU creation. A server restart makes an old page reject; later working-file edits do not mutate this host's served module graph. Fixture bytes are checked against that same manifest and QUERY_SHADER is additionally hashed. | **Closed for this bounded frozen-host protocol.** It does not claim Thrum source-admission/runtime integration. |

The former logical_bytes label is now physical_guarded_texture_bytes, matching what its sum actually counts.
Uniforms/staging/readback/temporary resources remain separate from this texture count; no complete-residency claim
is made by that field.

## Actual finite result and remaining boundaries

The receipt records **13 fixtures, 39 logical levels, 1273 sampled queries, zero packed-byte mismatches, 16 passing
half codec cases and nine passing constant Beer cases**, with no recorded failures. Reported maxima are:

- coefficient absolute error: 1.52587890625e-5;
- T absolute error: 0.0001493765881004916;
- S absolute error: 2.0942983312149943e-5;
- Lout absolute error: 0.0003196812583519115.

Those values satisfy the implemented finite gates. I reviewed the receipt and source; the root performed the GPU
execution. This closes the four reported R0 packing findings without certifying universal filter precision, optical
mip/coverage equivalence, general field capacity, temporal correspondence, device recovery, family appearance,
complete-sky viability or performance. Unused/older Cu-related files in a host manifest do not become exercised
by this receipt. New Cu work after this build is unrelated and unreviewed here.

No new or carrying blocker was found in the corrected R0 packing scope. Retain this bounded pass as arithmetic/
layout/operator evidence and proceed to the independently gated family work; do not relabel it as a cloud-family pass.


## 2 November follow-up — odd-mip world geometry

**Implementation inspection:** ScalarField.setParameters now keeps the same world lo and uses
`pitch_l=(base_extent/base_dimensions)*2^l`, `extent_l=ceil_dimensions_l*pitch_l`.
samplePackedWorld maps `(world-lo)/pitch_l` to the already-reviewed boundary-space interpolator. This is consistent
with eight-parent averages and zero-padded odd last blocks: their coarse sample is at the midpoint of the full parent
block, including its missing zero parent, rather than recompressing every coarse grid into the original extent.
The padded coarse extent is filtering geometry, not permission to enlarge authoritative physical cloud support.

**PACK-R5 P2 — current world test is self-consistent rather than independent.** At inspection, queryField constructs
sample world coordinates from `level.lo` and `level.voxelPitch`, then uploads those same properties as the shader's
world-to-logical parameters. Thus a regression to compressed pitch in ScalarField can move both the test point and
its inverse together and still reproduce every unchanged logical oracle. Recording the pitch/extent is useful but
is not an acceptance predicate. The existing byte/mip tests do not catch this world-geometry error either.

Concrete correction: derive expected world lo/pitch from the fixed independent fixture constants
`lo=[125.5,-33.25,4]`, `base_pitch=[1.25,.75,2.5]` and level index. Assert actual dimensions, lo, pitch and padded
extent against those expected values; build uploaded world sample coordinates from the independent expected values,
while the shader uses the actual field parameters. Keep a known odd-parent midpoint/last-partial-cell witness.

For `[7,5,9]`, level1 dimensions are `[4,3,5]`, pitch `[2.5,1.5,5]` and padded extent `[10,4.5,25]`. Its last coarse
cell centre is world `[134.25,-29.5,26.5]`, the midpoint of the final full zero-padded parent block. In the ramp fixture,
only parent index `[6,4,8]` contributes there, so the unrounded coarse coefficient is `75/131072` (and is exactly
representable in half). In Z alone, compressing the original22.5m extent into5cells would incorrectly move that last
centre to24.25m instead of26.5m. The test must reject that alternative rather than moving its expected sample with it.

No GPU run or code edit was performed in this follow-up. The earlier receipt remains valid for its prior logical
coordinate scope. A new root-run receipt can extend that scope after the independent world mapping gate is added;
this addendum does not claim the new world-coordinate path has already passed.


### PACK-R5 closure — independently pinned world geometry

The next inspected packing-proof.js supplies expectedGeometry from fixed fixture lo/base pitch and the independent
reference level index. Query world positions use those expected values, while query uniforms use the actual field
metadata. Separate assertions require actual lo, doubled pitch and padded extent to match the expected geometry;
the z9 case explicitly requires level1 z pitch5, extent25 and last centre26.5. This removes the coupled-test-point/
inverse loophole identified above. **PACK-R5 is closed for this finite R0 world-coordinate scope.**

Verified actual receipt `tmp/cloud-sky-2026-11-01/captures/packing-1793574393369.json`, build
`ade1e6341a763a26dddf22810d1e54f39cf73bb3fe1367bf5079792923036bae`. Its manifest hashes recompute that build ID;
current scalar-field.js `d731427f441aad597685b44c599b0f05cd70b31548e70079ce0f5e3320554212` and
packing-proof.js `b030f8cd951bf9495c2e982a44849b88cb09db660e5b0cc8f856c2d8d64cca35` match the manifest.
The unchanged independent fixture hash remains e115614d546d1de12b4328127d17f62fa308d5d0247be1c84dbd13e1dd09b1c9.
The new receipt records 13fixtures/39levels/1273queries, zero byte mismatches, no failures, 16passing half-codec cases
and nine passing Beer cases. The ramp's recorded world geometry matches all independently derived levels, including
level1 pitch[2.5,1.5,5]/extent[10,4.5,25] and level2 pitch[5,3,10]/extent[10,6,30].

The reference already contains the level1 logical point[3.5,2.5,4.5]. I checked that its expected coefficient is
75/131072 at world[134.25,-29.5,26.5]. Recompressing the original extent at that same independent world point gives
7.152557373046875e-5 instead of5.7220458984375e-4, an error5.006790161132812e-4, exceeding the receipt's level1
coefficient gate4.204246063232422e-5. Thus the new test is capable of rejecting the stated wrong geometry; this is an
analytical/reference counterexample, not an executed GPU mutation test.

The previous addendum is retained as the historical finding against its earlier harness read and is superseded by
this closure. No GPU execution or canonical edit was performed by this reviewer; execution belongs to root. This
extends only the bounded packed-field R0 proof, without admitting optical mip approximation, cloud-family quality,
full-sky cost or new Cu code.
