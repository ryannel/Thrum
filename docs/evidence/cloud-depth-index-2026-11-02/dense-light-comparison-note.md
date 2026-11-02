# Fixed uniform T-depth lattice: decisive bounded rejection

2 November2026. Root commissioned only64/128/256 active-plane checks, then stopped this investigation after the failure. **None passes the selected.005 depth-approximation allocation.** No GPU/source/candidate change or additional plane-count search was performed.

New CPU reference `scripts/cloud_sky_proof/dense_light_reference.py`, SHA256 `2a4543fac62525327dbe43a74140fa0cea842d576fcbbd5d0f772e76500b2dd6`. [Actual comparison receipt](dense-light-comparison.json), SHA256 `0e513bba600d4004bdbdb4409f2c758d3da0e3e7abefa92eb4a64ef1b72808c1`.

```sh
PYTHONDONTWRITEBYTECODE=1 /opt/homebrew/bin/python3.14 scripts/cloud_sky_proof/dense_light_reference.py tmp/cloud-sky-2026-11-01/captures/deep-probe-1793583015463.json tmp/cloud-sky-2026-11-01/captures/deep-probe-1793588276463.json --output tmp/cloud-sky-2026-11-01/review/dense-light-comparison.json
```

Inputs are the SHA-verified actual16×513 GPU path streams from the previous full/cropped captures, not regenerated source data. Source/receipt/shader hashes are retained in the JSON. D counts both active endpoint planes; duplicate endpoint guards add2 physical planes. Uniform mathematical active depths span0…L inclusive with spacing L/(D−1). Values are `exp(-tau_PL)` and `exp(-.25*tau_PL)`, separately rounded to half. Hardware trilinear T averaging in Z/XY would preserve the same XY filtered operator; it must not interpolate tau or infer one branch from the other.

The independent oracle merges every original PL-tau knot with every lattice plane. On each piece old tau=t+d*u, new transmittance=c+e*u. Extrema of their difference satisfy `exp(-a*(t+d*u))=-e/(a*d)`. Endpoints suffice for zero slopes; otherwise the one interior root is tested. Every original slope-change knot is included. Dense32-subdivision-per-input-interval checks of combined half-stored error agree with the analytical maxima within the reported float64 engineering guard2e-12. This is a finite CPU check, not a uniform GPU coordinate/exp/filter certificate.

| Active planes | Full128 depth error primary/quarter | Cropped128 depth error primary/quarter | Cropped rejected columns of16 |
| --- | --- | --- | --- |
|64|.12895344 /.03959111|.08697133 /.02401993|11|
|128|.05890329 /.01577175|.03103792 /.00896659|11|
|256|.02564269 /.00695766|.01000369 /.00284971|6|

Even cropped256 fails.005 on primary. Largest measured half-storage rounding error is.000244034, within.002; combined stored256 error is.01020114, above even the depth-plus-storage.007 sum. The cropped128 worst at column39,32/depth153.086868m is.03103792 primary. Worst errors here include darkening before sharp transitions; a smooth constant-density curvature estimate alone would miss the input slope-change witnesses. For a single constant-slope tau interval a smooth bound is `a²*m²*max(T)*h²/8`, but it does not cover derivative jumps across original knots. Use the actual piecewise oracle rather than assuming all cloud depth curves are smooth.

The proposed numerical allocation remains.005 quadrature+.005 depth approximation+.002 storage against the same filtered footprint. The test concerns only depth/storage against actual input PL tau; quadrature remains separate. Relabelling depth blur as a new model or changing coefficients would be another proposal, not a pass of this one.

| D | Physical Z | RGBA16F bytes for66² columns | Output exponentials | Source coefficient requests |
| --- | --- | ---: | ---: | ---: |
|64|66|2,299,968|557,568|2,230,272|
|128|130|4,530,240|1,115,136|2,230,272|
|256|258|8,990,784|2,230,272|2,230,272|

RGBA16F uses8bytes/texel even though only two channels hold terms. K48 nodes+metadata reserve1,812,096bytes. Current cropped capture has15,011 anchor builds; its coefficient exp/log-or-polynomial work is per anchor, not per dense plane. Dense128 therefore trades much more preparation/exponential work and~2.5× product memory for a potentially cheaper single hardware texture query. A texture query conceptually fetches eight8byte texels; cache blocks, filtering precision, compiler code, bandwidth and latency determine actual cost. No speed inference follows. Three failed counts are enough: do not implement this lattice under the selected depth profile merely because it may remove binary searches.
