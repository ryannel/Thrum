# Ambient wind waves

`Ambient` is the GPU half of `thrum-sea-model`. A water contribution prepares it with the
generation's `DeviceSite`, returns its `graph()` (or offsets and appends the resources/accesses
to its own declaration), calls `upload` in its upload callback, and forwards the named simulate
callbacks to `encode_pass`. The host owns the queue, submission and timestamps. Nothing is drawn.
All field and filtering passes run every frame, including captures. No ambient history or tick bank exists.

Use `display_seconds(frame.time)` for C02's `(committed tick - 1 + alpha) * tick duration`.
The caller supplies the CPU model's spectrum at that same time, including any wind transition.
Use `Transition::at_into` to blend into a retained spectrum without allocating a snapshot.
Upload phases are reduced in f64 relative to the supplied world anchor. Queries must be within
8,192 m of it. Cache levels are world-snapped and their addresses are available in `levels`.
The query inversion is prepared for contraction below 0.9; the default choppiness is 0.1.
Queries invert the horizontal map; the drawn cache evaluates the rest sheet forward once. Unresolved components are excluded
per level, with cutoff pi / texel width. The consumer can obtain their slope covariance by
summing `0.5 * amplitude² * outer(k,k)` over the excluded CPU-model components.
`LONG_WAVE_EVALUATOR` lends that same WGSL function to WATERS' body-query pipeline. Its
`Long` layout is `(kx, kz, amplitude, anchored phase)` followed by `(omega, 0, 0, 0)`;
`Field` is the three vec4 layout below. Query pipelines supply their own f64-reduced
phases at the force query's time and anchor, independently of the display cache's time.

Transform intermediates and the independent query/readback products use storage buffers.
The drawn fields use filterable `rgba16float` array textures with complete mip chains.
There are at most eight storage-buffer bindings, 256 invocations per workgroup and 2 KiB of
workgroup storage. Preparation names unsupported profile values and resource limits.
The same portable kernels run on both recipes; no accelerated variant is promoted here.

Each cascade has 256 × 256 texels, x varying first, at `(index + 0.5) * patch length / 256`.
Three bands are consecutive. Resolve a world sample modulo each band's length in f64 before
converting to f32. Cascade bins have integer loop cycles; unsigned multiplication wraps the
frame's 32-bit loop fraction before conversion to phase. Two shared 256-point transforms
apply an unnormalized positive-sign inverse to eleven complex channels.
Modes live in a separate read-only allocation and upload only when cascade components or
lengths change, including in-place replacement. Evolution, rows and columns each write
their own work buffer, so each graph resource has one complete producer.
Typed resource and pass keys in `src/spec.rs` drive sizes, usages, bindings, dispatches and
graph access. Each pass binds exactly the buffers it uses, including its mip uniform.
The shader's bound structs are reflected against whole Rust `Pod` records and member offsets.

Each field texel is three consecutive `vec4<f32>` values:

| Product | First vec4 | Second vec4 | Third vec4 |
| --- | --- | --- | --- |
| Cascade | displacement xyz, Jacobian | parametric slope xz, dX/dx, dX/dz | velocity xyz, dZ/dz |
| Long query | horizontal displacement x, world height, horizontal displacement z, Jacobian | world slope xz, zero, zero | material velocity xyz, Eulerian height derivative |
| Long cache | displacement xyz, Jacobian | parametric slope xz, dX/dx, dX/dz | velocity xyz, dZ/dz |

The long buffer starts with 4,096 query slots (unused slots are cleared without evaluation), followed
by eight row-major 128 × 128 cache levels. The cache stores the forward result at each
texel's rest place, from its restricted long-wave model. No forces read this display cache.

`filtered_views()` lends the cascade and long-cache textures, respectively. Each wave layer
has five consecutive array planes: the three field vec4s above, `(sx², sx*sz, sz², dZ/dz)`,
and `(J², displacement residual xyz)`. Mean J is in the displacement plane's fourth channel.
The pixel reads only the derivative and second-moment planes; vertices read the first three
and add the last plane's residual to displacement. Splitting the explicitly quantized high
part and its residual keeps large waves inside the millimetre height bound without another
texture. Reductions carry their new half-rounding error into the residual too. Every moment
and decoded field is averaged linearly through every mip. Cascades
repeat; cache levels clamp at their edges. Surface consumers use trilinear anisotropic
sampling over their own rest-space pixel derivatives, and recover covariance as the
second moments minus the outer product of the filtered mean. Levels omitted from a
cache contribute their model covariance. `filtered_accesses()` declares these reads.
The complete pyramids take 17,476,120 bytes; this is additional to the retained f32
query/transform products. `forward_cache_gpu` reads every texture texel of every mip,
all five planes, and checks half precision against the f32 product and direct base means.
Recover Jacobian variance as `mean J² - mean J * mean J`; clamp negative roundoff to zero.

Uploads reuse one CPU byte arena and GPU staging allocation. Record and submit one frame before
uploading another; the scene owner serializes recording. An encoded and discarded upload never
publishes its dynamic copies. A changed mode queue write survives discarding the encoder;
the following frame can safely reuse those modes. No compute pass writes the mode buffer.
`upload_bytes()` reports parameter, mode, long-wave and query payload bytes from that path.
Consumers rebuild `Ambient` when the device generation changes. Each next frame overwrites
all outputs from the CPU model; capture and discarded frames retain no output history.

The Metal tests require the repository's `gpu` lock and never skip an unavailable device. They
record timestamps and resource bytes in `tmp/sea-m2`; the selected measurement and bounds are
retained in [M2 evidence](../../.orchestrate/evidence/SEA/M2.md).
