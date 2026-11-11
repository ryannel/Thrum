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
