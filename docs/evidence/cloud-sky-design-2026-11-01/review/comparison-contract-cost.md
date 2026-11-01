# Revised cloud design: independent contract/cost comparison

1 November 2026. Reviewed the actual CLOUDS and CLOUD-CAPABILITIES drafts after the prior independent workload report. Read-only canonical review; no code, GPU execution, benchmarks or commits. File/line references below identify the reviewed draft and may move with subsequent edits. **Disposition: revise before declaring complete-design review passed.** No issue here removes the commissioned families, skies or transitions.

The new model repairs the principal scope failure: five families and four skies are required; source producers are family-specific; event identity/weather continuity, shared all-layer Sun paths and explicit cost failures are retained. These are design improvements, not implemented or qualified results. The scalar packing and streaming deep-curve ideas are credible candidates. This review does not endorse their quality/cost from that credibility.

## P1 — External ambient does not yet define an acyclic, once-only transport closure

**Locations:** CLOUDS:300–313,324–337,375–380,403–408; C08.

The draft defines sky by integrating air/cloud source, defines cloud source using `L_external`, then defines external cloud ambient as sampling pristine sky through the combined cloud/air/fog field. That leaves the actual producer dependency unresolved:

- If pristine sky already contains clear-air in-scattering over the queried angular path, multiplying it by another whole-path air transmittance double-attenuates that segment and does not add its correctly shadowed in-scattering.
- If the ambient ray runs the full joint cloud source, it needs `L_external` to produce `L_external`. Not feeding a final clouded probe back into itself does not by itself break this recursion.
- If pristine sky includes the new Sun disk, its angular contribution duplicates the separately evaluated direct `E_noncloud` term unless the disk is explicitly excluded or direct light is correspondingly partitioned.

**Correction:** select and write a finite dependency graph. A viable initial approximation is: source coefficients → all-cloud directional depth → common direct Sun attenuation → angular ambient integration containing terminal radiance and air/fog **direct** in-scattering, with all cloud/air/fog extinction but **excluding cloud in-scattering and excluding the solar disk already accounted as direct Sun** → cloud effective source → final joint main/reflection/environment products. This intentionally omits cloud-to-cloud diffuse exchange; label that approximation and test overcast/storm bases. Another finite iterative closure is possible but needs explicit iteration count, boundary/initial source, convergence/error and cost.

State precisely which air path is already represented in any clear-sky/LUT input. The final clouded surface irradiance/probe may include complete cloud radiance; it must not become an undeclared input to the external cloud ambient generation that produced it. The same closure applies to reflected rays and cloud-shaded air, independent of main visibility.

## P1 — 768 km is a visible-population bound, not demonstrated all-consumer support

**Locations:** CLOUDS:273–298,342–345,375–380,464–467; C01/C08/C12.

The sphere is a useful correction to the old flat 4 km profile. For R=6371 km, camera h=1.536 km and cloud top h=18 km, the sum of camera/cloud horizon distances is about620 km, so768 km has plausible view margin. That does not bound auxiliary rays launched **from** those visible samples.

A concrete spherical ambient counterexample, all distances in km:

- Camera C=(0,R+1.536) in a vertical plane.
- Visible cloud point P=(400,sqrt((R+6)^2-400^2)), radial altitude6. The camera-to-P segment stays above the planet; its minimum altitude is approximately0.232.
- Launch an outward local-tangent ambient ray with direction `(P_y/(R+6),-400/(R+6))`. It never intersects the planet.
- This ray reaches the18 km cloud shell at horizontal x≈790.627. At x=780 its radial altitude≈17.357 km. A high cloud there can attenuate required incident ambient at the visible point, despite lying outside768 km.

This does not require Sun below the5-degree admitted workload range or unrestricted flight. Camera offsets and environment anchors increase the required union. Reflected/probe origins must also be included. The existing expand/reject clause avoids fabricated clear output but does not close the selected workload/capacity; rejecting a required view because its ambient path exceeds a convenient source disk cannot close the commissioned sky.

