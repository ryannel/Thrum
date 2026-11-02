# Pinned prepared-Q1 all-time reconstructed-field support

2 November2026. **All162 source knots /154 unique lossless R8 chunks were checked.** The resulting support can reduce the private Sun-product projection without changing density, the field sampling grid, geometry or calibration. This is support metadata, not a lateral-quality or family pass.

## Exact inputs, decoder and command

- Selected manifest `docs/evidence/cloud-source-2026-11-01/selected-clip/cloudclip.json`: SHA256 `2fca53ff92a9787a0bb54bd9db0c470f5cfe2a5d755534c134eec38568acb739`.
- Selected mixed-codec payload `density.bin`: SHA256 `999cd89480371d02dca2f7553b938e5b93e23f0b31ca9b306ae60beb62e325bd`.
- Reused lossless mixed decoder `scripts/cloud_source_proof/ranked.py`: SHA256 `c988983d157195f2d1c582fbd37469697b6c51c4f8c85498a645a7df4822c1a7`.
- New [support helper](../../../scripts/cloud_sky_proof/prepared_support.py): SHA256 `100523813ac7fb222e019f6c4e4bdf8e15d1e7f0806a7a38a3a4ea6f15138e86`.
- New [tests](../../../scripts/cloud_sky_proof/test_prepared_support.py): SHA256 `a0cd295057d0dd934e313405e1acb1d7105fb013593e9386b3a64b2e1b2a454e`.
- [Computed support receipt](prepared-support.json): SHA256 `2bd62f15d4fa41fffd44519531e15d172a3e04d1f13bf239a2a48ef4a957bac4`.

Actual commands, from worktree root:

```sh
PYTHONDONTWRITEBYTECODE=1 /opt/homebrew/bin/python3.14 scripts/cloud_sky_proof/prepared_support.py --output tmp/cloud-sky-2026-11-01/review/prepared-support.json
PYTHONDONTWRITEBYTECODE=1 /opt/homebrew/bin/python3.14 scripts/cloud_sky_proof/test_prepared_support.py
```

Four focused CPU tests pass. The decoder is the existing pinned, validated mixed dense/ranked decoder; I do not claim to have independently reimplemented that codec here. Each unique encoded chunk's SHA and each complete decoded R8 array's manifest SHA were verified. New independent work is the all-time union, exact field-centre positivity construction and reconstructed-hat bounds. No existing candidate/cooker/source file was edited, and no GPU job ran.

## Complete all-time union

The source is128³ R8 over local `[0,512]³m`, X fastest, array order Z/Y/X. It uses direct nonnegative temporal interpolation with no flow/history/deformation. Sigma is code×(.125/255), the retained Q1 **calibration exception**; this does not admit the production.1 coefficient limit. A positive scalar calibration does not change nonzero support.

There are162 frame instances at source time knots,154 unique encoded R8 chunks,159 nonempty frame instances and153 nonempty unique chunks. Empty knot times are0,7.5 and240s. Every unique chunk is enumerated with positive count, bounds and all referring source times in the receipt. The union contains **45,438 positive R8 voxels**. Exact positive index extrema in XYZ are

`min=[33,15,28]`, `max=[94,56,101]` inclusive.

Thus the original positive voxel **cell hull** is `[132,60,112]` to `[380,228,408]m`. This cell hull alone would crop reconstructed coefficients. Every unique source frame lies inside the proposed union index hull: zero exceptions. The checked identity is the selected cooked reader over its complete0–240s interval, not the unquantized continuous authoring function or an age90-only sample.

## Reconstruction derivation, verified on actual positivity masks

For source voxel index i, the centre is `c_i=4i+2m`. Canonical trilinear reconstruction gives a product of nonnegative one-dimensional hats. A positive source voxel can contribute along one axis only for `|q-c_i|<4m`; its support closure has endpoints `c_i±4m`.

At field resolution128, samples occur at the same `4j+2m` centres. Canonical interpolation is exactly the original R8 value at those centres. Therefore positive field sample indices are exactly the R8 positive indices, and each stored sample's reconstructed hat extends±4m. The resulting support closure is the original cell hull expanded by**2m** on every face:

