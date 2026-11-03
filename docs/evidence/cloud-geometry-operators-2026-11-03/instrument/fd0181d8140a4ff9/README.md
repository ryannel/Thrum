# Manual held cloud geometry falsification

Owner: /root/cloud_lifecycle_diagnosis, delegated 2026-11-03. This agent owns only this directory. Peak disk budget: 1 GiB. Binary inputs are hardlinks to frozen files: never edit them in place. The master lab, source cook, distance cook, canonical notes and evidence remain untouched.

Run `python3 serve.py --port 8148`. The server binds only 127.0.0.1. Its cached, exact payload snapshot includes assets and excludes captures. The parent baseline is frozen master build adb13e8a53d0bd95. The shared geometry coefficient provider and bindings 18/19 are the only optical changes; illumination, view integration, shadows and reflections retain their existing math.

This is a manual, held-field experiment. It has no animation, weather advancement, PDE ticks or continuous render loop. Direct evaluation at each optical query is a falsification prototype, not a production cost claim. All optical consumers and the requested coefficient observer call the same `geometryAmount` function.

## Inputs and reference

Source 03 is available at 128³ and 256³, with iso densities .001 and .005. Signed distances use authored local metres, multiplied by four for the fixed world pose. They are opposite-class voxel-centre EDTs of `rho >= iso`, rather than exact continuous density-isosurface SDFs. Surface and normal errors are at voxel scale; the exact metadata also records half-float distance error.

The reference is `q * smoothstep(-32m, 32m, d0)`. It differs from the raw master. Its integral loss is about 1.28% / 2.47% for the two 128³ iso choices and 4.68% / 6.82% at 256³. Sub-iso tail loss is smaller and recorded separately. Changing quality or iso changes this approximate reference; do not hide that as a rendering improvement.

## Operators

- Raw: original q.
- Reference: the tapered reference above.
- SDF support: `q * smoothstep(-w, w, d0 - e*M)`.
- Density subtraction: `max(sum(reference) - rhoThreshold*M, 0)`, preserving phase proportions.
- Alpha: reference multiplied by the calibrated alpha.

M is either one or `.3 + .7*N`, with one fixed 32³ noise texture sampled linearly with clamp-to-edge at material UVW. There is no time offset. The exposure endpoint includes the maximum distance, taper width and an 8 m margin, divided by the minimum M. This does not establish constant noisy-front speed: the implicit front gradient changes with exposure and spatial M.

Exposure snaps to the nearest calibrated LUT row. All comparators use that actual exposure. Density threshold and alpha cannot be tuned independently. Receipts record the row, target native integral, thresholds, source/distance/noise/LUT hashes, iso, quality, camera, Sun, kappa and world transform. Native integral matching does not establish exact arbitrary-ray or projected-opacity matching. Raw-opacity display deliberately shows the unoperated input; coefficient-opacity display uses the selected operator.

The requested probe verifies the frozen scalar upload and queries the real shared coefficients on a 32³ world lattice. It reports independent CPU trilinear errors, positivity, the raw-extinction upper bound and sampled endpoint counts. It makes no native geometry-topology or whole-field integral claim. No voxel readback runs during ordinary frames.

Source and geometry textures are charged and fence-retired on replacement. Inactive PDE/rest resources stay at 1³; existing noise, curl and compiled source pipelines remain charged. After selection, retain exact code/data and selected receipts/images before removing only owned redundant artifacts. These local ignored files are not a remote backup.
