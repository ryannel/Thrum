# Independent R0 GGX numerical oracle — 3 November 2026

**Selected result:** a standard-library Python Decimal oracle separately
evaluated the original GGX/height-correlated Smith/Schlick BRDF and the
[scaled candidate](../../design/research/R0-ARITHMETIC-ENVELOPE-2026-11-03.md)
at 240 decimal digits. Across 5,098 deterministic fixtures, 5,086
positive-hemisphere original-versus-scaled evaluations met the declared
`1e-210` unit-scaled tolerance; maximum discrepancy was `8.026e-234`.
5,087 positive-hemisphere cases met the independently derived sharp
`E/(2πα³)` bound. These samples check algebra and transcription; the
real-number derivation, not sampling, establishes the bound under its stated
domain. No actual shader, authored R0 source, device or visual claim follows.

The [source, selected results, run summary and full report](../ARCHIVE-ACCESS.md#archive-inventory)
archive has SHA-256
`8c519edfa222ae264a537d584ede5726e2bcc24ca1ff0a0dc9384be413290c7c`.
Archived `oracle.py` is
`2759658594e418f6caa2ba8865785fc31ac4888037bbad348b982c8a7254c1f4`;
`results.json` is
`e90b1e432b6cd9338ecfe3a7a530e27372efcb8317803969c8351ed98d7fb52b`.
The report identifies actual execution main `50024fa`, candidate ancestor
`599baac`, input hashes, Python 3.15.2 and the 2.546-second local oracle
runtime. The governing design gained this evidence link afterward, so its
current file hash differs from the archived input hash. Runtime is diagnostic
workflow time, not shader cost. The independent
review checked formula separation, selected case construction, counts and
hashes; it found no remaining math or source-integrity blocker after two
provenance/statistic-label corrections.

| Discriminator | Observed result | Consequence |
| --- | --- | --- |
| Sharp opposing grazing family | At `t=2^-12`, `r=.32`, `F0=.04`, original real radiance per unit illuminance is `148.1011669969`; scaled real result agrees. | Tiny cosines/half-vector length cannot be assumed dim. |
| Unscaled combined Smith ratio with FTZ | At `ci=co=2^-126`, sampled denominator becomes zero and ratio infinite. | The unscaled arithmetic path is unsafe outside a certified domain. |
| Candidate f32 direction narrowing | Same `t=2^-12` case gives `148.0148468018` in eight selected modes, about 0.0583% below real reference. | Scale floors alone do not bound visual/numerical error. |
| Exact-positive view cosine `2^-140` | Selected FTZ mode returns zero instead of real `147.8708750744`. | A raw f32 hemisphere test can erase a bright contribution. |
| Opposite true signs with same f32 directions | Selected fused dot gives positive `ci≈7.15e-9` for both real frontface and backface inputs. | Sign classification needs a source-derived margin or robust predicate. |

The mathematical fixture admitted 3,507 cases under test-supplied
`r≥.32`, `s≥2^-16`, `hm≥2^-12` and a **flag** for sign certification.
That flag is not a certificate implementation. The selected f32 simulator
samples FTZ on/off, fusion on/off and two product associations; its
`evaluated_max_abs` includes diagnostic bypass cases and is not an admitted
error bound. It rounds explicit operations to binary32, uses host square
roots, can double-round Decimal input conversion and does not exercise every
WGSL reassociation or approximation. It reports final outputs, not the
finiteness of every intermediate. Its 320-digit cross-check retained the
same saved 240-digit directions and Pi constant, so it tests evaluation
stability rather than independent geometry reconstruction.

The loose illustrative stored-value bound is `24,339.89269846`, below the
RGBA16Float maximum `65,504`; the same oracle reproduces the low-roughness
normal-incidence overflow witness at `75,805.3951` after fixed pre-exposure.
Neither bound includes f32/WGSL operation error, half packing, tone, encoded
output or actual authored light/geometry extrema. The priority is to derive
the hemisphere/angular/normal-map and local-distance certificates for an
admitted source, then validate actual WGSL and moving-image quality. An
unqualified case must not be silently clamped or dropped.

Reproduce from the repository root with Python 3.11 or newer:

```sh
mkdir -p tmp/r0-ggx-oracle-repro
tar -xzf docs/evidence/render/R0-GGX-NUMERIC-ORACLE-2026-11-03.tar.gz -C tmp/r0-ggx-oracle-repro
python3 tmp/r0-ggx-oracle-repro/oracle.py > tmp/r0-ggx-oracle-repro/run-summary.json
```

The generated files record their current revision and wall time, so their
hashes are not expected to match this selected run after a later checkout or
Python version; inspect the semantic assertions and counts. No third-party
package or network access is required.