`min=[130,58,110]m`, `max=[382,230,410]m`.

At field resolution256, sample centres are `q_j=2j+1m`. A positive source voxel contributes iff

`4i-2 < 2j+1 < 4i+6`, equivalently `j∈{2i-1,2i,2i+1,2i+2}`.

The helper explicitly constructs the full3D boolean field-centre support using that four-index-per-axis product for every positive source voxel. It finds **443,432 positive256³ field centres**, XYZ index extrema `[65,29,55]` to `[190,114,204]`. Their first/last physical centres are `[131,59,111]` / `[381,229,409]m`. The final2m-pitch field hats extend±2m, so support closure is

`min=[129,57,109]m`, `max=[383,231,411]m`.

This is the original source cell hull expanded by**3m**, verified rather than assumed. A256 field extends1m beyond the canonical R8 hat envelope in this case: sampling then reconstructing a second field is a distinct operation. Clipping to the original canonical envelope would crop that final field.

The focused test uses an independent scalar eight-neighbour trilinear oracle at every sample of tiny same-grid/twice-grid examples, including boundary clipping. It agrees with the constructed support mask. A single interior positive voxel independently demonstrates the cell-hull±2/±3m extensions.

| Reconstructed field | Min local metres | Max local metres | Extent metres | Centre metres |
| --- | --- | --- | --- | --- |
|128³|130,58,110|382,230,410|252,172,300|256,144,260|
|256³|129,57,109|383,231,411|254,174,302|256,144,260|

At every intermediate time, sigma is a convex combination of two nonnegative endpoint fields. Its support is contained in the union of the two endpoint supports, hence in the checked all-knot union. Temporal interpolation cannot create support outside that union. Binary16 rounding of nonnegative coefficients can remove positives through underflow, but zero remains zero and no new positive outside the union appears. At a full R8 knot, even the minimum positive twice-grid coefficient(.125/255)×(.25³) is about7.66e-6, comfortably above half underflow; the computed positive extrema are retained for this calibration. Exterior zero support remains exact. Exact source-off endpoints are safe subsets of the same bound.

## Light projection consequence and limits

These are already bounds on the **final reconstructed field**, including its hats. A consumer projecting these support bounds must **not add the field half-voxel halo again**. For support centre C and half extent h, light-axis raw half support is `abs(axis)·h`. Use the same selected guard/cascade padding afterwards. The coefficient field itself retains its original `[0,512]³` domain/grid and source definition. Crop only the private light traversal/projection domain.

For the actual30-degree captured U/V/W frame, the64² projection with the current `1/(1-4/64)` lateral guard extension gives:

| Field | Old U/V pixel pitch, metres | Support-bound U/V pitch, metres | Pitch improvement factors |
| --- | --- | --- | --- |
|128³|11.48865 /13.19214|6.33074 /5.46233|1.815× /2.415×|
|256³|11.44412 /13.14101|6.37527 /5.51346|1.795× /2.383×|

The improvement exceeds2× only on one axis for this light direction; do not claim2× on both axes. At128, the raw U/V/W half supports are189.922124/163.869924/197.831040m. At256 they are191.258014/165.403895/199.487954m. This reduces both lateral spacing and traversed depth at unchanged curve count; actual build cost, integration-step quality and worst lateral error must be measured after integration.

One common conservative support for both field resolutions is the256 bound. Per-generation128 may use its tighter bound. The helper currently admits only these two selected resolutions, the pinned source/codec and direct temporal reader; a new recipe/hash, flow warp, resampling grid/domain, shear or instance transform needs a corresponding bound. Common rigid translation/rotation transforms this local support with the source; world AABBs may conservatively enclose its transformed eight corners. It must not remain fixed at the old world origin under source wind. No family-wide arbitrary512m assumption follows.

This bounds all positive support but does not bound its lateral transmittance variation. It supplies a better finite domain for the next existing-count comparison, not a proof that the.008 lateral allocation or.02 total query target will pass. Preserve the earlier.3086/.1207 lateral failure and re-evaluate the actual new product before admitting it.
