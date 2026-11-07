# R0 one-ratio Smith and represented-dot CPU oracle

3 November 2026. This selects a scratch-only Python 3.11 numerical experiment
for the [robust GGX research candidate](../../design/research/R0-ROBUST-GGX-EVALUATION-OPTIONS-2026-11-03.md).
The [archive](../ARCHIVE-ACCESS.md#archive-inventory) contains its editable
source, deterministic JSON results, full report, ownership record and hash
manifest. The execution revision was `81b1269`; `results.json` records exact
input-document hashes. Archive SHA-256:
`d17b94ebc24693f281484ab43ad4b0e492aaeef5a463d407abd9de8ff6ee92c5`.
Source SHA-256:
`dd1a5e12ca83042b620da253f0280c047724734c0f87141b3fb55c49c5b5caee`.
Results SHA-256:
`8e27f7e8f005feef2a1fb58902f97b683f9955a59c4668c9430b0330dc80a323`.
The parent and an independent adversarial reviewer reproduced the Python 3.11
result byte-for-byte; the selected run took about 0.47 s for the whole CPU
workflow. This is not shader timing.

| Bounded check | Result |
| --- | ---: |
| Original Lambda-based versus one-ratio expressions | 1,265 fixtures; 554 `ci≥co`, 707 `ci<co`, four hemisphere zeros |
| Maximum `abs(candidate−reference)/max(1,abs(reference))` | `7.7541e-334`, below the declared `1e-260` tolerance |
| Freshly reconstructed sharp-family precision cross-check | 340 versus 420 decimal digits; largest reference change `3.9311e-280` |
| Exact represented-dot checks | 4,000 seeded normal-or-zero f32 component cases plus five selected cases |
| Deliberate wrong implementations rejected | Missing branch numerator ratio; 256-bit fixed-point accumulator |

The original evaluator uses a separately normalized half vector, Lambda-based
correlated Smith, BRDF division and incident cosine. The candidate uses a
max-scaled half vector and one ratio in the selected branch. Agreement across
finite samples supports transcription; the algebraic derivation is separate.
An exact integer lattice and an independently coded `Fraction` route agree
on represented f32 dot signs. Normal operands can cancel to `±2^-298`, and
three products bounded by one need up to 301 signed bits on the shared
`2^-298` lattice. An exact rational norm interval certifies that two authored
unit directions with opposite real hemisphere signs narrow to identical f32
direction bits; the represented exact dot is zero. A selected product-FTZ
case loses a positive `2^-127` dot. These are distinct arithmetic and
information-loss findings, not a WGSL emulator.

No f32 one-ratio implementation, sound WGSL interval filter, multiword shader
fallback, source-error certificate, half packing, GPU/backend result, moving
visual qualification or performance claim follows. The next proof must
preserve positive magnitude as well as sign through the uncertain path, prove
allowed WGSL operation orders and measure fallback occupancy/cost in the
five-light workload before selecting an R0 shader.
