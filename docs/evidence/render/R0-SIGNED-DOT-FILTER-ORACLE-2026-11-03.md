# R0 represented-dot filter finite-operation oracle

3 November 2026. This selects an independent Python 3.11 rejection oracle for
the [conditional signed-dot bound](../../design/research/R0-ROBUST-GGX-EVALUATION-OPTIONS-2026-11-03.md#a-conservative-signed-dot-filter-to-challenge).
The execution revision was `b6b23d7`; `results.json` records its exact input
hashes and operand bits. The [archive](../ARCHIVE-ACCESS.md#archive-inventory)
contains the editable source, deterministic results, report, owner record and
hash manifest. Archive SHA-256:
`df29897ad357de22ae1ca3b48105be50dc6d058044523546f16d184255681453`.
Source SHA-256:
`4297ab9d9b04de583cab2e894ab4bb9e343dcc0691a7244f4901c74c58b3cd5c`.
Results SHA-256:
`94ba075b355ee45dcd9b8cf02ffbbe9a69c7664b128f5771cc75311645f955bc`.
The parent and an independent adversarial reviewer reproduced the result
byte-for-byte under Python 3.11. The reviewer also expanded every labeled
schedule for the sign-reversal and worst-error fixtures without state
compression and matched their saved distributions.

| Finite scalar model check | Result |
| --- | ---: |
| Distinct operand fixtures | 276 (20 focused, 256 seeded) |
| Labeled five-node schedules | 3,391,488 (`12` product-order/tree cases × `4^5` node choices per fixture) |
| Schedules admitted by strict `|d|>2^-20` filter | 1,272,832; every exact represented sign matched |
| Uncertain schedules | 2,118,656, including 227,328 exactly on the threshold |
| Focused fused schedules | 43,064; 12,304 outside the band, all sign-correct |
| Wrong threshold `2^-48` | Rejected by exact `+2^-47` / computed `−2^-47` witness |

The oracle decodes normal-or-zero f32 components in `[-1,1]` into exact
fractions, computes the exact dot independently through a second integer
lattice route, then explores both adjacent directed roundings and optional
effective FTZ at each product/add node. It preserves schedule multiplicity
when equal intermediate numerical states are merged. Each final state stays
within the analytically derived
`E=9·2^-24+5·2^-126<2^-20` envelope. The largest observed absolute error
was about `2.495e-7`, below E. Focused cases include the normal-input
`2^-298` cancellation, exact zero, product underflow, threshold neighbors,
and the nonzero wrong-sign computation. This finite exploration challenges
the derivation; it is not its proof over all binary32 operands.

The supplement models selected fused contractions, not every compiler
transformation. No actual WGSL expression, builtin, compiler, backend, device,
source-to-representation error certificate, magnitude-preserving fallback,
moving-image quality or cost was tested. Computed values inside the band
remain uncertain and may not be replaced with zero. The full report in the
archive records these limits and reproduction steps.
