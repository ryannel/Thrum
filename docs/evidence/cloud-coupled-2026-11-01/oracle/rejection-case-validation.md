# Retained GPU rejection-case validation

All four retained gzip probe sets have eight valid transport traces. Each detailed report compares actual GPU queries with independent1m joint transport.

| Case | Verified result |
| --- | --- |
| Upper occluder | Both analytic geometry and GPU visibility are0 at the shadow probe; direct incident RGB is exactly0. |
| Main cloud off; reflection on | Every main-ray cloud flag is0; every secondary flag is1. Actual T agrees with the corresponding independent masked solve. |
| Foreground fog | Near-to-mirror foreground composes once with the separate reflected ray. Reapplying the foreground produces a large rejected counterexample. |
| All illumination zero | S, terminal radiance and composed radiance are exactly0 on every trace while cloud/air/fog extinction remains active. |

Per-case numerical limits, identities and composition counterexamples are retained in [the JSON](rejection-case-validation.json) and the linked per-probe query reports.
