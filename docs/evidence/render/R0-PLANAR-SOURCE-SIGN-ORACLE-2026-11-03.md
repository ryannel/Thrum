# R0 planar source-sign oracle

3 November 2026. A bounded offline oracle certifies the source-real Sun and
view hemisphere signs for one static, one-sided planar triangle with a flat
geometric normal, identity transform, no normal map, and pinned camera and Sun.
Each source number means the exact value of its decoded binary64 bits. This is
a first-fixture specialization, not general C08 material admission or a shader
certificate. The [editable proof archive](R0-PLANAR-SOURCE-SIGN-ORACLE-2026-11-03.tar.gz)
has SHA-256
`cf6770db384aaa37c9b30bc56969e42a1e514fae46807740508f1a59dd50642f`.
Its oracle and result hashes are
`bbcf517fea1f85d511043d76978f328fc81836637c011e6fa906842459aaabde`
and
`202e04dd3b5917c77f30a6a55ed7b7115b64d941263ad59c84f3408d13cb08ae`.
The retained manifest verifies all ten archived source/result files.

For exact source vertices `P0,P1,P2`, the oracle forms
`N=(P1−P0)×(P2−P0)` and `d=N·P0`. It rejects `N=0`, then tests the signs of
`N·Sun` and `N·Camera−d`, where Sun is C08's world-space
receiver-to-source direction. The view expression has the same sign at every
point on the source plane. An integer-dyadic route and an independent rational
determinant route agree exactly on the plane and sign results.

| Offline check | Result |
| --- | ---: |
| Named exact certificates | 18 |
| Invalid or out-of-specialization refusals | 15 |
| Stale binding changes refused | 21 |
| Wrong algorithms killed by concrete witnesses | 9 |
| Supplementary exact-route agreements | 192 |
| Exact barycentric plane/view identities | 3,186 |

An adversarial review found that the initial proof-local unit-length check
used a different operation from F3. The corrected host-specific gate now
matches F3's ordered native chained `hypot` bits for all 33 fixture directions,
including a boundary direction that the earlier check falsely admitted.
Python 3.11 and 3.15 reproduced the final result byte-for-byte; the parent
re-ran the Python 3.11 oracle, Rust `hypot` reference and archive hashes. A
second independent review found no remaining actionable issue. The exact sign
arithmetic does not depend on the host `hypot` implementation.

Two schema-tolerated binary64 Sun directions have opposite source signs
`±2^-53` but narrow to identical f32 direction words. No shader seeing only
those words can recover both signs. The proof separately records a
direction-only error bound for proof-local F4 normalization and f32 packing;
the margin becomes inconclusive for this pair. It does not authenticate an
actual F4 product or bound shader normal, interpolation, WGSL arithmetic,
half-vector magnitude, BRDF output, storage, coverage or moving-image quality.
Smooth/normal-mapped or transformed faces need new certificates. See the
[R0 robust evaluation research](../../design/research/R0-ROBUST-GGX-EVALUATION-OPTIONS-2026-11-03.md)
for the general represented-input and fallback gates.
