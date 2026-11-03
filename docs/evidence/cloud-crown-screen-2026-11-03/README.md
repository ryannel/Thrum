# One structured-crown preflight — rejected before distance cook

The commissioned ONE mask is implemented and previewed. **Author and root reject this recipe before EDT/full asset cook.** Root inspected all six final perspective contacts; neither agreement nor these images qualify full source art or lifecycle acceptance.

Narrowing the old R33→40 capsule creates visible asymmetric upper notches and two/three lobes. The old isolated oval becomes a smaller mitten/peanut form. In actual220 composite side view the upright straight trunk remains; front/oblique heads still read as broad smooth knobs. It improves gross silhouette variation but does not deliver convincing cauliflower anatomy. That limitation is already visible without lighting. The field is not optically empty, but substantially less amount survives. No alternative masks, density compensation, threshold relaxation or additional recipes were searched. No EDT, runtime payload, GPU/browser, canonical/evidence change or current-source edit was made.

## Exact comparison

Every contact is raw crop diagnostic / old masked rho / new masked rho, left to right. Cyan contour = projected peak rho≥.005; orange = projected peak rho≥.041. Gray = 1−exp(−.019 integral q ds), displayed directly without lighting, scattering, tone mapping or gamma conversion. All comparisons retain identical ray samples/cameras per contact. Contours are peak-field projection contours, not a surface-mesh measurement.

- [Native front](observations/native-front.png), [native side](observations/native-side.png): native2m axis integration, isotropic world scale6.
- [Isolated front](observations/isolated-front.png), [side](observations/isolated-side.png), [oblique](observations/isolated-oblique.png): actual220 ID5 pose; fixed1200m cameras targeting root/head midpoint, recorded yaw/pitch,48° vertical FOV;256×144,8world-m midpoint samples, displayed2×nearest.
- [Composite220 front](observations/composite220-front.png), [side](observations/composite220-side.png), [oblique](observations/composite220-oblique.png): exact recorded c3e0 source220 parts and capture4/5/6 cameras. Existing BODY uses its actual density, fresh distance, Aseed, age exposure and shared material-coordinate noise. Captured weather-grid integration count is0 at220, so weather exposure is exactly0. Every point takes max across the same-formation parts before Beer integration.

**All three B comparators are masked rho before fresh distance taper**. They are upper bounds on eventual post-cook B q; old preview also omits its existing fresh taper for a matched operator comparison. The actual B parts contributing at220 have ageE0; newborn ID7 has Aseed0 and contributes nothing. These are not beauty images or an exact replay of the currently displayed renderer. Raw crop crosses domain faces and image borders and is explicitly unqualified diagnostic content. Old/new have zero projected≥.005 border pixels in all final perspective views; neither is clipped. All three use return0 outside the same192³ local domain.

The simplified proven path has9 sections/10 vertices. Two-way continuous distance is bounded by sampled distances plus.0625m (1-Lipschitz guard): maximum 0.956927m≤2. Radius grows12→24 by simplified-path arc length. Sections use hard max; only whole stem→upper R20→depth R16 uses compact polynomial smoothmax3, at most.75m enlargement per join. The new rho is raw master R16 times smoothstep(−8,8,mask), Float32 product then R16 rounding. The old recipe reconstructs the frozen old rho **byte-for-byte**.

## Native and projection outcomes

Old masked sum = 4410.992326200; new = 1622.424038827; new retains 36.781% of old amount. Native rho>.041 cells: 39,701→15,443. All six new native border faces are0. Native rho>.041 maximum Y is333→335localm; nonzero maximum Y remains337m, so upper extent is retained. This does not prove the current finite admission catalogue still works.

Maximum Beer opacity, fixed κ=.019:

| View | Old isolated | New isolated | Old composite | New composite |
|---|---:|---:|---:|---:|
| front | 0.7304 | 0.5732 | 0.8683 | 0.8501 |
| side | 0.6838 | 0.5042 | 0.9314 | 0.8691 |
| oblique | 0.7144 | 0.5645 | 0.8997 | 0.8658 |

These values show retained optical amount, not an art acceptance threshold. Perspective sums are numerical8m midpoint quadrature, not an exact analytic integral or converged GPU reference.