**Correction:** specify support as the union of primary/reflected/probe paths and their admitted direct/ambient query closures, with exact sphere-intersection admission tests and maximum camera/anchor offsets. Either enlarge the fixed source region according to that derivation, or explicitly evaluate additional deterministic virtual world cells under a bounded query domain. A consumer/sky boundary has to be beyond its significant **lighting** dependencies, not merely beyond the main-camera horizon. Keep the5-degree admitted and2-degree stress distinction explicit; derive their Sun guards separately. This is a source/light-domain correction, not a need for a new external API or planetary gameplay system.

## P1 — The selected light/fallback workload has no concrete total bound yet

**Locations:** CLOUDS:347–368,451–503; C12/performance policy.

The24 MiB deep-node arithmetic is correct:512²×8×8=16 MiB;256²×12×8=6 MiB;128²×16×8=2 MiB. The three maps contain344064 columns. However their spatial extents, source quadrature/feature floors, total traversed intervals and maximum direct/refined fallback work are not selected. The phrase finite fallback with visible out-of-capacity failure is necessary correctness policy; it is not yet an analytical work bound for the complete sky.

The streaming removal rule performs candidate error evaluation after capacity is exceeded. With M integrated endpoints and K retained knots, straightforward implementation costs O(MK×number-of-closure-terms), not O(K) total/column. Each candidate needs the error over the changed piecewise segments and their exponential extrema. Dense overcast cannot assume short/empty columns. Invalid lateral footprints can also force fallback even when knot compression fits perfectly.

**Correction:** before building the chosen full light product, pin near/mid/far extents and a counted table for FAIR/BROKEN/OVERCAST/STORM: traversed source intervals/endpoints, node-fit candidate evaluations, active/resolved source evaluations, invalid columns, direct fallback rays/steps and refinement cap. Bound or explicitly reject the maximum source feature/optical complexity with the same visible witnesses retained. Keep worst-case work-cap failure distinct from merely missing the desired milliseconds. It is reasonable for runtime qualification to remain open, but total work must be concrete enough to falsify the representation early.

For perspective, just36.5 MiB for512 independent32³ current Cu phases +24 MiB nodes +~16 MiB raw targets is~76.5 MiB before immutable detail/recipes, metadata, CPU copies, high layers/Cb fields, air LUTs, ambient/probes, scratch and replacement overlap. This is not a whole-sky lower bound—different evaluated U/reuse can reduce it—but88 MiB cannot be inferred from the small-cloud reservation. A second light direction doubles the directional node/metadata closure. Fixed source/detail profiles and all allocations need the C05 peak ledger.

## P2 — Deep-fit error must be a composable query guarantee, not only a curve-fit number

**Locations:** CLOUDS:347–368; C08/C12.

The improved rule is mathematically plausible: compare the old/new piecewise-linear cumulative τ through each exponential closure term; check endpoints and all interior extrema; maintain an accumulated triangle bound for removals. That can conservatively bound the **current curve approximation** without retaining every old source sample, provided each local replacement error is evaluated over its entire changed support and the global accumulated bound is preserved through appends/replacements. Original knot τ values remaining monotone is appropriate. The rule must not recompute error solely from currently retained points and discard previous removal error.

Missing parameters/semantics:

- Only primary T has a numeric0.02 target. Higher-order terms can remain significant when primary T is black; give their bounds or a radiance-weighted combined bound.
- State whether the reported0.02 covers compression alone or the complete primary query. Lateral filtering, source quadrature, LUT/field approximation and storage consume separate error, then contribute to the final incident-light/radiance error.
- Define every invalid-footprint case: one invalid column in a filter footprint, disocclusion, outside cascade support and temporal source changes. Invalid data cannot be averaged as clear.
- Averaging per-column `exp(-b^n τ)` is appropriate for an explicitly filtered footprint, but it generally produces **no single τ** that reproduces all the averaged orders. Do not return `-log(mean T_primary)` and use that to reconstruct every higher order. Keep the order terms internal to V3 or evaluate the terms independently from each point τ before weighting. A typed point/segment τ query can remain unchanged.
- If finite collinear optical depth is ever obtained as a difference of large cumulative depths, its precision/error needs a separate bound. The current point-to-Sun query can avoid that subtraction; finite lightning queries already select direct integration.

