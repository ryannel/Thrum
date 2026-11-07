# R0 exact represented-dot Metal diagnostic

3 November 2026. A standalone public-wgpu 30.0.1 / Naga 30.0.1 WGSL compute
kernel exactly accumulates a three-product dot from six **raw `u32` binary32
encodings**. Accepted components are normal or signed zero in `[-1,1]`;
subnormal, nonfinite and larger inputs refuse. The kernel uses a `2^-298`
integer lattice and returns the complete ten-limb magnitude, sign, exponent,
truncated and nearest-even normalized significands. It never materializes a
tiny exact dot as a GPU float. This is an isolated represented-input
diagnostic, not an admitted R0 shader fallback.

The [editable source, inputs, selected results and provenance archive](../ARCHIVE-ACCESS.md#archive-inventory)
has SHA-256
`4ef0722af94b438e179ca06c17f5b096f79decb51f4240ea332cd846bbc65c84`.
Its 26-file manifest was independently verified. The archive records the
execution revision `8bf10ef` separately from the later packaging revision
`9e25432` and pins the executed shader, host, input and readback hashes.
Large raw GPU readbacks are excluded but their SHA-256 values and comparison
records are retained; `expected.bin` regenerates from the included oracle.

| Bounded check on Apple M3 Max Metal | Result |
| --- | ---: |
| Input records / distinct tuples | 189,803 / 189,792 |
| Invalid records refused | 66 |
| Selected output words matched to independent Fraction oracle | 9,679,953 across three processes; zero mismatches |
| Repeated dispatch outputs | 10 byte-identical per process |
| Valid delayed-resolve GPU timestamp pairs | 30 |
| Whole-corpus isolated GPU pass median, by process | 1.045, 0.885, 1.024 ms |

The corpus spans every admitted exponent pair, carry and long-borrow
boundaries, exact zeros, signed `2^-298` cancellation, a positive `2^-127`
dot, threshold neighbours and wrong-sign floating-evaluation witnesses.
Python 3.11 and 3.15 generated identical input/reference bytes. The parent
rechecked the archive's 26 member hashes and one full 189,803-record saved
readback against the independent oracle. Pinned release build, Naga
validation, rustfmt and Clippy passed in the scratch crate.

The timings include the complete compute pass and its 12.9 MB result writes,
but exclude uploads, query resolution, readback and CPU work. They are not
ordinary fragment cost, fallback frequency or sustained frame cost. The
scratch shader declares four ten-word arrays; actual registers, spills,
occupancy and divergence are unmeasured. A common fallback over millions of
fragment/light pairs could be expensive and needs a smaller output ABI and
paired workload measurements before selection.

Raw integer input bypasses the unresolved float-to-bit recovery path. This
does not certify source-real signs, upstream packing, WGSL floating dot
accuracy, ratio magnitude, near-opposed half-vector construction, full
GGX/Smith/Fresnel, half storage, moving-image quality or another backend.
See the [robust evaluation research](../../design/research/R0-ROBUST-GGX-EVALUATION-OPTIONS-2026-11-03.md)
and [planar source-sign proof](R0-PLANAR-SOURCE-SIGN-ORACLE-2026-11-03.md)
for those separate boundaries.