Six-metre raw trilinear-cell lower minima: root .0765381, primary .0624695, upper .0534973, depth .0588684. Along the original23 path vertices the minimum is .0582886; overlapping6m spheres give a connected raw positive chain. This does **not** prove a continuous6m-radius tube, an at-most16-ball final chain, or post-taper attachment. The fresh-distance sufficient support screen checks an expanded radius6+8+2sqrt(3)+.13m, with rho cell minima≥.001. Only primary passes (minimum .0400085); root/upper/depth include0. Failure of that strong sufficient screen is inconclusive about their eventual fresh-distance balls. Actual new distance, post-taper root/head/chain radii, burial/neck, all-old exposure cue and1200s catalogue remain unrun. Rejection stops before those costs.

## Correctness, cost and reproduction

`/opt/homebrew/bin/python3.14 test_preflight.py`:3 tests passed (stored Q inverse/forward, exact varying-radius sphere envelope against dense scalar reference, compact smoothmax/trilinear domain contract). Old mask's byte identity additionally checks native axes, crop position and mask arithmetic. Final script/input/output hashes are in [original scratch handoff](observations/HANDOFF.json); full metrics/pins/cameras are in [results.json](observations/results.json).

Final native+projection run: 34.682s elapsed, sampled peak 85.766MiB, process high-water 86.484MiB. Planned≤128MiB, early112/hard160MiB; no guard breach. Native candidate arrays5,308,416B; working-array bound<32MiB is a design account, not a measured allocation ledger. Existing32MiB master and component/noise inputs are read-only mmap. No native candidate payload was written. Output is under1MiB; this is CPU preflight cost, not runtime/source/GPU performance.

Exact [frozen script](frozen/preflight.py), [tests](frozen/test_preflight.py), [commission](frozen/COMMISSION.md), [pre-allocation plan](frozen/PREFLIGHT.md), [dependency pins](dependencies.json) and [package manifest](manifest.json) are retained. No R16 payload is duplicated here. The original32MiB master is recovered losslessly from the [authored-master package](../cloud-authored-masters-2026-11-03/README.md), whose selected VDB/CC0 licence/provenance remains authoritative; BODY/B inputs come from the [frozen v2 component package](../cloud-component-proof-2026-11-03/README.md);64KiB noise comes from the [integrated pilot archive](../cloud-system-pilot-2026-11-03/README.md). None depends on a live tmp checkout. File/capture names retain historical labels, without asserting a new GPU build.

Verify the package and compressed dependency identities without restoring or rendering:

```sh
python3 reproduce.py --verify-only
```

Restore only required exact inputs and the frozen preflight into a fresh disposable directory; `--test` executes the three small math tests but does not render:

```sh
/opt/homebrew/bin/python3.14 reproduce.py --output /tmp/crown-screen-replay --test
```

After root releases any CPU timing window, use `--run` on a fresh output to reproduce the projections. It invokes the unchanged frozen script. NumPy/Pillow are required; exact historical interpreter was Python3.14/NumPy2.4.6/Pillow12.2.0. `ps` RSS guards and recorded high-water units are macOS-specific. Cross-host bitwise PNG regeneration is not certified; exact retained outputs and source bytes are hash-pinned. The wrapper checks every decoded dependency, preserves Xfastest axes and refuses existing/evidence output directories. Temporary decoded inputs total48,143,445bytes; no new native asset is retained in this package. [Verification receipt](verification.json) records checks actually executed.

The next [two-band geometric-displacement design](NEXT-OPERATOR.md) is a bounded proposal only. Its hashes, interpolation, support guard, amplitude uncertainty and proof gates are specified. No noise field, candidate rho, EDT, new asset or runtime implementation of that operator has been produced.

An initial capture-reader `time`/`sourceTime` schema error stopped after native work at69MiB (`adverse-reader.json`). A separate forward-map transpose error was caught by clipped previews, interrupted, corrected and regression-tested; those incorrect PNGs were overwritten and are not retained evidence. A corrected900m framing run completed37.868s/90MiB, but touched the old isolated front edge. Its metrics are retained only as `framing-900-results.json`; final1200m/root-head-midpoint framing uniformly replaces it, with all composite metrics bit-identical. No mask or sampling step changed between these framing runs.
