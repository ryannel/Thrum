# Independent prepared-Q1 crop / producer / observer review

2026-11-02. Read frozen `prepared-cu.js` `40a62b37fe55f31086416322f9665703441c4ca97619bfff6e74cc4c043aef78` and `prepared-field-probes.js` `81fbc6927f91e07f4b20cf79900393487d32f38e18e6fb1bc94605890718b93d` completely; inspected coordinator changes in ScalarField, preview JS/WGSL and neighboring fit/deep observer/consumer mappings. Also read the all-time support note/helper/tests and pinned source decoder. No GPU execution, candidate edits, canonical edits or commits by this reviewer.

Coordinator identities at inspection: `scalar-field.js` `013b09879461b36002800897a4a50d549e229fb32cffe3c7df5ef21715a60bf1`; `cu-preview.js` `5d8354d7a57691a81c37c02a2e7c227280fab2a4fe0d8a5cd412a1060aa82f5d`; `cu-preview.wgsl` `197185242415bcc30a98b3b4a4b543b6c7335ba21f2fd93c1c453138bdf51882`.

## Concrete findings sent to owners

**CROP-R1 — P1 observer failure masking: asynchronous GPU validation errors need to reject the comparison.** `capturePreparedFieldComparison` currently does not push/pop a validation error scope or consume an equivalent fatal error latch for its private builds/copies/dispatches. The host's existing `uncapturederror` handler pauses and writes status, but does not reject this helper. A validation failure in the shared field-build command can leave all three zero-initialized fields/mips equal; the later independent canonical decoder pass can execute successfully, satisfy per-role bit parity, and allow `receipt.pass=true`. Shader compilation errors are checked, but that does not cover command/bind-group/copy validation errors. Fail/throw the inspector on scoped device errors, after draining the observer submissions and before returning/saving a passing receipt; preserve cleanup and avoid attributing observer work to frame timing. This is not a request for a new numerical quality threshold. The pure codeword comparator is correctly an equivalence predicate; it alone cannot establish that two equal outputs were successfully produced.

**CROP-R2 — P2 neighboring body observer uses the master domain with cropped dimensions.** `fit-probes.js` still sets `bounds=recipe.bounds` then uses current product nx/ny/nz for canonical CPU column positions and Y-halo integration. For128 crop, column0 CPU X is4m (`512/64/2`) while the actual cropped field column is130m (`128+4/2`). CPU Y pitch becomes512/48 instead of4m, and the integral traverses the wrong interval. World column coordinates reported from state use the crop, so the receipt compares different columns while labelling this current-field error. Use `sourceObject.fieldBounds??recipe.bounds` for the canonical source-local column/halo mapping and keep master bounds for camera/content only. The analogous256 case also needs this correction. This affects the quality observer, not the actual ScatterFit column shader, whose extent/texture-Y pitch remains correct.

Both owners were notified during review. No correction is assumed closed from a message alone; closure must inspect the revised files below or in a follow-up.

## Mapping, semantics and mip result

The all-time support metadata is tied to the exact manifest/R8 payload/codec and complete162 source knots, not age90 alone. Nonnegative direct temporal interpolation cannot introduce support outside their union. The positivity construction and half-voxel hat extensions are coherent; the support note correctly distinguishes source hats from a second sampled/reconstructed256 field. I did not independently rewrite the mixed codec or rerun a new bulk source export.

Actual allocations are:

| Nominal side | Original logical min | Cropped dimensions | Local field lo / extent metres |
| --- | --- | --- | --- |
|128|32,12,28|64,48,76|128,48,112 /256,192,304|
|256|64,28,52|128,88,156|128,56,104 /256,176,312|

Minima and max-exclusive bounds align4 original cells. At levels0/1/2 the logical offset is min/(1/2/4), always integer, and world pitch is original pitch times1/2/4. Matching full/cropped sample centres coincide exactly in these power-of-two metre grids. This preserves the spatial averaging phase of both logical mip operations. It does not require cropped RGBA groups to share the full texture's channel phase: the comparator maps logical Z through full group/channel coordinates correctly, including offsets that cross group boundaries at later mips.

`FIELD_DISPATCH` now forms world positions as `lo+(logical+.5)*(extent/grid)`, keeping the exact4m/2m pitch before multiplication. For these aligned bounds, the128 producer query is always at original source centres4i+2m. The shortcut's floor(q/4) is then exactly source index i. Canonical trilinear sigma at that centre has one nonzero weight, so the shortcut calls the same decoded R8 voxel values and same temporal mix without changing coefficient units. The canonical arbitrary-position function remains the unchanged trilinear query. Source decoder side remains128 even for the256 sampled field. No arbitrary-point direct-voxel substitution is admitted.

All three logical mips are generated from the ordinary scalar children, not by hardware mipping packed groups. Removing only identically zero original cells outside the support hull, with aligned averaging blocks and explicit zero guards, preserves these scalar mip values. The pure comparison checks every cropped logical codeword, every crop/full guard and padding channel, full exterior zero, and infinity/NaN in both allocations. It can reject an omitted exterior contributor, rather than checking only the overlapping region. Finite-age GPU comparisons still need to run; all-time omission safety comes from the pinned CPU support union, not from finitely sampled GPU ages.

## Coordinator and consumers

Recipe master bounds remain512m and the independent canonical source coordinates remain source-local. Preview camera width/centre/target and reference comparison material point derive from the master, so crop dimensions do not recenter or rescale the camera/patch. Actual Scene lo/extent and grid derive from fieldBounds, with common rigid source translation applied once. Legacy projection widths now use the actual field extent, consistent with their Scene centre; deep support projection independently uses that same field domain and halo. No coordinate inconsistency was found there. The renderer's source interval retains the half-cell reconstruction collar, including a positive first cropped Z sample whose hat extends before field lo.

Preview Scene adds referencePoint at byte192 and grows to208B; image layout/buffer match208. Deep and ScatterFit retain the unchanged first192B ABI. The deep inspector's `lastUniformWords.slice(0,48)` keeps its private192B copy exact rather than treating the appended point as a new grid member. Allocation-failure UI restore uses prepared nominalSide, preventing a64/128 cropped X count from being mistaken for the requested128/256 selector value. Source/light binding swaps still publish after successful preparation and quiescence.

The crop changes ray entry/exit and can change midpoint phase; it also changes the extent/spacing of approximate light/ambient products. Exact field/mip codewords therefore do **not** imply native image bit equivalence. Camera equality and correct units are necessary, but actual native/CU-ray/light comparisons remain separate. The original support note's historical statement that only private light projection is cropped should be updated when documenting adoption of this field-allocation crop; its support derivation remains useful.

## Checks actually run

- Existing `test_prepared_support.py`:4 focused CPU tests pass under Python3.14.
- Independent Node allocation/float32-centre check across every per-axis sample at both nominal resolutions and all3 mip levels: exact matching world centres and integral offsets.
- Pure comparison fixture: matching nonzero crop passes; a nonzero full logical sample outside the crop rejects with outside count1; nonzero crop guard rejects with guard count1.

No GPU/source capture, image comparison or timing by this reviewer. Remaining requested runtime evidence: actual full/generic/special/crop packed outputs across all3 levels, guards/exterior and finite ages; canonical query parity; validation-error rejection; native views/CU-ray integration and cache differences. No production coefficient admission, new variant/mature motion/family acceptance or performance pass follows from this review.