**Correction:** add the exact retained error invariant, per-term/total error budget, invalid-query fallback policy and small independent piecewise-linear oracle cases. Include append/remove sequences with many separated thin layers, deliberately undersized K, opaque columns and translated lateral edges. This is correctness instrumentation, not a demand to certify the final maps before implementation.

## P2 — Sun-aware storage and exit have the right intent but lack an executable bound

**Locations:** CLOUDS:310–313,394–399; C08 and RENDER-SPEC scene units.

The generic transmittance cutoff was correctly rejected. However “actual terminal/Sun-disk and medium-source maxima” does not specify how those maxima are derived or admitted. Likewise choosing pre-exposure to fit the brightest **terminal** does not bound accumulated in-scattering/emission from every admitted medium/local-light input.

**Correction:** publish admitted maxima for solar irradiance/disk angular radius, sky/ground terminal radiance, phase peaks, optical coefficients/local optical length, higher-order coefficients and external ambient; include any finite lightning light/emission bound. Derive the exit test per colour channel. With nonnegative source and `j/σ_t≤B_source`, remaining Beer transport is bounded by `T_acc ×max(B_terminal,B_source)` for a nonemissive finite interval; zero-extinction emission needs a separate integrated bound. Sum/mixture and higher-order bounds must match the actual selected source closure, including the ambient dependency fixed above. Use the ray's actual possible disk intersection/planet occlusion, not an unrelated sky maximum when a tighter valid bound is available.

Choose storage pre-exposure from a bound on **composed radiance**, then test both overflow and underflow/precision. For illustration, E_sun=100000 lux and solar solid angle≈6.8e-5 sr implies disk radiance about1.47e9 cd/m². Fitting that into half requires p≤~4.45e-5, so dark signal/subnormal precision needs checking as well as saturation. These are illustrative quantities, not selected source assets. State finite-disk antialias/filtering and the approximation if illumination/shadows use a central directional ray while the terminal disk has finite size. V4 remains the sole final display owner.

## P2 — Packed scalar layout needs explicit boundary and optical filtering rules

**Locations:** CLOUDS:238–250,485–488; C05/C08.

The4Z RGBA16F method can reconstruct trilinear scalar samples: XY hardware filtering within a centred group, select the correct component for each adjacent logical Z cell, then interpolate in logical Z. Two filtered fetches per integer mip is plausible; no hardware Z interpolation across packed groups. No inherent sampling contradiction was found.

Before making it the selected current-field path, specify:

- logical-centre convention, negative/last Z cells and zero-support guards; grouping/channel selection across every4-cell boundary;
- odd/nonmultiple-of-four Z sizes, guard-group/padding cost and tiny anisotropic mip dimensions;
- atlas XY filter halos and mip offsets; logical mip generation from eight **logical** parents, not native texture mipmapping over packed groups;
- half-rounding/oracle tolerance and all admitted source values, not only current Q1 values;
- which coefficient mip is allowed for point light queries versus a filtered camera footprint. Correct coefficient averaging is not a guarantee of correct average transmittance across unresolved clear/cloud mixtures; the later footprint rule must govern these mips too.

The stated2.28125 bytes/logical voxel matches base plus two regular1/8-volume mip levels, before guards/padding. Name that level count explicitly; “three mips” can mean three additional levels. Near the smallest dimensions,4Z rounding invalidates the ideal byte ratio and should appear in the actual ledger. Retain RGBA16F one-channel golden fallback until scalar/operator, boundary, source-family image and native/browser format tests pass. This does not require editing the public coefficient interface.

## Capability-ledger corrections and comparison result

CLOUD-CAPABILITIES correctly retains incomplete/fail scope and makes storm/five families required. One wording correction: FS01 calls the high layer “optional required high-layer witness”; select whether it is part of FAIR or a separate required cross-layer scenario. Do not leave its acceptance ambiguous.

The reviewed design now has concrete source mechanisms and meaningful workload populations. It has **not** yet closed all dependency/support/work bounds. Resolve the P1 items in canonical design, disposition P2 details into the selected representation or explicit correctness gate, then re-review changed sections. Preserve earlier Q1/coupled images and adverse cost receipts; do not promote any new family or all-sky result from this review. Current engine construction and shared-contract owners remain authoritative; no new package, speculative API, code or performance certification is proposed.
