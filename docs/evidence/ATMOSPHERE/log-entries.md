# ATMOSPHERE: log entries in full

Entries of [the module log](../../plans/ATMOSPHERE/LOG.md) that ran past a few lines, moved here unchanged on 11 November 2026 when the plans took the orchestrate 1.7.0 shape. The log keeps each one's opening and a link.

<a id="entry-1"></a>

## Entry 1: 10 November

- 10 November revision after [the design review](design-review.md) at `4bbe44a4`
  (changes requested). Finding 1: the design now carries a numerical acceptance table. P1–P11 are the existing
  referees with their own references, cases and bounds, preserved; H1–H5 are the new handoff rows for the lent
  operations, on Metal and in Chrome; M5.2, M5.3 and M6.3 point at those rows, and M6.4 is the explicit Chrome
  run. Finding 2: environment reflection is the fifth lent operation, with its roughness and decoding stated,
  and H5 tests it from an independent contribution. Finding 3: the cooked volume's march calls the shared
  cloud-factor function per lit march sample and keeps today's lighting with no provider; M5.6 adds a spatially
  varying field. Finding 4: each lent lookup's support and the cloud field's limits are in the design, and a
  LUT generation is defined as a replacement, not a recompute. Finding 5: M6.5 compares against a fresh rerun
  of the commit M6 starts from. Also from the item notes: Chrome cost and texture counts are their own item
  (M5.10), and M6.1 says the crossing exclusion is only the moving spur edge.

<a id="entry-2"></a>

## Entry 2: 11 November

- 11 November revision after [the second design review](design-review.md) at
  `27473469` (changes requested, one Must Fix). Finding 1: the texel-range rule of H2 and H4 accepted a lookup
  that returns one depth slice. It is replaced by the filtered-decode rule: an f64 evaluation of the addressing
  and blend on the read-back texels, written apart from the library, with a bound of `C/32 + 1e-5 × M`, a live
  arm, a known-field arm with interior queries, and single-texel and single-slice lookups that must fail on
  both targets. M5.2 and M5.3 name it; M6.3 and M6.4 repeat the same rows and their wording is unchanged.
  Finding 2: the lent values are stated as physical and free of pre-exposure, the design says where a
  contribution applies the lent scale once, and each handoff row repeats one case at a non-unit scale.
  The `C/32` weight allowance rests on 8-bit sampler weights, which is common practice and not measured here
  on the M3 Max or in Chrome. If a correct lookup misses it, the builder reports the measured weight
  resolution and the bound comes back to the plan; it is not widened in the build.

<a id="entry-3"></a>

## Entry 3: 11 November, M5's H2 blocker

- 11 November plan change. M5's builder ended blocked at WIP commit `7911c75b` on `build/atmosphere`, base
  `a471fe5d`: H2 failed the filtered-decode bound `C/32 + 1e-5 × M` on the live sky chart, and the design sends
  such a miss back to planning. The builder's note and logs are `docs/evidence/ATMOSPHERE/m5-h2-blocker.md` and
  `docs/evidence/ATMOSPHERE/m5-h2/` in that commit; they reach main when M5 lands.

**What was measured**, on the Apple M3 Max, Metal, the production `RGBA16Float` sky chart and linear sampler:

| Case | GPU result | f64 filtered decode | Old allowance | Error | One stored step, decoded |
|---|---|---|---|---|---|
| Live chart, Sun 45°, view 1° above the anti-Sun horizon, green | 0.086669921875 | 0.0866538116 | 0.0000123 | 0.0000161 | 0.0000610 |
| The same query, blue | 0.09643555 | 0.0964067323 | 0.0000239 | 0.0000288 | 0.0000610 |
| Two texels one stored step apart (64 and 64.0625), midpoint | 0.06256103515625 | 0.0625305176 | 0.0000025 | 0.0000305 | 0.0000610 |

- Across 257 positions in one texel interval, stored endpoints 0 and 1 return 257 distinct values, all inside
  the old bound. So the sampler's weights resolve at least 1/256, and the `C/32` weight allowance holds.
- Endpoints 64 and 64.0625 are neighbouring half-float values. They return two distinct values, and 235 of the
  257 positions miss the old bound. A filter that returned float32 results would return values between them.
- The largest difference between the GPU's address and the f64 address is 0.000006814 texels. Evaluating the f64
  blend at the GPU's observed address moves the expected value in the eleventh decimal place, so addressing does
  not explain the miss. A signed-π wrap change was tried and removed after the result persisted.
- Every error above is at most half a stored step, and each GPU result is itself a half-float value at the
  chart's storage gain. The filter returns its result rounded to the nearest half-float.
- H4's live and known-field arms, its exposure control and its three wrong lookups pass on Metal under the old
  bound, as do H1's 20 low-altitude cases.

**Decision.** The lookup is correct, and the old bound was wrong to say that exact read-back texels leave no
storage step: the filter's result is stored-format precision on this target. The bound becomes
`C/32 + 1e-5 × M + Q`, where `Q` is one stored half-float step at the largest stored magnitude the blend reads,
per stored channel, through the same decode scale. The same rule applies to H2 and H4, on Metal and in Chrome.

- **A whole step, not half.** The measurement shows rounding to nearest, an error of at most half a step, and an
  exact half-step at the midpoint. Neither WebGPU nor Metal specifies the precision or rounding of a filtered
  result, and a bound of exactly half a step would sit on that tie. One step covers nearest and truncating
  filters. Direct3D's floating-point rules ask 0.5 ULP of 16-bit float operations, which is the same order.
- **The rule still catches a wrong lookup.** On the known field a single-texel, single-row, single-column or
  single-slice lookup is wrong, in at least one channel, by a quarter or more of that channel's smallest step
  between neighbouring texels, at every query. With every step at least 32 stored steps, `Q` is at most 1/32 of
  it; on a sky field whose two axis steps are `s` and `2s` the whole bound is about `s/8`. The design now states
  the 32-step floor, and the wrong lookups must still fail at every query between texel centres. The builder's
  first known sky field is a ramp, `1 + x + y` in stored units, which steps by only 8 to 16 stored steps at its
  queries. A ramp cannot do better far from the origin, since the stored step grows with the value, so the
  field becomes a repeating pattern.
- **The live arm still means something.** It shows the right chart, gain and mapping: an error in any of those
  is far larger than 0.1% of the value wherever neighbouring texels differ at all.
- **The lookup mechanism and its cost do not change.** One filtered read for the sky, and two per aerial chart.
  Blending exact texels in the shader (four loads for the sky, sixteen for the aerial volume) would give float32
  results, at more reads on every sky pixel and lit fragment and a changed shader and image for every scene,
  since the forward pass shares these functions. The stored texels are rounded to the same step, so the product
  would be no more accurate. Float32 charts need the optional `float32-filterable` WebGPU feature and double
  the charts' memory. Neither is taken. M6's review judges sky and twilight banding by eye; a visible step
  there reopens the choice.
- **The precision measurement stays** in the Metal run as a printed record, with no gate of its own.
- **No visible claim is made here.** Whether a 0.1% step shows in a sky gradient is M6's judgement, not this
  entry's.

Changed: the design's filtered-decode rule, lent-products paragraph and alternatives; M5.2's question; the
plan's M5 work list and one new assumption for the owner. No milestone, check, case or other bound changed.
M5's state is left as the loop recorded it; the lead unblocks it after the design's review.
