# Ambient wind waves

`Ambient` is the GPU half of `thrum-sea-model`. A water contribution prepares it with the
generation's `DeviceSite`, returns its `graph()` (or offsets and appends the resources/accesses
to its own declaration), calls `upload` in its upload callback, and forwards the named simulate
callbacks to `encode_pass`. The host owns the queue, submission and timestamps. Nothing is drawn.
All thirteen passes run every frame, including captures. No ambient history or tick bank exists.

Use `display_seconds(frame.time)` for C02's `(committed tick - 1 + alpha) * tick duration`.
The caller supplies the CPU model's spectrum at that same time, including any wind transition.
Upload phases are reduced in f64 relative to the supplied world anchor. Queries must be within
8,192 m of it. Cache levels are world-snapped and their addresses are available in `levels`.
The long-wave inversion is prepared for contraction below 0.5; the default choppiness is 0.1.
The query kernel and cache kernel call the same evaluator. Unresolved components are excluded
per level, with cutoff pi / texel width. The consumer can obtain their slope covariance by
summing `0.5 * amplitude² * outer(k,k)` over the excluded CPU-model components.
`LONG_WAVE_EVALUATOR` lends that same WGSL function to WATERS' body-query pipeline. Its
`Long` layout is `(kx, kz, amplitude, anchored phase)` followed by `(omega, 0, 0, 0)`;
`Field` is the three vec4 layout below. Query pipelines supply their own f64-reduced
phases at the force query's time and anchor, independently of the display cache's time.

GPU outputs use storage buffers, requiring no storage texture format or texture binding.
There are eight storage bindings, at most 256 invocations per workgroup and 2 KiB of
workgroup storage. Preparation names unsupported profile values and resource limits.
The same portable kernels run on both recipes; no accelerated variant is promoted here.

Each cascade has 256 × 256 texels, x varying first, at `(index + 0.5) * patch length / 256`.
Three bands are consecutive. Resolve a world sample modulo each band's length in f64 before
converting to f32. Cascade bins have integer loop cycles; unsigned multiplication wraps the
frame's 32-bit loop fraction before conversion to phase. Two shared 256-point transforms
apply an unnormalized positive-sign inverse to eleven complex channels.
The uploaded spectrum allocation becomes the final transform output after evolution;
the next frame uploads fresh CPU-owned modes before reading it. Each transform stage
therefore has an unambiguous producer on the effect graph.

Each field texel is three consecutive `vec4<f32>` values:

| Product | First vec4 | Second vec4 | Third vec4 |
| --- | --- | --- | --- |
| Cascade | displacement xyz, Jacobian | slope xz, dX/dx, dX/dz | velocity xyz, dZ/dz |
| Long waves | horizontal displacement x, world height, horizontal displacement z, Jacobian | world slope xz, zero, zero | material velocity xyz, Eulerian height derivative |

The long buffer starts with 4,096 query slots (unused slots are cleared without evaluation), followed
by eight row-major 128 × 128 cache levels. The cache stores the displaced-world-point result
of its restricted long-wave model. No forces read this display cache.

Moments store two `vec4<f32>` values: `(mean sx, mean sz, mean sx², mean sx*sz)` and
`(mean sz², mean J, mean J², 0)`. Recover covariance as second moment minus outer mean,
and Jacobian variance as `mean J² - mean J * mean J`; clamp negative roundoff to zero.
Each band has 87,381 moment texels across nine separately allocated mip buffers.
`moments_buffer(mip)` lends one level with three consecutive bands. The eight reductions
average raw moments, preserving variance instead of averaging covariance away.

Uploads reuse one CPU byte arena and GPU staging allocation. Record and submit one frame before
uploading another; the scene owner serializes recording. An encoded and discarded upload never
publishes its copies. Consumers rebuild `Ambient` when the device generation changes. Each next
frame overwrites all outputs from the CPU model; capture and discarded frames retain no state.

The Metal tests require the repository's `gpu` lock and never skip an unavailable device. They
record timestamps and resource bytes in `tmp/sea-m2`; the selected measurement and bounds are
retained in [M2 evidence](../../docs/evidence/SEA/M2.md).
